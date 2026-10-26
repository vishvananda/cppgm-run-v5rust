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
    pub guard: Option<SymbolId>,
    pub constructed: Option<SymbolId>,
    pub reference_temporary: bool,
}
pub(super) fn for_type(
    program: &mut Program,
    symbols: &mut super::symbols::Symbols<'_, '_>,
    graph: &crate::sema::Graph,
    global: SymbolId,
    ty: crate::sema::lowering::TypeId,
    guard: Option<SymbolId>,
) -> Result<Option<Finalizer>, Error> {
    let Some(destructor) = symbols.class_destructor(ty)? else {
        return Ok(None);
    };
    symbols.demand(destructor);
    let callee = symbols.function_symbol(program, destructor)?;
    let mut element = ty;
    let mut count = 1_u64;
    let mut array = false;
    while let crate::sema::lowering::TypeView::Array {
        element: inner,
        bound,
    } = graph.lower_type(element)?
    {
        array = true;
        count = count.wrapping_mul(bound);
        element = inner;
    }
    Ok(Some(Finalizer {
        global,
        callee,
        count,
        array,
        element_size: super::types::layout(graph, element)?.0,
        guard,
        constructed: None,
        reference_temporary: false,
    }))
}
pub(super) fn finish(
    program: &mut Program,
    symbol: SymbolId,
    finalizers: &[Finalizer],
) -> Result<(), Error> {
    finish_body(program, symbol, finalizers, Some(Role::Fini))
}
pub(super) fn local(
    program: &mut Program,
    symbol: SymbolId,
    finalizers: &[Finalizer],
) -> Result<(), Error> {
    finish_body(program, symbol, finalizers, None)
}
fn finish_body(
    program: &mut Program,
    symbol: SymbolId,
    finalizers: &[Finalizer],
    role: Option<Role>,
) -> Result<(), Error> {
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let mut builder = FunctionBuilder::new(program, &name, &[], Type::Void)?;
    builder.metadata(Metadata {
        role,
        binding: Some(Binding::Internal),
        ..Metadata::default()
    });
    if role.is_none() {
        builder.boundary(crate::lowir::model::Boundary {
            unwind: crate::lowir::model::Unwind::No,
            ..crate::lowir::model::Boundary::default()
        });
    }
    builder.block("entry")?;
    let mut next_block = 0_u32;
    for finalizer in finalizers.iter().rev() {
        let skip = if let Some(flag) = finalizer.constructed {
            let destroy = block(&mut builder, "ref_destroy", &mut next_block)?;
            let skip = block(&mut builder, "ref_destroy_done", &mut next_block)?;
            let ready =
                builder.generated(I::Load(Facts::default(), Type::I8, Value::Symbol(flag)))?;
            builder.effect(I::Branch(ready, destroy, skip))?;
            builder.place_block(destroy)?;
            Some(skip)
        } else if let Some(guard) = finalizer.guard {
            let destroy = block(&mut builder, "local_static_destroy", &mut next_block)?;
            let skip = block(&mut builder, "local_static_fini_next", &mut next_block)?;
            let address = builder.generated(I::Addr(Value::Symbol(guard)))?;
            let loaded =
                builder.generated(I::Load(Facts::memory(false, true), Type::I64, address))?;
            let zero = builder.integer(0)?;
            let ready = builder.generated(I::Compare(Compare::Ne, Type::I64, loaded, zero))?;
            builder.effect(I::Branch(ready, destroy, skip))?;
            builder.place_block(destroy)?;
            Some(skip)
        } else {
            None
        };
        if finalizer.count > 8 {
            array_loop(&mut builder, finalizer, &mut next_block)?;
        } else {
            let base = if finalizer.reference_temporary && finalizer.array {
                Some(builder.generated(I::Addr(Value::Symbol(finalizer.global)))?)
            } else {
                None
            };
            for index in (0..finalizer.count).rev() {
                let mut object = if let Some(base) = base {
                    base
                } else {
                    builder.generated(I::Addr(Value::Symbol(finalizer.global)))?
                };
                if base.is_some() {
                    let offset =
                        builder.integer(u128::from(index.wrapping_mul(finalizer.element_size)))?;
                    object = builder.generated(I::Index(
                        Type::I8,
                        Some(Projection::ArrayElement),
                        object,
                        offset,
                    ))?;
                } else if finalizer.array {
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
        if let Some(skip) = skip {
            builder.effect(I::Jump(skip))?;
            builder.place_block(skip)?;
        }
    }
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    Ok(())
}
fn block(
    builder: &mut FunctionBuilder<'_>,
    prefix: &str,
    next: &mut u32,
) -> Result<crate::lowir::model::BlockId, Error> {
    *next = next.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
    let name = format!("{prefix}_{next}");
    Ok(builder.generated_block(&name, prefix)?)
}
fn call(builder: &mut FunctionBuilder<'_>, callee: SymbolId, object: Value) -> Result<(), Error> {
    let arguments = builder.arguments(&[object])?;
    builder.effect(I::Call(Call {
        result: Type::Void,
        callee: Value::Symbol(callee),
        arguments,
        signature: None,
        copy_elision: false,
        argument_classes: Vec::new(),
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
    let test = block(builder, "destructor_array_cond", next)?;
    let body = block(builder, "destructor_array_body", next)?;
    let end = block(builder, "destructor_array_end", next)?;
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
