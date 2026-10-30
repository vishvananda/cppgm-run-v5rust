// Translated from lower/global_lowering.cpp::thread_local_initialization and
// function_lowering.cpp::lower_thread_local_initializer. See NOTICE.
//! A completed TLS definition owns its guard and first-use initialization body.
use super::{Error, function::Function, globals::GlobalOutput};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Facts, InstructionKind as I, SymbolId, Value},
        vocabulary::{Binding, Compare, Type},
    },
    sema::lowering::EntityId,
    support::stack::StackBudget,
    syntax::ast,
};

pub(super) fn initialize(
    output: &mut GlobalOutput<'_>,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    dynamic: bool,
    budget: StackBudget,
) -> Result<(Option<SymbolId>, SymbolId), Error> {
    let (guard, init, unordered) = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .tls_initializer(output.program, entity)?;
    let mut builder = FunctionBuilder::from_forward_boundary(output.program, init)?;
    builder.block("entry")?;
    let mut function = Function::new(cx.borrow(), builder, None, None, budget);
    function.static_initializer = true;
    let run = guard
        .map(|_| function.new_block("local_static_ctor_run"))
        .transpose()?;
    let done = function.new_block("local_static_ctor_done")?;
    if let (Some(guard), Some(run)) = (guard, run) {
        let address = function.builder.generated(I::Addr(Value::Symbol(guard)))?;
        let loaded =
            function
                .builder
                .generated(I::Load(Facts::memory(false, true), Type::I64, address))?;
        let zero = function.builder.integer(0)?;
        let ready = function
            .builder
            .generated(I::Compare(Compare::Ne, Type::I64, loaded, zero))?;
        function.effect(I::Branch(ready, done, run))?;
        function.place(run)?;
    }
    let saved = function
        .symbols
        .view(&*function.graph, &*function.tree, function.cursor)
        .replace_temporary_owner(Some((entity, 0)));
    let result = if dynamic {
        function
            .initialize_global(entity, initializer)
            .and_then(|()| {
                // lower_dynamic_initializer leaves a thread's reference binding in its
                // frame; construct_global ends an aggregate's initializer temporaries.
                if matches!(
                    function
                        .graph
                        .lower_type(function.graph.lower_entity_type(entity)?)?,
                    crate::sema::lowering::TypeView::Reference { .. }
                ) {
                    Ok(())
                } else {
                    function.destroy_temporaries(0)
                }
            })
    } else {
        Ok(())
    };
    function
        .symbols
        .view(&*function.graph, &*function.tree, function.cursor)
        .replace_temporary_owner(saved);
    result?;
    if function
        .symbols
        .view(&*function.graph, &*function.tree, function.cursor)
        .object_mode()
    {
        let global = function
            .symbols
            .view(&*function.graph, &*function.tree, function.cursor)
            .variable_symbol(function.builder.program(), entity)?;
        function.register_destruction(global, function.graph.lower_entity_type(entity)?, true)?;
    }
    if let Some(guard) = guard {
        let one = function.builder.integer(1)?;
        function.effect(I::Store(
            Facts::memory(false, true),
            Type::I64,
            one,
            Value::Symbol(guard),
        ))?;
    }
    function.jump(done)?;
    function.place(done)?;
    function.effect(I::Return(Type::Void, None))?;
    function.builder.finish()?;
    output
        .definitions
        .generated(&mut cx.symbols.generated_definitions)?;
    cx.prepare_forward_boundaries(output.program)?;
    output.definitions.stream(output.program, None)?;
    let mut entry = init;
    if cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .object_mode()
        && !unordered
    {
        entry = cx
            .symbols
            .view(&*cx.graph, &*cx.tree, cx.cursor)
            .tls_ordered_entry(output.program, init)?;
        if cx.graph.lower_variable(entity)?.locality != crate::sema::lowering::Locality::Unit {
            let external = cx
                .symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .tls_init_entry(output.program, entity, true)?;
            forwarder(output.program, external, Some(entry))?;
            cx.prepare_forward_boundaries(output.program)?;
            output.definitions.stream(output.program, None)?;
            let global = cx
                .symbols
                .view(&*cx.graph, &*cx.tree, cx.cursor)
                .variable_symbol(output.program, entity)?;
            if output
                .program
                .global(output.program.symbol(global))
                .is_some_and(|g| g.metadata.binding == Some(Binding::Weak))
            {
                entry = external;
            }
        }
    }
    let global = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .variable_symbol(output.program, entity)?;
    cx.symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .tls_wrapper_symbol(output.program, entity, global)?;
    Ok((guard, entry))
}

