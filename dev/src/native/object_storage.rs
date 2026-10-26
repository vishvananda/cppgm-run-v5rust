//! Object-slot interference and aggregate home coalescing from frozen allocation.cpp. See NOTICE.
use super::{
    abi,
    address_facts::{AddressBase, Addresses},
    slot_analysis::Slot,
    value_facts::{Alias, Facts},
};
use crate::lowir::{
    Error,
    model::{Body, InstructionKind as I, SlotId, TempId, Value},
    vocabulary::Type,
};

// Slots live for the whole call. They share only when no path names both,
// with the frozen first-earlier-group choice and 64-object admission bound.
pub(super) fn shared_slots(body: &Body, facts: &Facts, states: &[Slot]) -> Vec<Option<SlotId>> {
    if facts.level == 0
        || facts
            .flow
            .instructions
            .iter()
            .enumerate()
            .any(|(pos, &id)| {
                facts.active[pos]
                    && matches!(
                        body.instruction(id).kind,
                        I::Runtime(
                            crate::lowir::model::RuntimeInstruction::EhTry(_)
                                | crate::lowir::model::RuntimeInstruction::EhCleanup(_)
                        )
                    )
            })
    {
        return Vec::new();
    }
    let mut candidates = Vec::new();
    let mut pair = false;
    for (index, (slot, state)) in body.slots().iter().zip(states).enumerate() {
        if !matches!(state, Slot::Memory) || !matches!(slot.ty, Type::Object { .. }) {
            continue;
        }
        pair |= candidates
            .iter()
            .any(|&c: &usize| body.slots()[c].ty == slot.ty);
        candidates.push(index);
        if candidates.len() > 64 {
            return Vec::new();
        }
    }
    if !pair {
        return Vec::new();
    }
    let mut bit = vec![0_u64; body.slots().len()];
    for (c, &slot) in candidates.iter().enumerate() {
        bit[slot] = 1 << c;
    }
    let mut named = vec![0_u64; body.blocks().len()];
    for &block in &body.layout {
        if facts.flow.reachable[block.index()] {
            for &id in &body.block(block).instructions {
                body.visit_values(&body.instruction(id).kind, |value| {
                    if let Value::Slot(slot) = value {
                        named[block.index()] |= bit[slot.index()];
                    }
                });
            }
        }
    }
    let mut reach = vec![0_u64; candidates.len()];
    let mut seen = vec![None; body.blocks().len()];
    let mut work = Vec::new();
    for (c, reached) in reach.iter_mut().enumerate() {
        for &block in &body.layout {
            if named[block.index()] & (1 << c) != 0 {
                seen[block.index()] = Some(c);
                work.push(block);
            }
        }
        while let Some(block) = work.pop() {
            *reached |= named[block.index()];
            for &next in facts.flow.successors(block) {
                if seen[next.index()] != Some(c) {
                    seen[next.index()] = Some(c);
                    work.push(next);
                }
            }
        }
    }
    let mut owners = vec![None; body.slots().len()];
    let mut groups = [0_u64; 64];
    for (c, &slot) in candidates.iter().enumerate() {
        let mut interferes = reach[c];
        for (d, &mask) in reach.iter().enumerate() {
            interferes |= ((mask >> c) & 1) << d;
        }
        let group = (0..c)
            .find(|&d| {
                groups[d] != 0
                    && groups[d] & interferes == 0
                    && body.slots()[candidates[d]].ty == body.slots()[slot].ty
            })
            .unwrap_or(c);
        groups[group] |= 1 << c;
        owners[slot] = body.slot_id(candidates[group]);
    }
    owners
}

fn frame_slot(addresses: &Addresses, temp: TempId) -> Option<SlotId> {
    let expr = addresses.expressions[temp.index()]?;
    if let AddressBase::Slot(slot) = expr.base {
        Some(slot)
    } else {
        None
    }
}

