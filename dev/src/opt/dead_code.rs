//! Mark and sweep from `opt/dead_code.cpp`. See NOTICE.
use super::pass_context::Context;
use crate::lowir::{
    Error,
    model::{InstructionKind as I, Returns, RuntimeInstruction as R, Unwind, Value},
    vocabulary::Effects,
};

fn removable(context: &Context<'_>, id: crate::lowir::model::InstId) -> bool {
    let instruction = context.body.instruction(id);
    if instruction.result.is_none() {
        return false;
    }
    match &instruction.kind {
        I::Const(..)
        | I::Copy(..)
        | I::Phi(..)
        | I::Addr(..)
        | I::Index(..)
        | I::Unary(..)
        | I::Binary(..)
        | I::Compare(..)
        | I::Select(..)
        | I::Convert(..)
        | I::Runtime(R::StackAlloc(_)) => true,
        I::Load(facts, ..) => !facts.volatile(),
        I::Call(call) => context.program.call_boundary(call).is_some_and(|s| {
            matches!(
                s.boundary.effects,
                Some(Effects::ReadNone | Effects::ReadOnly)
            ) && s.boundary.unwind == Unwind::No
                && s.boundary.returns != Returns::NoReturn
        }),
        _ => false,
    }
}
pub(super) fn remove(context: &mut Context<'_>) -> Result<bool, Error> {
    // Missing includes unlinked instructions; a linked use can still name one
    // while a pass is editing. Only Unknown contributes to the early-stop count.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        Missing,
        Live,
        Unknown,
    }
    let body = &*context.body;
    let mut live = vec![State::Missing; body.instructions().len()];
    let mut work = Vec::new();
    let mut unknown = 0_usize;
    for &block in &body.layout {
        for &id in &body.block(block).instructions {
            if removable(context, id) {
                live[id.index()] = State::Unknown;
                unknown += 1;
            } else {
                live[id.index()] = State::Live;
                work.push(id);
            }
        }
    }
    while unknown != 0 {
        let Some(id) = work.pop() else {
            break;
        };
        body.visit_values(&body.instruction(id).kind, |value| {
            let Value::Temp(temp) = value else {
                return;
            };
            if body.temp_is_parameter(temp) {
                return;
            }
            if let Some(def) = body.temp(temp).definition
                && live[def.index()] != State::Live
            {
                unknown -= usize::from(live[def.index()] == State::Unknown);
                live[def.index()] = State::Live;
                work.push(def);
            }
        });
    }
    if unknown == 0 {
        return Ok(false);
    }
    let keep = live
        .iter()
        .map(|&state| state == State::Live)
        .collect::<Vec<_>>();
    let changed = context.body.retain_instructions(&keep)?;
    context.dead = context
        .dead
        .checked_add(u64::try_from(unknown).map_err(|_| Error::Limit)?)
        .ok_or(Error::Limit)?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{model::Program, writer};
    fn transform(source: &str) -> (Program, u64) {
        let mut program = crate::lowir::test_program(source.as_bytes()).unwrap();
        let symbol = program
            .publications()
            .iter()
            .copied()
            .find(|&s| {
                program
                    .function(program.symbol(s))
                    .is_some_and(|f| f.body.is_some())
            })
            .unwrap();
        let mut body = program.take_body(symbol).unwrap().unwrap();
        let dead = {
            let mut context = Context::new(&program, symbol, &mut body, 1, &[]);
            let changed = remove(&mut context).unwrap();
            assert_eq!(changed, context.dead != 0);
            context.dead
        };
        body.compact(&mut program.names).unwrap();
        program.restore_body(symbol, body).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        (program, dead)
    }
    fn text(program: &Program) -> String {
        let mut bytes = Vec::new();
        writer::write(program, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    #[test]
    fn effect_and_unwind_promises_do_not_remove_atomic_volatile_or_nonreturning_roots() {
        let (program, dead) = transform(
            r"
declare function @pure() -> i64 [effects=readnone, unwind=no]
declare function @read() -> i64 [effects=readonly, unwind=no]
declare function @throws() -> i64 [effects=readnone]
declare function @writes() -> i64 [unwind=no]
declare function @nr() -> i64 [effects=readnone, unwind=no, return=noreturn]
declare function @void_pure() -> void [effects=readnone, unwind=no]
function @f(%p : ptr) -> void {
 block ^entry:
  %a = call i64 @pure()
  %b = call i64 @read()
  %c = call i64 @throws()
  %d = call i64 @writes()
  %e = load i64 %p
  %v = load volatile i64 %p
  %atomic = atomic_load i64 %p, 0
  %stack = stack_alloc 64
  call void @void_pure()
  %n = call i64 @nr()
  return void
}",
        );
        assert_eq!(dead, 4);
        let result = text(&program);
        for removed in ["%a =", "%b =", "%e =", "%stack ="] {
            assert!(!result.contains(removed), "{result}");
        }
        for kept in [
            "%c =",
            "%d =",
            "%v =",
            "%atomic =",
            "%n =",
            "call void @void_pure()",
        ] {
            assert!(result.contains(kept), "{result}");
        }
    }
    #[test]
    fn dead_phi_cycles_disappear_but_control_and_parameter_identities_stay() {
        let (program, dead) = transform(
            r"
function @f(%condition : i64) -> i64 {
 block ^entry: jump ^loop
 block ^loop:
  %a = phi i64 [^entry: 0, ^loop: %b]
  %b = copy i64 %a
  branch %condition, ^loop, ^exit
 block ^exit: return i64 0
}",
        );
        assert_eq!(dead, 2);
        let result = text(&program);
        assert!(!result.contains("phi") && !result.contains("copy"));
        assert!(result.contains("branch %condition, ^loop, ^exit"));
        let body = program
            .functions()
            .iter()
            .find_map(|f| f.body.as_ref())
            .unwrap();
        assert_eq!(body.temps().len(), 1);
        assert_eq!(body.temps()[0].uses, 1);
    }
    #[test]
    fn root_dependencies_keep_a_transitive_chain_and_unused_arithmetic_goes() {
        let (program, dead) = transform(
            r"
function @f(%p : ptr, %v : i64) -> i64 {
 block ^entry:
  %unused = const i64 99
  %a = binary add i64 %v, 1
  %b = binary mul i64 %a, 2
  %c = binary xor i64 %b, 7
  store i64 %b, %p
  return i64 %a
}",
        );
        assert_eq!(dead, 2);
        let result = text(&program);
        assert!(!result.contains("%unused") && !result.contains("%c ="));
        assert!(result.contains("%a = binary add") && result.contains("%b = binary mul"));
    }
}
