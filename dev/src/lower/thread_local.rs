// Translated from lower/global_lowering.cpp::thread_local_initialization and
// function_lowering.cpp::lower_thread_local_initializer. See NOTICE.
//! A completed TLS definition owns its guard and first-use initialization body.
use super::{Error, function::Function, globals::GlobalOutput, symbols::Symbols};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Facts, InstructionKind as I, Metadata, SymbolId, Value},
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
    budget: StackBudget,
) -> Result<(SymbolId, SymbolId), Error> {
    let (graph, tree) = source;
    let (guard, init) = symbols.tls_initialization_symbols(output.program, entity)?;
    let name = output
        .program
        .names
        .get(output.program.symbol(init).name)
        .to_owned();
    let mut builder = FunctionBuilder::new(output.program, &name, &[], Type::Void)?;
    builder.metadata(Metadata {
        binding: Some(Binding::Internal),
        ..Metadata::default()
    });
    builder.block("entry")?;
    let mut function = Function::new(graph, tree, builder, symbols, None, None, budget);
    function.persistent_references = true;
    function.static_initializer = true;
    let run = function.new_block("local_static_ctor_run")?;
    let done = function.new_block("local_static_ctor_done")?;
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
    let saved = function.symbols.replace_temporary_owner(Some((entity, 0)));
    let result = function
        .initialize_global(entity, initializer)
        .and_then(|()| function.destroy_temporaries(0));
    function.symbols.replace_temporary_owner(saved);
    result?;
    let one = function.builder.integer(1)?;
    function.effect(I::Store(
        Facts::memory(false, true),
        Type::I64,
        one,
        Value::Symbol(guard),
    ))?;
    function.jump(done)?;
    function.place(done)?;
    function.effect(I::Return(Type::Void, None))?;
    function.builder.finish()?;
    output.definitions.stream(output.program, None)?;
    let global = symbols.variable_symbol(output.program, entity)?;
    symbols.tls_wrapper_symbol(output.program, entity, global)?;
    Ok((guard, init))
}

impl Function<'_, '_, '_, '_> {
    /// Both an id-expression and object member syntax reach the same TLS address.
    pub(super) fn global_place(&mut self, entity: EntityId) -> Result<Value, Error> {
        let symbol = self
            .symbols
            .variable_symbol(self.builder.program(), entity)?;
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