pub(super) fn forwarder(
    program: &mut crate::lowir::model::Program,
    symbol: SymbolId,
    target: Option<SymbolId>,
) -> Result<(), Error> {
    let mut builder = FunctionBuilder::from_forward_boundary(program, symbol)?;
    builder.block("entry")?;
    if let Some(target) = target {
        let arguments = builder.arguments(&[])?;
        builder.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: Value::Symbol(target),
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature: None,
        }))?;
    }
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    Ok(())
}

pub(super) fn finish_unit(
    program: &mut crate::lowir::model::Program,
    definitions: &mut super::definition_order::Definitions,
    cx: &mut crate::lower::unit_frontend::Context<'_, '_, '_>,
    budget: StackBudget,
) -> Result<(), Error> {
    let Some(unit) = cx
        .symbols
        .view(&*cx.graph, &*cx.tree, cx.cursor)
        .take_tls_unit()
    else {
        return Ok(());
    };
    let mut builder = FunctionBuilder::from_forward_boundary(program, unit.init)?;
    builder.block("entry")?;
    let mut function = Function::new(cx.borrow(), builder, None, None, budget);
    let run = function.new_block("local_static_ctor_run")?;
    let done = function.new_block("local_static_ctor_done")?;
    let address = function
        .builder
        .generated(I::Addr(Value::Symbol(unit.guard)))?;
    let loaded = function
        .builder
        .generated(I::Load(Facts::default(), Type::I8, address))?;
    let zero = function.builder.integer(0)?;
    let ready = function
        .builder
        .generated(I::Compare(Compare::Ne, Type::I8, loaded, zero))?;
    function.effect(I::Branch(ready, done, run))?;
    function.place(run)?;
    let one = function.builder.integer(1)?;
    function.effect(I::Store(
        Facts::default(),
        Type::I8,
        one,
        Value::Symbol(unit.guard),
    ))?;
    for init in unit.initializers {
        let arguments = function.builder.arguments(&[])?;
        function.effect(I::Call(crate::lowir::model::Call {
            result: Type::Void,
            callee: Value::Symbol(init),
            arguments,
            copy_elision: false,
            argument_classes: (Vec::new()).into(),
            signature: None,
        }))?;
    }
    function.jump(done)?;
    function.place(done)?;
    function.effect(I::Return(Type::Void, None))?;
    function.builder.finish()?;
    cx.prepare_forward_boundaries(program)?;
    definitions.stream(program, None)?;
    Ok(())
}

impl Function<'_, '_, '_, '_> {
    /// Both an id-expression and object member syntax reach the same TLS address.
    pub(super) fn global_place(&mut self, entity: EntityId) -> Result<Value, Error> {
        let symbol = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .variable_symbol(self.builder.program(), entity)?;
        self.emit_referenced_instance_variable(entity, symbol)?;
        if !self.graph.lower_variable(entity)?.thread_local {
            return Ok(Value::Symbol(symbol));
        }
        if let Some(wrapper) = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .tls_wrapper(self.builder.program(), entity, symbol)?
        {
            let arguments = self.builder.arguments(&[])?;
            // expression_lowering.cpp::name_place uses emit_call(false):
            // a wrapper participates in the active cleanup segment even
            // though the wrapper call itself adds no throwing-call mark.
            self.before_lifetime_call(false)?;
            self.ensure_insertion()?;
            let value = self.builder.generated(I::Call(crate::lowir::model::Call {
                result: Type::Ptr,
                callee: Value::Symbol(wrapper),
                arguments,
                copy_elision: false,
                argument_classes: (Vec::new()).into(),
                signature: None,
            }))?;
            self.finish_lifetime_value(value, Type::Ptr, false)
        } else {
            Ok(self.builder.generated(I::Addr(Value::Symbol(symbol)))?)
        }
    }
}
