// Translated from lower/global_lowering.cpp::thread_local_initialization and
// function_lowering.cpp::lower_thread_local_initializer. See NOTICE.
//! A completed TLS definition owns its guard and first-use initialization body.
use super::{Error, function::Function, globals::GlobalOutput, symbols::Symbols};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Facts, InstructionKind as I, SymbolId, Value},
        vocabulary::{Binding, Compare, Type},
    },
    sema::{Graph, lowering::EntityId},
    support::stack::StackBudget,
    syntax::ast,
};

pub(super) fn initialize<'a>(
    output: &mut GlobalOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    source: (&'a Graph, &'a ast::Tree),
    entity: EntityId,
    initializer: Option<ast::Initializer>,
    dynamic: bool,
    budget: StackBudget,
) -> Result<(Option<SymbolId>, SymbolId), Error> {
    let (graph, tree) = source;
    let (guard, init, unordered) = symbols.tls_initializer(output.program, entity)?;
    let mut builder = FunctionBuilder::from_forward_boundary(output.program, init)?;
    builder.block("entry")?;
    let mut function = Function::new(graph, tree, builder, symbols, None, None, budget);
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
    let saved = function.symbols.replace_temporary_owner(Some((entity, 0)));
    let result = if dynamic {
        function
            .initialize_global(entity, initializer)
            .and_then(|()| {
                // lower_dynamic_initializer leaves a thread's reference binding in its
                // frame; construct_global ends an aggregate's initializer temporaries.
                if matches!(
                    graph.lower_type(graph.lower_entity_type(entity)?)?,
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
    function.symbols.replace_temporary_owner(saved);
    result?;
    if function.symbols.object_mode() {
        let global = function
            .symbols
            .variable_symbol(function.builder.program(), entity)?;
        function.register_destruction(global, graph.lower_entity_type(entity)?, true)?;
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
        .generated(&mut symbols.generated_definitions)?;
    output.definitions.stream(output.program, None)?;
    let mut entry = init;
    if symbols.object_mode() && !unordered {
        entry = symbols.tls_ordered_entry(output.program, init)?;
        if graph.lower_variable(entity)?.locality != crate::sema::lowering::Locality::Unit {
            let external = symbols.tls_init_entry(output.program, entity, true)?;
            forwarder(output.program, external, Some(entry))?;
            output.definitions.stream(output.program, None)?;
            let global = symbols.variable_symbol(output.program, entity)?;
            if output
                .program
                .global(output.program.symbol(global))
                .is_some_and(|g| g.metadata.binding == Some(Binding::Weak))
            {
                entry = external;
            }
        }
    }
    let global = symbols.variable_symbol(output.program, entity)?;
    symbols.tls_wrapper_symbol(output.program, entity, global)?;
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
            signature: None,
        }))?;
    }
    builder.effect(I::Return(Type::Void, None))?;
    builder.finish()?;
    Ok(())
}

pub(super) fn finish_unit<'a>(
    program: &mut crate::lowir::model::Program,
    definitions: &mut super::definition_order::Definitions,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    budget: StackBudget,
) -> Result<(), Error> {
    let Some(unit) = symbols.take_tls_unit() else {
        return Ok(());
    };
    let mut builder = FunctionBuilder::from_forward_boundary(program, unit.init)?;
    builder.block("entry")?;
    let mut function = Function::new(graph, tree, builder, symbols, None, None, budget);
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
            signature: None,
        }))?;
    }
    function.jump(done)?;
    function.place(done)?;
    function.effect(I::Return(Type::Void, None))?;
    function.builder.finish()?;
    definitions.stream(program, None)?;
    Ok(())
}

impl Function<'_, '_, '_, '_> {
    /// Both an id-expression and object member syntax reach the same TLS address.
    pub(super) fn global_place(&mut self, entity: EntityId) -> Result<Value, Error> {
        let symbol = self
            .symbols
            .variable_symbol(self.builder.program(), entity)?;
        self.emit_referenced_instance_variable(entity, symbol)?;
        if !self.graph.lower_variable(entity)?.thread_local {
            return Ok(Value::Symbol(symbol));
        }
        if let Some(wrapper) = self
            .symbols
            .tls_wrapper(self.builder.program(), entity, symbol)?
        {
            let arguments = self.builder.arguments(&[])?;
            Ok(self.builder.generated(I::Call(crate::lowir::model::Call {
                result: Type::Ptr,
                callee: Value::Symbol(wrapper),
                arguments,
                copy_elision: false,
                signature: None,
            }))?)
        } else {
            Ok(self.builder.generated(I::Addr(Value::Symbol(symbol)))?)
        }
    }
}
