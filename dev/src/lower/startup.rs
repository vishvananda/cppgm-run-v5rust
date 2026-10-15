// Translated from lower/global_lowering.cpp::initializer_function and
// function_lowering.cpp::lower_dynamic_initializer/object_lowering.cpp::construct_global.
// See NOTICE.
//! One program-owned startup body; source facts and scratch end with each initializer.
use super::{Error, function::Function, symbols::Symbols, types};
use crate::{
    lowir::{
        builder::{FunctionBuilder, SuspendedFunction},
        model::{InstructionKind as I, Metadata, Program, SymbolId, Value},
        vocabulary::{Binding, Role, Type},
    },
    sema::{
        Graph,
        lowering::{EntityId, TypeView},
    },
    support::stack::StackBudget,
    syntax::ast,
};

#[derive(Default)]
pub(super) struct Startup {
    body: Option<SuspendedFunction>,
    next_block: u32,
    terminated: bool,
    selector: i64,
}
impl Startup {
    fn builder<'a>(
        &mut self,
        program: &'a mut Program,
        symbol: SymbolId,
    ) -> Result<FunctionBuilder<'a>, Error> {
        if let Some(body) = self.body.take() {
            return Ok(FunctionBuilder::resume(program, body));
        }
        let name = program.names.get(program.symbol(symbol).name).to_owned();
        let mut builder = FunctionBuilder::new(program, &name, &[], Type::Void)?;
        builder.metadata(Metadata {
            role: Some(Role::Init),
            binding: Some(Binding::Internal),
            ..Metadata::default()
        });
        builder.block("entry")?;
        Ok(builder)
    }
    pub(super) fn initialize<'a>(
        &mut self,
        program: &mut Program,
        symbols: &mut Symbols<'a, '_>,
        source: (&'a Graph, &'a ast::Tree),
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        budget: StackBudget,
    ) -> Result<(), Error> {
        let (graph, tree) = source;
        symbols.request_initialization(program)?;
        let symbol =
            symbols
                .initialization
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "startup symbol missing",
                )))?;
        let builder = self.builder(program, symbol)?;
        let mut function = Function::new(graph, tree, builder, symbols, None, None, budget);
        function.next_block = self.next_block;
        function.terminated = self.terminated;
        function.destruction.selector = self.selector;
        function.persistent_references = true;
        function.static_initializer = true;
        let saved = function.symbols.replace_temporary_owner(Some((entity, 0)));
        let result = function
            .initialize_global(entity, initializer)
            .and_then(|()| function.destroy_temporaries(0));
        function.symbols.replace_temporary_owner(saved);
        result?;
        self.next_block = function.next_block;
        self.terminated = function.terminated;
        self.selector = function.destruction.selector;
        self.body = Some(function.builder.suspend());
        Ok(())
    }
    pub(super) fn finish(&mut self, program: &mut Program, symbol: SymbolId) -> Result<(), Error> {
        if program.function_defined(symbol) {
            return Ok(());
        }
        let terminated = self.terminated;
        let mut builder = self.builder(program, symbol)?;
        if !terminated {
            builder.effect(I::Return(Type::Void, None))?;
        }
        builder.finish()?;
        Ok(())
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn initialize_global(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let symbol = self
            .symbols
            .variable_symbol(self.builder.program(), entity)?;
        let storage = Value::Symbol(symbol);
        match self.graph.lower_type(ty)? {
            TypeView::Class(_) => self.initialize_aggregate(
                entity,
                ty,
                storage,
                initializer,
                super::aggregates::NamedStorage::Static,
            ),
            TypeView::Array { mut element, .. } => {
                while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
                    element = inner;
                }
                if matches!(self.graph.lower_type(element)?, TypeView::Class(_)) {
                    if let Some(initializer) = initializer {
                        self.global_class_array(ty, storage, initializer)
                    } else {
                        self.local_array(entity, ty, storage, None)
                    }
                } else {
                    self.local_array(entity, ty, storage, initializer)
                }
            }
            TypeView::Reference { .. } => {
                let written = initializer.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "global reference without initializer",
                )))?;
                let expr = self.initializer_expression(written)?;
                let address = if self.persistent_references {
                    self.bind_static_reference(expr, ty)?
                } else {
                    self.bind_reference(expr, ty, super::conversions::Context::Initialization)?
                };
                self.effect(I::Store(
                    types::memory(self.graph, ty)?,
                    Type::Ptr,
                    address,
                    storage,
                ))
            }
            _ => self.initialize_scalar(ty, storage, initializer),
        }
    }
}
