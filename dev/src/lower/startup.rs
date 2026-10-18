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

pub(super) struct Guard {
    pub shared: SymbolId,
    pub constructed: Option<SymbolId>,
}
pub(super) struct FunctionOutput<'a> {
    pub program: &'a mut Program,
    pub initialization: Option<InitializationOutput<'a>>,
}
impl<'a> FunctionOutput<'a> {
    pub(super) fn new(
        program: &'a mut Program,
        startup: &'a mut Startup,
        definitions: &'a mut super::definition_order::Definitions,
        finalizers: &'a mut Vec<super::shutdown::Finalizer>,
    ) -> Self {
        Self {
            program,
            initialization: Some(InitializationOutput {
                startup: Some(startup),
                definitions,
                finalizers,
            }),
        }
    }
}
pub(super) struct InitializationOutput<'a> {
    pub startup: Option<&'a mut Startup>,
    pub definitions: &'a mut super::definition_order::Definitions,
    pub finalizers: &'a mut Vec<super::shutdown::Finalizer>,
}
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
        (program, definitions, finalizers): (
            &mut Program,
            &mut super::definition_order::Definitions,
            &mut Vec<super::shutdown::Finalizer>,
        ),
        symbols: &mut Symbols<'a, '_>,
        source: (&'a Graph, &'a ast::Tree),
        (entity, initializer): (EntityId, Option<ast::Initializer>),
        budget: StackBudget,
        guard: Option<Guard>,
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
        function.initialization_output = Some(InitializationOutput {
            startup: None,
            definitions,
            finalizers,
        });
        let saved = function.symbols.replace_temporary_owner(Some((entity, 0)));
        let result = function.initialize_guarded_global(entity, initializer, guard);
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
impl<'a, 'source> Function<'a, 'source, '_, '_> {
    /// Suspend only this function's construction while a data address defines
    /// its dependencies. Startup fragments share the current body; ordinary
    /// functions borrow the unit's independent startup owner.
    pub(super) fn with_static_data<T>(
        &mut self,
        action: impl FnOnce(&mut dyn super::data::Output, &mut Symbols<'a, 'source>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let Some(mut output) = self.initialization_output.take() else {
            return action(self.builder.program(), self.symbols);
        };
        let inline = output.startup.is_none();
        let mut current = Startup {
            body: Some(self.builder.take_construction()),
            next_block: self.next_block,
            terminated: self.terminated,
            selector: self.destruction.selector,
        };
        let startup = output.startup.as_deref_mut().unwrap_or(&mut current);
        let result = action(
            &mut super::globals::GlobalOutput {
                program: self.builder.program(),
                startup,
                definitions: output.definitions,
                finalizers: output.finalizers,
            },
            self.symbols,
        );
        self.initialization_output = Some(output);
        if let Some(body) = current.body {
            self.builder.restore_construction(body);
            if inline {
                self.next_block = current.next_block;
                self.terminated = current.terminated;
                self.destruction.selector = current.selector;
            }
            result
        } else {
            // A failed nested initializer may already have consumed its body.
            // Preserve that failure rather than replacing its diagnostic.
            result.and(Err(crate::sema::Error::Invariant(
                "data dependency lost function construction",
            )
            .into()))
        }
    }
}
impl Function<'_, '_, '_, '_> {
    /// A first reference made by an initializer defines its dependencies at
    /// that exact position in the shared startup body. Move the builder while
    /// nested initializers borrow the program; each retains its own scratch.
    pub(super) fn emit_referenced_instance_variable(
        &mut self,
        entity: EntityId,
        symbol: SymbolId,
    ) -> Result<(), Error> {
        // A static-image attempt can have published this symbol while its
        // request still belongs to a suspended sibling queue. The evaluated
        // reference must finish its definition here too; the old queue entry
        // will then observe the published global and do no work.
        if self.initialization_output.is_some() {
            let program = self.builder.program();
            if program.global(program.symbol(symbol)).is_none()
                && self.graph.lower_demanded_initializer(entity).is_some()
            {
                self.symbols.instance_variables.push_front(entity);
            }
        }
        if self.symbols.instance_variables.is_empty() {
            return Ok(());
        }
        let (graph, tree) = (self.graph, self.tree);
        self.with_static_data(|output, symbols| output.referenced_variable(symbols, graph, tree))
    }
    fn initialize_guarded_global(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        guard: Option<Guard>,
    ) -> Result<(), Error> {
        let Some(guard) = guard else {
            self.initialize_global(entity, initializer)?;
            return self.destroy_temporaries(0);
        };
        let ty = self.graph.lower_entity_type(entity)?;
        let flag = Value::Symbol(guard.shared);
        let loaded = self.emit(
            I::Load(
                crate::lowir::model::Facts::memory(false, true),
                Type::I64,
                flag,
            ),
            ty,
        )?;
        let zero = self.builder.integer(0)?;
        let test = self.emit(
            I::Compare(
                crate::lowir::vocabulary::Compare::Ne,
                Type::I64,
                loaded.value,
                zero,
            ),
            ty,
        )?;
        let run = self.new_block("local_static_ctor_run")?;
        let done = self.new_block("local_static_ctor_done")?;
        self.effect(I::Branch(test.value, done, run))?;
        self.terminated = true;
        self.place(run)?;
        let one = self.builder.integer(1)?;
        self.effect(I::Store(
            crate::lowir::model::Facts::memory(false, true),
            Type::I64,
            one,
            flag,
        ))?;
        self.initialize_global(entity, initializer)?;
        self.destroy_temporaries(0)?;
        if let Some(constructed) = guard.constructed {
            let one = self.builder.integer(1)?;
            self.effect(I::Store(
                crate::lowir::model::Facts::default(),
                Type::I8,
                one,
                Value::Symbol(constructed),
            ))?;
        }
        self.jump(done)?;
        self.place(done)
    }
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