// A memory-class parameter has a slot home only when its sole reader copies
// all its bytes there before any non-address instruction touches that slot.
pub(super) fn parameter_home(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    states: &[Slot],
    parameter: TempId,
) -> Result<Option<SlotId>, Error> {
    let entry = body.layout[0];
    let interval = facts.intervals[parameter.index()];
    if facts.level == 0
        || !facts.flow.predecessors(entry).is_empty()
        || !matches!(
            body.temp(parameter).ty,
            Some(Type::Object { .. } | Type::I128 | Type::F80)
        )
        || interval.uses != 1
    {
        return Ok(None);
    }
    let Some(user) = interval.consumer else {
        return Ok(None);
    };
    let I::CopyObject(bytes, _, source, destination) =
        body.instruction(facts.flow.instructions[user]).kind
    else {
        return Ok(None);
    };
    if facts.flow.blocks[user] != entry
        || !matches!(facts.resolve(body, source), Alias::Temporary(t) if t == parameter)
        || bytes
            != u64::from(abi::value_bytes(
                body.temp(parameter).ty.ok_or(Error::Limit)?,
            )?)
    {
        return Ok(None);
    }
    let Alias::Temporary(destination) = facts.resolve(body, destination) else {
        return Ok(None);
    };
    let Some(expr) = addresses.expressions[destination.index()] else {
        return Ok(None);
    };
    let Some(slot) = frame_slot(addresses, destination) else {
        return Ok(None);
    };
    if expr.displacement != 0
        || expr.index.is_some()
        || !matches!(states[slot.index()], Slot::Memory)
        || u64::from(abi::value_bytes(body.slot(slot).ty)?) != bytes
    {
        return Ok(None);
    }
    for pos in facts.flow.ranges[entry.index()].start..user {
        let inst = body.instruction(facts.flow.instructions[pos]);
        if matches!(inst.kind, I::Addr(_) | I::Index(..)) {
            continue;
        }
        let mut touched = false;
        body.visit_values(&inst.kind, |value| {
            touched |= match facts.resolve(body, value) {
                Alias::Slot(s) => s == slot,
                Alias::Temporary(t) => frame_slot(addresses, t) == Some(slot),
                _ => false,
            };
        });
        if touched {
            return Ok(None);
        }
    }
    Ok(Some(slot))
}

