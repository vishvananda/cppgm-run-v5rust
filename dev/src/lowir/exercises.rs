//! PA8 constructions translated from frozen `lowir/exercises.cpp`. See NOTICE.
use super::{
    Error,
    builder::FunctionBuilder,
    model::{Call, Facts, InstructionKind as I, Program},
    vocabulary::{Binary, Type},
};
pub(super) fn build(program: &mut Program, exercise: &str) -> Result<(), Error> {
    match exercise {
        "sum" => sum(program),
        "swap" => swap(program),
        "call" => call(program),
        _ => Err(Error::Diagnostic(format!(
            "unknown LowIR exercise: {exercise}"
        ))),
    }
}
fn sum(program: &mut Program) -> Result<(), Error> {
    let mut b = FunctionBuilder::new(program, "sum_to", &[("n", Type::I64)], Type::I64)?;
    b.block("entry")?;
    let n = b.parameter(0)?;
    let one = b.integer(1)?;
    let next = b.value(
        I::Binary(Binary::Add, Facts::default(), Type::I64, n, one),
        "n_plus_one",
    )?;
    let product = b.value(
        I::Binary(Binary::Mul, Facts::default(), Type::I64, n, next),
        "product",
    )?;
    let two = b.integer(2)?;
    let sum = b.value(
        I::Binary(Binary::UDiv, Facts::default(), Type::I64, product, two),
        "sum",
    )?;
    b.effect(I::Return(Type::I64, Some(sum)))?;
    b.finish()
}
fn swap(program: &mut Program) -> Result<(), Error> {
    let mut b = FunctionBuilder::new(
        program,
        "swap_values",
        &[("left", Type::Ptr), ("right", Type::Ptr)],
        Type::Void,
    )?;
    b.block("entry")?;
    let left = b.parameter(0)?;
    let right = b.parameter(1)?;
    let left_value = b.value(I::Load(Facts::default(), Type::I64, left), "left_value")?;
    let right_value = b.value(I::Load(Facts::default(), Type::I64, right), "right_value")?;
    b.effect(I::Store(Facts::default(), Type::I64, right_value, left))?;
    b.effect(I::Store(Facts::default(), Type::I64, left_value, right))?;
    b.effect(I::Return(Type::Void, None))?;
    b.finish()
}
fn call(program: &mut Program) -> Result<(), Error> {
    let mut b = FunctionBuilder::new(
        program,
        "call_twice",
        &[("fn", Type::Ptr), ("x", Type::I64)],
        Type::I64,
    )?;
    b.block("entry")?;
    let signature = Some(b.signature("value", Type::I64, Type::I64)?);
    let callee = b.parameter(0)?;
    let x = b.parameter(1)?;
    let arguments = b.arguments(&[x])?;
    let first = b.value(
        I::Call(Call {
            result: Type::I64,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature,
        }),
        "first",
    )?;
    let arguments = b.arguments(&[first])?;
    let second = b.value(
        I::Call(Call {
            result: Type::I64,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature,
        }),
        "second",
    )?;
    b.effect(I::Return(Type::I64, Some(second)))?;
    b.finish()
}
