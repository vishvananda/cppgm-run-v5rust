// Translated from lower/global_lowering.cpp::finalizer_function and
// object_lowering.cpp::finalize_global. See NOTICE.
//! Program-owned finalization retains only typed `LowIR` symbols and target sizes.
use super::Error;
use crate::lowir::{
    builder::FunctionBuilder,
    model::{Call, Facts, InstructionKind as I, Metadata, Program, SymbolId, Value},
    vocabulary::{Binary, Binding, Compare, Projection, Role, Type},
};
pub(super) struct Finalizer {
    pub global: SymbolId,
    pub callee: SymbolId,
    pub count: u64,
    pub element_size: u64,
    pub array: bool,
}
pub(super) fn finish(
    program: &mut Program,
    symbol: SymbolId,
    finalizers: &[Finalizer],
) -> Result<(), Error> {
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let mut builder = FunctionBuilder::new(program, &name, &[], Type::Void)?;
    builder.metadata(Metadata {
        role: Some(Role::Fini),
        binding: Some(Binding::Internal),
        ..Metadata::default()
    });
    builder.block("entry")?;
    let mut next_block = 0_u32;
    for finalizer in finalizers.iter().rev() {
        if finalizer.count > 8 {
            array_loop(&mut builder, finalizer, &mut next_block)?;
        } else {
            for index in (0..finalizer.count).rev() {
                let mut object = builder.generated(I::Addr(Value::Symbol(finalizer.global)))?;
                if finalizer.array {
                    let index = builder.integer(u128::from(index))?;
                    let offset = if finalizer.element_size == 1 {
                        index
                    } else {
                        let size = builder.integer(u128::from(finalizer.element_size))?;
                        builder.generated(I::Binary(
                            Binary::Mul,
                            Facts::default(),
                            Type::I64,
                            index,
                            size,
                        ))?
                    };
                    object = builder.generated(I::Index(
                        Type::I8,
                        Some(Projection::ArrayElement),
                        object,
                        offset,
                    ))?;
                }
                call(&mut builder, finalizer.callee, object)?;
            }
        }
    }
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    Ok(())
}
fn call(builder: &mut FunctionBuilder<'_>, callee: SymbolId, object: Value) -> Result<(), Error> {
    let arguments = builder.arguments(&[object])?;
    builder.effect(I::Call(Call {
        result: Type::Void,
        callee: Value::Symbol(callee),
        arguments,
        signature: None,
        copy_elision: false,
    }))?;
    Ok(())
}
fn array_loop(
    builder: &mut FunctionBuilder<'_>,
    finalizer: &Finalizer,
    next: &mut u32,
) -> Result<(), Error> {
    let base = builder.generated(I::Addr(Value::Symbol(finalizer.global)))?;
    let slot = builder.generated_slot("destructor_array_index", Type::I64)?;
    let count = builder.integer(u128::from(finalizer.count))?;
    builder.effect(I::Store(Facts::memory(false, true), Type::I64, count, slot))?;
    let mut block = |prefix: &str| -> Result<_, Error> {
        *next = next.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        Ok(builder.block_id(&format!("{prefix}_{next}"))?)
    };
    let test = block("destructor_array_cond")?;
    let body = block("destructor_array_body")?;
    let end = block("destructor_array_end")?;
    builder.effect(I::Jump(test))?;
    builder.place_block(test)?;
    let current = builder.generated(I::Load(Facts::memory(false, true), Type::I64, slot))?;
    let zero = builder.integer(0)?;
    let condition = builder.generated(I::Compare(Compare::Ne, Type::I64, current, zero))?;
    builder.effect(I::Branch(condition, body, end))?;
    builder.place_block(body)?;
    let one = builder.integer(1)?;
    let last = builder.generated(I::Binary(
        Binary::Sub,
        Facts::default(),
        Type::I64,
        current,
        one,
    ))?;
    builder.effect(I::Store(Facts::memory(false, true), Type::I64, last, slot))?;
    let offset = if finalizer.element_size == 1 {
        last
    } else {
        let size = builder.integer(u128::from(finalizer.element_size))?;
        builder.generated(I::Binary(
            Binary::Mul,
            Facts::default(),
            Type::I64,
            last,
            size,
        ))?
    };
    let object = builder.generated(I::Index(
        Type::I8,
        Some(Projection::ArrayElement),
        base,
        offset,
    ))?;
    call(builder, finalizer.callee, object)?;
    builder.effect(I::Jump(test))?;
    builder.place_block(end)?;
    Ok(())
}