// A register aggregate may be stored directly into its sole following copy's
// destination. Folded address arithmetic may intervene, emitted work may not.
pub(super) fn result_destination(
    body: &Body,
    facts: &Facts,
    addresses: &Addresses,
    result: TempId,
    definition: usize,
) -> Result<Option<Value>, Error> {
    let interval = facts.intervals[result.index()];
    let Some(user) = interval.consumer.filter(|&u| u > definition) else {
        return Ok(None);
    };
    if facts.level == 0
        || interval.uses != 1
        || facts.flow.blocks[user] != facts.flow.blocks[definition]
    {
        return Ok(None);
    }
    let I::CopyObject(bytes, _, source, destination) =
        body.instruction(facts.flow.instructions[user]).kind
    else {
        return Ok(None);
    };
    if !matches!(facts.resolve(body, source), Alias::Temporary(t) if t == result)
        || matches!(facts.resolve(body, destination), Alias::Temporary(t) if t == result)
        || bytes != u64::from(abi::value_bytes(body.temp(result).ty.ok_or(Error::Limit)?)?)
    {
        return Ok(None);
    }
    for pos in definition + 1..user {
        let inst = body.instruction(facts.flow.instructions[pos]);
        if !matches!(inst.kind, I::Addr(_) | I::Index(..))
            || !inst
                .result
                .is_some_and(|t| addresses.lazy[t.index()] || facts.intervals[t.index()].uses == 0)
        {
            return Ok(None);
        }
    }
    Ok(Some(destination))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::model::Program;

    fn select(program: &Program, level: u8) -> Result<super::super::mir::Function, Error> {
        let function = &program.functions()[0];
        super::super::value_selection::select(
            program,
            program.find_symbol(function.name).ok_or(Error::Limit)?,
            function,
            level,
        )
    }

    #[test]
    fn parameter_copy_uses_its_entry_slot_without_an_extra_home() -> Result<(), Error> {
        let program = crate::lowir::test_program(
            b"function @f(%p : obj<16x8>) -> i64 {
                slot $local : obj<16x8>
                block ^entry:
                  %to = addr $local
                  copyobj 16x8 %p, %to
                  %value = load i64 %to
                  return i64 %value
              }",
        )?;
        let plain = select(&program, 0)?;
        let optimized = select(&program, 1)?;
        assert_eq!(plain.homes, 1);
        assert_eq!(optimized.homes, 0);
        assert!(
            !optimized
                .instructions
                .iter()
                .any(|inst| { matches!(inst.op, super::super::mir::Op::Bulk(_)) })
        );
        let slot = optimized
            .bindings
            .iter()
            .find(|b| b.kind == super::super::mir::BindingKind::Slot)
            .ok_or(Error::Limit)?;
        let parameter = optimized
            .bindings
            .iter()
            .find(|b| b.kind == super::super::mir::BindingKind::Parameter)
            .ok_or(Error::Limit)?;
        assert_eq!(slot.offset, parameter.offset);
        Ok(())
    }

    #[test]
    fn prior_access_and_partial_parameter_copies_keep_separate_homes() -> Result<(), Error> {
        for (before, bytes) in [
            ("store i64 7, $local", 16),
            ("%field = index i8 %to, 8 store i64 7, %field", 16),
            ("", 8),
        ] {
            let input = format!(
                "function @f(%p : obj<16x8>) -> i64 {{
                slot $local : obj<16x8>
                block ^entry:
                  %to = addr $local
                  {before}
                  copyobj {bytes}x8 %p, %to
                  %value = load i64 %to
                  return i64 %value
              }}"
            );
            let program = crate::lowir::test_program(input.as_bytes())?;
            let mir = select(&program, 1)?;
            assert_eq!(mir.homes, 1, "{input}");
            let slot = mir
                .bindings
                .iter()
                .find(|b| b.kind == super::super::mir::BindingKind::Slot)
                .ok_or(Error::Limit)?;
            let parameter = mir
                .bindings
                .iter()
                .find(|b| b.kind == super::super::mir::BindingKind::Parameter)
                .ok_or(Error::Limit)?;
            assert_ne!(slot.offset, parameter.offset, "{input}");
        }
        Ok(())
    }

    #[test]
    fn register_results_skip_the_home_only_for_a_whole_adjacent_copy() -> Result<(), Error> {
        for (ty, bytes, between, expected_homes) in [
            ("obj<16x8>", 16, "%to = addr $local", 0),
            ("i128", 16, "%to = addr $local", 0),
            ("obj<9x1>", 9, "%to = addr $local", 1),
            ("obj<16x8>", 8, "%to = addr $local", 1),
            ("obj<16x8>", 16, "%to = addr $local store i64 7, %to", 1),
        ] {
            let input = format!(
                "function @f() -> i64 {{
                slot $local : {ty}
                block ^entry:
                  %result = call {ty} @producer()
                  {between}
                  copyobj {bytes}x1 %result, %to
                  %value = load i64 %to
                  return i64 %value
              }}
              declare function @producer() -> {ty}"
            );
            let program = crate::lowir::test_program(input.as_bytes())?;
            let mir = select(&program, 1)?;
            assert_eq!(mir.homes, expected_homes, "{input}");
        }
        Ok(())
    }

    #[test]
    fn slot_sharing_uses_paths_types_and_region_edges() -> Result<(), Error> {
        for (left_tail, right_type, region, shares) in [
            ("return void", "obj<16x8>", "", true),
            ("jump ^right", "obj<16x8>", "", false),
            ("return void", "obj<8x8>", "", false),
            ("return void", "obj<16x8>", "eh_cleanup", false),
        ] {
            let input = format!(
                "function @f(%condition : i64) -> void {{
                slot $left : obj<16x8>
                slot $right : {right_type}
                block ^entry:
                  {region}
                  branch %condition, ^left, ^right
                block ^left:
                  %l = addr $left
                  call void @observe(%l)
                  {left_tail}
                block ^right:
                  %r = addr $right
                  call void @observe(%r)
                  return void
              }}
              declare function @observe(%p : ptr) -> void"
            );
            let program = crate::lowir::test_program(input.as_bytes())?;
            let body = program.functions()[0].body.as_ref().ok_or(Error::Limit)?;
            let flow = super::super::control_flow::Flow::analyze(body)?;
            let slots = super::super::slot_analysis::analyze(body, &flow)?;
            let facts = Facts::analyze(body, &slots, flow, 1)?;
            let owners = shared_slots(body, &facts, &slots);
            let shared = owners
                .get(1)
                .copied()
                .flatten()
                .is_some_and(|s| s.index() == 0);
            assert_eq!(shared, shares, "{input}");
        }
        Ok(())
    }
}
