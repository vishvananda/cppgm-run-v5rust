//! Unit-owned unwritten-global facts from `opt/function_ir.cpp`. See NOTICE.
use crate::lowir::{
    model::{
        Body, DataItem, GlobalInit, Instruction, InstructionKind as I, Program, SymbolId, TempId,
        Value,
    },
    vocabulary::Binding,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Origin {
    #[default]
    Missing,
    Symbol(SymbolId),
    Conflict,
}
struct Addresses {
    carried: Vec<Origin>,
    held: Vec<Origin>,
    taken: Vec<bool>,
}
impl Addresses {
    fn candidate(&self, value: Value, unwritten: &[bool]) -> Option<SymbolId> {
        match value {
            Value::Symbol(symbol) => unwritten[symbol.index()].then_some(symbol),
            Value::Temp(temp) => match self.carried[temp.index()] {
                Origin::Symbol(symbol) => Some(symbol),
                _ => None,
            },
            _ => None,
        }
    }
    fn carry(
        &mut self,
        result: Option<TempId>,
        symbol: Option<SymbolId>,
        unwritten: &mut [bool],
    ) -> bool {
        result.is_some_and(|result| merge(&mut self.carried[result.index()], symbol, unwritten))
    }
    fn propagate(&mut self, instruction: &Instruction, unwritten: &mut [bool]) -> bool {
        let mut changed = false;
        match &instruction.kind {
            I::Addr(value) | I::Copy(_, value) | I::Convert(_, _, _, value) => {
                changed |= self.carry(
                    instruction.result,
                    self.candidate(*value, unwritten),
                    unwritten,
                );
            }
            I::Index(_, _, first, second) | I::Binary(_, _, _, first, second) => {
                changed |= self.carry(
                    instruction.result,
                    self.candidate(*first, unwritten),
                    unwritten,
                );
                changed |= self.carry(
                    instruction.result,
                    self.candidate(*second, unwritten),
                    unwritten,
                );
            }
            I::Phi(_, incoming) => {
                for &(_, value) in incoming {
                    changed |= self.carry(
                        instruction.result,
                        self.candidate(value, unwritten),
                        unwritten,
                    );
                }
            }
            I::Store(_, _, value, Value::Slot(slot)) if !self.taken[slot.index()] => {
                let symbol = self.candidate(*value, unwritten);
                changed |= merge(&mut self.held[slot.index()], symbol, unwritten);
            }
            I::Load(_, _, Value::Slot(slot)) => {
                if let Origin::Symbol(symbol) = self.held[slot.index()] {
                    changed |= self.carry(instruction.result, Some(symbol), unwritten);
                }
            }
            _ => (),
        }
        // Phi cycles and private slots use the same bounded eight rounds as
        // the frozen owner; conflict is a monotonic terminal state.
        changed
    }
}
fn merge(origin: &mut Origin, symbol: Option<SymbolId>, unwritten: &mut [bool]) -> bool {
    let Some(symbol) = symbol else {
        return false;
    };
    if *origin == Origin::Conflict || *origin == Origin::Symbol(symbol) {
        return false;
    }
    if let Origin::Symbol(previous) = *origin {
        unwritten[previous.index()] = false;
        unwritten[symbol.index()] = false;
        *origin = Origin::Conflict;
    } else {
        *origin = Origin::Symbol(symbol);
    }
    true
}

pub(super) fn find_unwritten(program: &Program) -> Vec<bool> {
    let mut unwritten = program
        .symbols()
        .iter()
        .map(|symbol| {
            program.global(symbol).is_some_and(|global| {
                !matches!(global.init, GlobalInit::Declaration)
                    && global.metadata.binding == Some(Binding::Internal)
                    && global.metadata.storage.is_none()
                    && global.metadata.flags & 64 == 0
            })
        })
        .collect::<Vec<_>>();
    for symbol in program.symbols() {
        let Some(global) = program.global(symbol) else {
            continue;
        };
        let mut escape = |target: SymbolId| {
            if program.symbol(target).name != symbol.name {
                unwritten[target.index()] = false;
            }
        };
        match &global.init {
            GlobalInit::Address(target, _) => escape(*target),
            GlobalInit::Data(items) => {
                for item in items {
                    if let DataItem::Address(_, target, _) = item {
                        escape(*target);
                    }
                }
            }
            _ => (),
        }
    }
    for symbol in program.symbols() {
        if let Some(body) = program
            .function(symbol)
            .and_then(|function| function.body.as_ref())
        {
            check_body(body, &mut unwritten);
        }
    }
    unwritten
}

fn check_body(body: &Body, unwritten: &mut [bool]) {
    let mut addresses = Addresses {
        carried: vec![Origin::Missing; body.temps().len()],
        held: vec![Origin::Missing; body.slots().len()],
        taken: vec![false; body.slots().len()],
    };
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            let instruction = &body.instruction(id).kind;
            let direct = matches!(instruction, I::Load(_, _, Value::Slot(_)))
                || matches!(instruction, I::Store(_, _, value, Value::Slot(_)) if !matches!(value, Value::Slot(_)));
            body.visit_values(instruction, |value| {
                if !direct && let Value::Slot(slot) = value {
                    addresses.taken[slot.index()] = true;
                }
            });
        }
    }
    for _ in 0..8 {
        let mut changed = false;
        for &block in &body.layout {
            for &id in &body.block(block).instructions {
                changed |= addresses.propagate(body.instruction(id), unwritten);
            }
        }
        if !changed {
            break;
        }
    }
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            let instruction = &body.instruction(id).kind;
            let mut position = 0;
            body.visit_values(instruction, |value| {
                let at = position;
                position += 1;
                if let Some(symbol) = addresses.candidate(value, unwritten) {
                    let reads = match instruction {
                        I::Load(..)
                        | I::Compare(..)
                        | I::Addr(_)
                        | I::Copy(..)
                        | I::Convert(..)
                        | I::Index(..)
                        | I::Binary(..)
                        | I::Phi(..) => true,
                        I::CopyObject(..) => at == 0,
                        I::Store(_, _, _, Value::Slot(slot)) => {
                            at == 0 && !addresses.taken[slot.index()]
                        }
                        _ => false,
                    };
                    if !reads {
                        unwritten[symbol.index()] = false;
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_internal_default_storage_definitions_start_as_candidates() {
        let program = crate::lowir::test_program(
            br"
global @mutable : i64 [binding=internal] = 3
global @constant : i64 [binding=internal, storage=readonly] = 4
global @thread : i64 [binding=internal, storage=thread_local] = 5
global @public : i64 = 6
global @weak : i64 [binding=weak, replaceable=yes] = 7
declare global @declaration : i64 [binding=internal]

function @read() -> i64 {
 block ^entry:
  %p = addr @mutable
  %v = load i64 %p
  return i64 %v
}",
        )
        .unwrap();
        let result = find_unwritten(&program);
        for (symbol, &unwritten) in program.symbols().iter().zip(&result) {
            assert_eq!(unwritten, program.names.get(symbol.name) == "mutable");
        }
    }

    #[test]
    fn addresses_read_through_private_slots_but_escape_through_taken_slots() {
        for (body, expected) in [
            (
                "slot $local : ptr\nblock ^entry: %p = addr @a store ptr %p, $local %q = load ptr $local %v = load i64 %q return i64 %v",
                true,
            ),
            (
                "slot $local : ptr\nblock ^entry: %s = addr $local %p = addr @a store ptr %p, $local %q = load ptr $local %v = load i64 %q return i64 %v",
                false,
            ),
            (
                "block ^entry: %p = addr @a %q = index i8 %p, 4 %v = load i64 %q return i64 %v",
                true,
            ),
            (
                "block ^entry: %p = addr @a %q = copy ptr %p %v = load i64 %q store i64 %v, %q return i64 %v",
                false,
            ),
            ("block ^entry: %p = addr @a return ptr %p", false),
        ] {
            let result_type = if body.contains("return ptr") {
                "ptr"
            } else {
                "i64"
            };
            let source = format!(
                "global @a : i64 [binding=internal] = 7\nfunction @f() -> {result_type} {{\n{body}\n}}\n"
            );
            let program = crate::lowir::test_program(source.as_bytes()).unwrap();
            assert_eq!(find_unwritten(&program)[0], expected, "{body}");
        }
    }

    #[test]
    fn globals_and_conflicting_phi_or_slot_origins_clear_both_candidates() {
        for source in [
            "global @holder : ptr = addr @a",
            "function @f() -> i64 {slot $local : ptr block ^entry: %p = addr @a %q = addr @b store ptr %p, $local store ptr %q, $local return i64 0}",
            "function @f(%c : i64) -> i64 {block ^entry: %p = addr @a %q = addr @b branch %c, ^left, ^right block ^left: jump ^join block ^right: jump ^join block ^join: %r = phi ptr [^left: %p, ^right: %q] %v = load i64 %r return i64 %v}",
        ] {
            let text = format!(
                "global @a : i64 [binding=internal] = 7\nglobal @b : i64 [binding=internal] = 9\n{source}\n"
            );
            let program = crate::lowir::test_program(text.as_bytes()).unwrap();
            let result = find_unwritten(&program);
            assert!(!result[0]);
            assert_eq!(result[1], source.starts_with("global"));
        }
        let program =
            crate::lowir::test_program(b"global @self : ptr [binding=internal] = addr @self\n")
                .unwrap();
        assert!(
            find_unwritten(&program)[0],
            "a self initializer is not another global's escape"
        );
    }
}
