//! Entry stores that stand for loads, and dead private stores. See NOTICE.
use super::value_selection::alias_compatible;
use crate::lowir::{
    Error,
    model::{Body, InstructionKind as I, Value},
    vocabulary::Type,
};
#[derive(Clone, Copy, Default)]
pub(super) enum Slot {
    #[default]
    Unused,
    Dead,
    Promoted(Value),
    Memory,
}
#[derive(Default)]
struct Uses {
    loads: u32,
    stores: u32,
    escape: u8,
    stored: Option<Value>,
}
pub(super) fn analyze(body: &Body, flow: &super::control_flow::Flow) -> Result<Vec<Slot>, Error> {
    let mut uses: Vec<Uses> = (0..body.slots().len()).map(|_| Uses::default()).collect();
    let mut stored_in_entry = vec![false; body.slots().len()];
    for (pos, &id) in flow.instructions.iter().enumerate() {
        let block = flow.blocks[pos];
        if !flow.reachable[block.index()] {
            continue;
        }
        match body.instruction(id).kind {
            I::Load(facts, ty, Value::Slot(slot)) => {
                let u = &mut uses[slot.index()];
                u.loads += 1;
                if facts.volatile()
                    || ty != body.slot(slot).ty
                    || block == body.layout[0] && !stored_in_entry[slot.index()]
                {
                    u.escape = 1;
                }
            }
            I::Store(facts, ty, stored, Value::Slot(slot)) => {
                let u = &mut uses[slot.index()];
                if block == body.layout[0] {
                    stored_in_entry[slot.index()] = true;
                }
                u.stores += 1;
                u.stored = if u.stores == 1 { Some(stored) } else { None };
                if facts.volatile() || ty != body.slot(slot).ty {
                    u.escape = 2;
                }
                if let Value::Slot(source) = stored {
                    uses[source.index()].escape = 1;
                }
            }
            ref kind => body.visit_values(kind, |value| {
                if let Value::Slot(slot) = value {
                    uses[slot.index()].escape = 1;
                }
            }),
        }
    }
    let mut out = Vec::with_capacity(uses.len());
    for (slot, u) in body.slots().iter().zip(uses) {
        let scalar = slot.ty.bits() != 0 && slot.ty != Type::I128
            || matches!(slot.ty, Type::Ptr | Type::F32 | Type::F64);
        let stands_for = u.stored.is_some_and(|value| match value {
            Value::Slot(_) => false,
            Value::Temp(temp) => body
                .temp(temp)
                .ty
                .is_some_and(|from| alias_compatible(slot.ty, from)),
            _ => true,
        });
        out.push(if u.loads == 0 && u.stores == 0 && u.escape == 0 {
            Slot::Unused
        } else if u.loads == 0 && u.escape == 0 {
            Slot::Dead
        } else if u.escape == 0
            && u.stores == 1
            && stored_in_entry[out.len()]
            && flow.predecessors(body.layout[0]).is_empty()
            && scalar
            && stands_for
        {
            Slot::Promoted(u.stored.ok_or(Error::Limit)?)
        } else {
            Slot::Memory
        });
    }
    Ok(out)
}
