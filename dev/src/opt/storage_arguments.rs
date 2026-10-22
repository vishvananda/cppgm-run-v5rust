//! Addressable scalar call arguments from opt/optimizer.cpp. See NOTICE.
use crate::lowir::{
    Error,
    model::{Body, Facts, Instruction, InstructionKind as I, Program, Value},
    vocabulary::Type,
};

pub(super) fn materialize(program: &mut Program) -> Result<(), Error> {
    for index in 0..program.symbols().len() {
        let symbol = program
            .find_symbol(program.symbols()[index].name)
            .ok_or(Error::Limit)?;
        if program.function(program.symbol(symbol)).is_none() {
            continue;
        }
        let Some(mut body) = program.take_body(symbol)? else {
            continue;
        };
        let result = materialize_body(program, &mut body);
        // A failed edit still returns its body to the owning program. No later
        // phase observes the partial edit because the error leaves the pipeline.
        program.restore_body(symbol, body)?;
        if result? {
            let signature = program
                .function(program.symbol(symbol))
                .ok_or(Error::Limit)?
                .signature;
            let parameters = program
                .signature(signature)
                .parameters
                .iter()
                .map(|parameter| parameter.name)
                .collect::<Vec<_>>();
            let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
            let result = body.name_optimizer_slots(&mut program.names, &parameters);
            program.restore_body(symbol, body)?;
            result?;
        }
    }
    Ok(())
}

fn materialize_body(program: &Program, body: &mut Body) -> Result<bool, Error> {
    let mut changed = false;
    for layout_index in 0..body.layout.len() {
        let block = body.layout[layout_index];
        // Rebuild each block's ID sequence once; inserting at every call would
        // repeatedly shift a growing vector. Instructions and values keep IDs.
        let original = std::mem::take(&mut body.block_mut(block).instructions);
        body.block_mut(block).instructions.reserve(original.len());
        for id in original {
            if let I::Call(call) = &body.instruction(id).kind
                && let Some(boundary) = program.call_boundary(call)
            {
                let arguments = call.arguments;
                let count = body
                    .arguments(arguments)
                    .len()
                    .min(boundary.parameters.len());
                for index in 0..count {
                    let argument = body.arguments(arguments)[index];
                    let Value::Temp(temp) = argument else {
                        continue;
                    };
                    if boundary.parameters[index].pass.is_none() {
                        continue;
                    }
                    let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
                    if ty == Type::Ptr || matches!(ty, Type::Object { .. }) {
                        continue;
                    }
                    let slot = body.refarg_slot(ty)?;
                    let result = body.generated_temp(Type::Ptr)?;
                    let line = body.instruction(id).line;
                    let address =
                        body.append(block, Instruction::new(Some(result), I::Addr(slot), line))?;
                    body.inherit_location(address, id);
                    let address = Value::Temp(result);
                    let store = body.append(
                        block,
                        Instruction::new(
                            None,
                            I::Store(Facts::memory(false, false), ty, argument, address),
                            line,
                        ),
                    )?;
                    body.inherit_location(store, id);
                    body.replace_argument(arguments, index, address)?;
                    changed = true;
                }
            }
            body.block_mut(block).instructions.push(id);
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_storage_arguments_keep_call_order_locations_and_distinct_slots() {
        let mut program = crate::lowir::test_program(
            br"
declare function @sink(%arg : ptr [pass=by_address], %other : ptr) -> void
function @f(%p : ptr, %object : obj<8x8>) -> void {
 slot $refarg__1 : i64
 block ^entry:
  %value = const i64 7
  %refarg__2 = const i64 8
  call void @sink(%value, %p) !dbg(test, 4, 2)
  call void @sink(5, %p)
  call void @sink(%p, %p)
  call void @sink(%object, %p)
  call void %p(%value) as (%arg : ptr [pass=indirect_result]) -> void !dbg(test, 8, 3)
  return void
}",
        )
        .unwrap();
        materialize(&mut program).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let function = program
            .functions()
            .iter()
            .find(|f| f.body.is_some())
            .unwrap();
        let body = function.body.as_ref().unwrap();
        assert_eq!(body.slots().len(), 3);
        let names = body
            .slots()
            .iter()
            .map(|slot| program.names.get(slot.name))
            .collect::<Vec<_>>();
        assert_eq!(names, ["refarg__1", "refarg__3", "refarg__4"]);
        let instructions = &body.block(body.layout[0]).instructions;
        let calls = instructions
            .iter()
            .enumerate()
            .filter_map(|(index, &id)| {
                matches!(body.instruction(id).kind, I::Call(_)).then_some((index, id))
            })
            .collect::<Vec<_>>();
        assert_eq!(calls.len(), 5);
        for (index, expected_line) in [(0, 4), (4, 8)] {
            let (position, call) = calls[index];
            let I::Call(call_value) = &body.instruction(call).kind else {
                panic!("call");
            };
            let argument = body.arguments(call_value.arguments)[0];
            let Value::Temp(temp) = argument else {
                panic!("materialized pointer");
            };
            assert_eq!(body.temp(temp).ty, Some(Type::Ptr));
            let address = instructions[position - 2];
            let store = instructions[position - 1];
            assert_eq!(body.temp(temp).definition, Some(address));
            assert!(matches!(
                body.instruction(address).kind,
                I::Addr(Value::Slot(_))
            ));
            assert!(
                matches!(body.instruction(store).kind, I::Store(facts, Type::I64, _, destination) if !facts.typed() && !facts.volatile() && destination == argument)
            );
            for id in [address, store, call] {
                assert_eq!(
                    body.location(body.instruction(id)).unwrap().line,
                    expected_line
                );
            }
        }
        let before = body.instructions().len();
        materialize(&mut program).unwrap();
        let body = program
            .functions()
            .iter()
            .find_map(|function| function.body.as_ref())
            .unwrap();
        assert_eq!(
            body.instructions().len(),
            before,
            "second preparation is idempotent"
        );
    }

    #[test]
    fn scalar_kinds_and_multiple_arguments_use_the_temporary_storage_type() {
        let mut program = crate::lowir::test_program(
            br"
declare function @sink(%a : ptr [pass=by_address], %b : ptr [pass=by_address]) -> void
function @f(%wide : i128, %floating : f64) -> void {
 block ^entry:
  call void @sink(%wide, %floating)
  return void
}",
        )
        .unwrap();
        materialize(&mut program).unwrap();
        crate::lowir::validate_test_program(&program).unwrap();
        let body = program
            .functions()
            .iter()
            .find_map(|f| f.body.as_ref())
            .unwrap();
        assert_eq!(
            body.slots().iter().map(|s| s.ty).collect::<Vec<_>>(),
            [Type::I128, Type::F64]
        );
        assert_eq!(body.instructions().len(), 6);
        let instructions = &body.block(body.layout[0]).instructions;
        assert!(matches!(
            body.instruction(instructions[1]).kind,
            I::Store(_, Type::I128, ..)
        ));
        assert!(matches!(
            body.instruction(instructions[3]).kind,
            I::Store(_, Type::F64, ..)
        ));
    }
}
