// Translated from lower/function_lowering.cpp and statement_lowering.cpp. See NOTICE.
//! Each body owns local places, temporary numbering, and byte-budget recursion guards.
use super::{Error, symbols::Symbols, types};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Arity, Boundary, InstructionKind as I, Program, Unwind, Value},
        vocabulary::Type,
    },
    sema::{
        Graph,
        lowering::{EntityId, TypeId, TypeView},
    },
    support::stack::StackBudget,
    syntax::ast::{self, DeclKind},
};
use std::collections::HashMap;
#[derive(Clone, Copy)]
pub(super) struct RValue {
    pub value: Value,
    pub ty: Type,
    pub source: TypeId,
    pub literal: bool,
    pub bits: Option<u64>,
}
pub(super) struct Function<'a, 'source, 'program, 'symbols> {
    pub graph: &'a Graph,
    pub tree: &'a ast::Tree,
    pub builder: FunctionBuilder<'program>,
    pub symbols: &'symbols mut Symbols<'a, 'source>,
    pub locals: HashMap<EntityId, Value>,
    pub untyped_places: std::collections::HashSet<Value>,
    pub empty_storage: super::member_initialization::EmptyStorage,
    pub bit_initialization: super::bit_fields::Initialization,
    pub bit_initialization_order: super::bit_fields::InitializationOrder,
    pub named_path: Vec<super::aggregates::Step>,
    pub aggregation: super::aggregate_cleanup::State,
    pub entity: Option<EntityId>,
    pub name_context: Option<crate::sema::lowering::SourceFunction>,
    pub this_slot: Option<Value>,
    pub virtual_context: super::construction_context::FunctionContext,
    pub call_object_class: Option<EntityId>,
    pub captured_members: HashMap<EntityId, EntityId>,
    pub closure_this: Option<EntityId>,
    pub persistent_references: bool,
    pub static_temporaries: Option<super::local_storage::StaticTemporaries>,
    pub static_initializer: bool,
    pub initialization_output: Option<super::startup::InitializationOutput<'symbols>>,
    // While a converting constructor consumes its source, its first standard
    // conversion is active; the outer construction use must not recur.
    pub source_site: Option<crate::support::source::SourceOffset>,
    pub converting_source: Option<ast::ExprId>,
    pub initialization_context: super::class_results::InitializationContext,
    pub reference_lifetimes: super::reference_lifetimes::Extensions,
    pub next_static: u32,
    pub next_block: u32,
    pub jumps: Vec<super::statements::JumpTargets>,
    pub switch_blocks: HashMap<ast::StmtId, crate::lowir::model::BlockId>,
    pub label_memo: HashMap<ast::StmtId, bool>,
    pub labels: HashMap<crate::lex::identifiers::NameId, crate::lowir::model::BlockId>,
    pub folded_targets: Vec<crate::lowir::model::BlockId>,
    pub conditional_slot: Option<Value>,
    pub lifetime: super::lifetime::Lifetime,
    pub handlers: super::handlers::Handlers,
    pub lexical: super::lexical_cleanup::Lexical,
    pub destruction: super::destruction::Destruction,
    pub construction: super::constructor_cleanup::Construction,
    pub copying: super::copy_cleanup::State,
    pub element_temporaries: super::element_temporaries::State,
    pub terminated: bool,
    pub result: Option<TypeId>,
    pub result_destination: Option<Value>,
    pub result_storage: Option<Value>,
    pub fallback_returns: Vec<super::class_results::FallbackReturn>,
    pub returned_local: Option<EntityId>,
    pub budget: StackBudget,
    pub stack_base: usize,
}
pub(super) fn definition_body(
    graph: &Graph,
    tree: &ast::Tree,
    entity: EntityId,
) -> Result<Option<ast::BodyId>, Error> {
    if let Some(body) = graph.lower_function_body(entity) {
        return Ok(Some(body));
    }
    Ok(
        match graph
            .lower_function(entity)?
            .definition
            .map(|id| &tree.declarations.get(id).kind)
        {
            Some(ast::DeclKind::Function { body, .. }) => Some(*body),
            Some(ast::DeclKind::SpecialMember { .. }) => graph.lower_function_body(entity),
            Some(ast::DeclKind::Simple { .. }) if graph.lower_assignment(entity).is_some() => None,
            None => None,
            _ => return Err(Error::Missing("special function definition lowering")),
        },
    )
}
fn require_analyzed_body(
    graph: &Graph,
    tree: &ast::Tree,
    entity: Option<EntityId>,
    body: Option<ast::BodyId>,
) -> Result<(), Error> {
    if let Some(entity) = entity
        && graph.lower_reserved_body(entity)
        && let Some(body) = body
        && let ast::Body::Parsed(body) = tree.bodies.get(body)
    {
        return Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location: tree.statements.get(body.compound).location,
            message: "an emitted inline function that calls a reserved builtin this compiler lacks",
        }));
    }
    Ok(())
}

pub(super) fn lower<'a>(
    output: super::startup::FunctionOutput<'_>,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    entry: super::construction::Entry,
    body: Option<ast::BodyId>,
    budget: StackBudget,
) -> Result<(), Error> {
    require_analyzed_body(graph, tree, entry.entity(graph), body)?;
    let program = output.program;
    let facts = entry.facts(graph)?;
    let entity = entry.entity(graph);
    let TypeView::Function {
        result,
        parameters,
        variadic,
    } = graph.lower_type(facts.ty)?
    else {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "function signature missing",
        )));
    };
    let parameter_bindings: Vec<_> = graph.lower_parameters(facts.scope).collect();
    if parameters.len() != parameter_bindings.len() {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "function parameter binding mismatch",
        )));
    }
    let mut reserved = std::collections::HashSet::new();
    for &(_, name, _) in &parameter_bindings {
        if let Some(name) = name {
            reserved.insert(symbols.parameter_name(name)?);
        }
    }
    let object_count = usize::from(facts.object.is_some());
    let hidden_count = usize::from(entry.hidden_vtt(graph)?);
    let result_boundary = symbols.result_boundary(result)?;
    let result_count = usize::from(result_boundary.indirect);
    let mut names = Vec::with_capacity(parameters.len() + object_count + result_count);
    if result_boundary.indirect {
        names.push(("ret".to_owned(), Type::Ptr));
    }
    if object_count != 0 {
        names.push(("this".to_owned(), Type::Ptr));
    }
    if hidden_count != 0 {
        names.push(("__vtt".to_owned(), Type::Ptr));
    }
    for &(_, name, ty) in &parameter_bindings {
        names.push((
            if entity.is_some_and(|entity| {
                graph.lower_implicit_assignment(entity) || graph.lower_implicit_copy(entity)
            }) {
                "other".to_owned()
            } else if let Some(name) = name {
                symbols.parameter_name(name)?
            } else {
                let mut name = format!("__param{}", names.len());
                while reserved.contains(&name) {
                    name.push('_');
                }
                name
            },
            parameter_type(graph, ty)?,
        ));
    }
    let symbol = symbols.entry_symbol(program, entry)?;
    let replaces_entry = symbols.definition_replacement(program, entry, symbol)?;
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let metadata = symbols.definition_metadata(program, entry, symbol)?;
    let is_main = metadata.role == Some(crate::lowir::vocabulary::Role::Entry);
    let parameters: Vec<_> = names
        .iter()
        .map(|(name, ty)| (name.as_str(), *ty))
        .collect();
    let mut builder = if replaces_entry {
        FunctionBuilder::rebuild_abi_entry(program, symbol, &parameters, result_boundary.ty)?
    } else {
        FunctionBuilder::new(program, &name, &parameters, result_boundary.ty)?
    };
    builder.metadata(metadata);
    let distinct = entity.is_some_and(|entity| graph.lower_copy_constructor(entity));
    if result_boundary.indirect {
        builder.parameter_boundary(
            0,
            Some(crate::lowir::vocabulary::Pass::IndirectResult),
            types::layout(graph, result)?.0,
            false,
        )?;
    }
    if let Some(class) = facts.object {
        builder.parameter_boundary(
            result_count,
            None,
            types::layout(graph, graph.lower_entity_type(class)?)?.0,
            distinct,
        )?;
    }
    for (index, &ty) in parameters_types(graph, facts.ty)?.iter().enumerate() {
        let (pass, bytes) = reference_boundary(graph, ty)?;
        builder.parameter_classes(
            index + object_count + result_count + hidden_count,
            symbols.parameter_classes(ty)?,
        )?;
        builder.parameter_boundary(
            index + object_count + result_count + hidden_count,
            pass,
            bytes,
            distinct && index == 0 || parameter_no_alias(facts.builtin, index),
        )?;
    }
    let mut boundary = Boundary {
        result_classes: result_boundary.classes,
        arity: if variadic {
            Arity::Variadic
        } else {
            Arity::Fixed
        },
        unwind: if facts.nothrow {
            Unwind::No
        } else {
            Unwind::Default
        },
        ..Boundary::default()
    };
    builtin_boundary(&mut boundary, facts.builtin);
    declared_effects(&mut boundary, graph, entity);
    builder.boundary(boundary);
    builder.block("entry")?;
    let mut function = Function::new(graph, tree, builder, symbols, entity, Some(result), budget);
    function.name_context = entry.source_function(graph);
    function.initialization_output = output.initialization;
    function.virtual_context.class = facts.object;
    if result_boundary.indirect {
        function.result_destination = Some(function.builder.parameter(0)?);
    }
    let body = body
        .map(|id| {
            let ast::Body::Parsed(body) = tree.bodies.get(id) else {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "lowering deferred body",
                )));
            };
            Ok(body)
        })
        .transpose()?;
    function.lifetime.body = body.map(|body| body.compound);
    if result_boundary.indirect
        && let Some(body) = body.filter(|body| body.handlers.is_none())
    {
        function.find_returned_local(body.compound, result)?;
    }
    let deleting = matches!(entry, super::construction::Entry::DestructorDeleting(_));
    let calls_complete = if deleting {
        !function.symbols.deleting_inlines_body(
            entity.ok_or(crate::sema::Error::Invariant("deleting entity"))?,
        )?
    } else {
        false
    };
    let specification = entity.filter(|&entity| {
        function.symbols.host_objects() && graph.lower_throw_specification(entity).is_some()
    });
    if (facts.nothrow || specification.is_some()) && !calls_complete {
        let raises = body
            .map(|body| function.statement_throws(body.compound))
            .transpose()?
            .unwrap_or(false)
            || if let super::construction::Entry::Constructor(id, _) = entry {
                function.constructor_plan_throws(id)?
            } else if matches!(
                facts.name,
                crate::sema::lowering::FunctionName::Destructor(_)
            ) {
                facts
                    .object
                    .map(|class| graph.lower_destructor_subobjects_throw(class))
                    .transpose()?
                    .unwrap_or(false)
            } else {
                false
            };
        if raises {
            function.lifetime.unexpected_specification = specification;
            function.open_function_guard()?;
        }
    }
    if object_count != 0 {
        function.initialize_this(result_count)?;
    }
    if hidden_count != 0 {
        function.virtual_context.vtt =
            Some(function.builder.parameter(object_count + result_count)?);
    }
    function.initialize_parameters(
        &parameter_bindings,
        &names,
        object_count + result_count + hidden_count,
    )?;
    if let Some(entity) = entity
        && graph.lower_assignment(entity).is_some()
    {
        function.assign_members(entity)?;
    }
    function.source_body(entry, body, calls_complete)?;
    if !function.terminated {
        let ty = result_boundary.ty;
        if ty != Type::Void
            && !is_main
            && function
                .builder
                .insertion_targeted(&function.folded_targets)?
        {
            function.builder.effect(I::Unreachable)?;
        } else {
            let value = function.zero_return(result)?;
            function.effect(I::Return(ty, value))?;
        }
    }
    function.finish_constructor_cleanups()?;
    // An empty trailing region can leave its already allocated handlers
    // pending. finish_body publishes those blocks even without a later call.
    function.place_cleanup_chains(false)?;
    function.place_lexical_cleanups()?;
    function.place_cleanup_chains(true)?;
    function.place_destructor_terminates()?;
    function.finish_fallback_returns()?;
    function.place_function_guard(entry)?;
    // The terminate landing itself has a no-return continuation. Its
    // fallback slot follows the guard's exception slot and earlier patches.
    function.finish_fallback_returns()?;
    function.builder.finish()?;
    super::definition_demands::finish_function_definition(symbols, graph, program, entry, symbol)?;
    Ok(())
}
impl<'a, 'source, 'program, 'symbols> Function<'a, 'source, 'program, 'symbols> {
    pub(super) fn new(
        graph: &'a Graph,
        tree: &'a ast::Tree,
        builder: FunctionBuilder<'program>,
        symbols: &'symbols mut Symbols<'a, 'source>,
        entity: Option<EntityId>,
        result: Option<TypeId>,
        budget: StackBudget,
    ) -> Self {
        let marker = 0u8;
        Self {
            graph,
            tree,
            builder,
            symbols,
            locals: HashMap::new(),
            untyped_places: std::collections::HashSet::new(),
            empty_storage: super::member_initialization::EmptyStorage::Owned,
            bit_initialization: super::bit_fields::Initialization::default(),
            bit_initialization_order: super::bit_fields::InitializationOrder::Ordinary,
            named_path: Vec::new(),
            aggregation: super::aggregate_cleanup::State::default(),
            entity,
            name_context: entity.map(crate::sema::lowering::SourceFunction::Function),
            this_slot: None,
            virtual_context: super::construction_context::FunctionContext::default(),
            call_object_class: None,
            captured_members: entity
                .and_then(|function| graph.lower_call_closure(function))
                .map(|class| {
                    graph
                        .lower_closure_captures(class)
                        .iter()
                        .map(|capture| (capture.entity, capture.member))
                        .collect()
                })
                .unwrap_or_default(),
            closure_this: entity
                .and_then(|function| graph.lower_call_closure(function))
                .and_then(|class| graph.lower_closure(class))
                .and_then(|closure| closure.this_member),
            persistent_references: false,
            static_temporaries: None,
            static_initializer: false,
            initialization_output: None,
            converting_source: None,
            source_site: None,
            initialization_context: super::class_results::InitializationContext::Ordinary,
            reference_lifetimes: super::reference_lifetimes::Extensions::default(),
            next_static: 0,
            next_block: 0,
            jumps: Vec::new(),
            switch_blocks: HashMap::new(),
            label_memo: HashMap::new(),
            labels: HashMap::new(),
            folded_targets: Vec::new(),
            conditional_slot: None,
            lifetime: super::lifetime::Lifetime::default(),
            handlers: super::handlers::Handlers::default(),
            lexical: super::lexical_cleanup::Lexical::default(),
            destruction: super::destruction::Destruction::default(),
            construction: super::constructor_cleanup::Construction::default(),
            copying: super::copy_cleanup::State::default(),
            element_temporaries: super::element_temporaries::State::default(),
            terminated: false,
            result,
            result_destination: None,
            result_storage: None,
            fallback_returns: Vec::new(),
            returned_local: None,
            budget,
            stack_base: std::ptr::from_ref(&marker).addr(),
        }
    }
}
impl Function<'_, '_, '_, '_> {
    fn initialize_parameters(
        &mut self,
        bindings: &[(EntityId, Option<crate::lex::identifiers::NameId>, TypeId)],
        names: &[(String, Type)],
        first: usize,
    ) -> Result<(), Error> {
        for (ordinal, &(entity, name, parameter)) in bindings.iter().enumerate() {
            let index = ordinal + first;
            let storage = types::storage(self.graph, parameter)?;
            let slot = if name.is_some()
                || self.entity.is_some_and(|e| {
                    self.graph.lower_implicit_assignment(e) || self.graph.lower_implicit_copy(e)
                }) {
                self.builder.slot(&names[index].0, storage)?
            } else {
                self.builder.parameter_slot(index, storage)?
            };
            let incoming = self.builder.parameter(index)?;
            if matches!(self.graph.lower_type(parameter)?, TypeView::Class(class) if !self.graph.lower_direct_class_parameter(class)?)
                && !self
                    .entity
                    .is_some_and(|entity| self.graph.lower_positional_parameters(entity))
            {
                self.locals.insert(entity, incoming);
            } else {
                self.initialize_parameter(parameter, incoming, slot)?;
                self.locals.insert(entity, slot);
            }
        }
        Ok(())
    }
    fn initialize_this(&mut self, index: usize) -> Result<(), Error> {
        let slot = self.builder.slot("this", Type::Ptr)?;
        let incoming = self.builder.parameter(index)?;
        self.builder.effect(I::Store(
            crate::lowir::model::Facts::memory(false, true),
            Type::Ptr,
            incoming,
            slot,
        ))?;
        self.this_slot = Some(slot);
        Ok(())
    }
    fn initialize_parameter(
        &mut self,
        ty: TypeId,
        incoming: Value,
        slot: Value,
    ) -> Result<(), Error> {
        if let TypeView::Class(class) = self.graph.lower_type(ty)?
            && !self
                .entity
                .is_some_and(|entity| self.graph.lower_positional_parameters(entity))
        {
            if !self.graph.lower_direct_class_parameter(class)? {
                return Err(Error::Missing("indirect class parameter ownership"));
            }
            if !self.graph.lower_class_empty(class)? {
                let address = self.emit(I::Addr(slot), ty)?.value;
                self.copy_object(ty, incoming, address)?;
            }
        } else {
            let storage = types::storage(self.graph, ty)?;
            // Prologue writes an unqualified frame place. Source accesses
            // retain the parameter object's volatile fact.
            self.builder.effect(I::Store(
                crate::lowir::model::Facts::memory(
                    false,
                    !matches!(storage, Type::I8 | Type::U8 | Type::Object { .. }),
                ),
                storage,
                incoming,
                slot,
            ))?;
        }
        Ok(())
    }
    pub(super) fn object_pointer(&mut self, source: TypeId) -> Result<RValue, Error> {
        let value = self.this_pointer(source)?;
        if let Some(member) = self.closure_this {
            let place = self.closure_field(value.value, member, source)?;
            return self.emit(
                I::Load(
                    crate::lowir::model::Facts::memory(false, true),
                    Type::Ptr,
                    place,
                ),
                source,
            );
        }
        Ok(value)
    }
    pub(super) fn this_pointer(&mut self, source: TypeId) -> Result<RValue, Error> {
        let slot = self
            .this_slot
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "implicit object without this slot",
            )))?;
        let value = self.emit(
            I::Load(
                crate::lowir::model::Facts::memory(false, true),
                Type::Ptr,
                slot,
            ),
            source,
        )?;
        if self.virtual_context.pending || self.virtual_context.vtt.is_some() {
            self.virtual_context.this_values.insert(value.value);
        }
        Ok(value)
    }

    pub(super) fn check_stack(&self) -> Result<(), Error> {
        let marker = 0u8;
        if self
            .budget
            .exceeded(self.stack_base, std::ptr::from_ref(&marker).addr())
        {
            return Err(Error::Semantic(crate::sema::Error::Limit(
                "constructs nested too deeply",
            )));
        }
        Ok(())
    }
    pub(super) fn emit(&mut self, kind: I, source: TypeId) -> Result<RValue, Error> {
        self.ensure_insertion()?;
        let ty = kind.result_type();
        let value = self.builder.generated(kind)?;
        Ok(RValue {
            value,
            ty,
            source,
            literal: false,
            bits: None,
        })
    }
    pub(super) fn ensure_insertion(&mut self) -> Result<(), Error> {
        if self.terminated {
            let block = self.new_block("unreachable")?;
            self.place(block)?;
        }
        Ok(())
    }
    pub(super) fn effect(&mut self, kind: I) -> Result<(), Error> {
        self.effect_with_return_guard(kind, true)
    }
    pub(super) fn effect_with_return_guard(
        &mut self,
        kind: I,
        end_guard: bool,
    ) -> Result<(), Error> {
        if matches!(kind, I::Branch(..) | I::Jump(_))
            && self.lifetime.flow_region == super::lifetime::FlowRegion::Local
        {
            self.pause_call_region()?;
        }
        if kind.terminator()
            && (self.lifetime.full_expression
                || self.lifetime.construction_flow
                    == super::construction_flow::ConstructionFlow::Branched)
            && self.lifetime.flow_region == super::lifetime::FlowRegion::Local
            && self.handlers.explicit == 0
        {
            self.close_lifetime_region()?;
        }
        if matches!(kind, I::Return(..)) && !self.construction.placing {
            self.end_constructor_region()?;
        }
        if matches!(kind, I::Return(..)) && end_guard && self.lifetime.function_guard.is_some() {
            self.builder
                .effect(I::Runtime(crate::lowir::model::RuntimeInstruction::EhEnd))?;
            self.note_element_marker()?;
        }
        self.ensure_insertion()?;
        let marker = matches!(
            kind,
            I::Runtime(
                crate::lowir::model::RuntimeInstruction::EhTry(_)
                    | crate::lowir::model::RuntimeInstruction::EhEnd
                    | crate::lowir::model::RuntimeInstruction::EhCleanup(_)
            )
        );
        self.builder.effect(kind)?;
        if marker {
            self.note_element_marker()?;
        }
        Ok(())
    }
    pub(super) fn zero(&mut self, source: TypeId) -> Result<RValue, Error> {
        if super::complex::part(self.graph, source)?.is_some()
            || matches!(self.graph.lower_type(source)?, TypeView::Vector { .. })
        {
            let ty = types::storage(self.graph, source)?;
            let (bytes, align) = types::layout(self.graph, source)?;
            let slot = self.generated_slot("complex", ty)?;
            self.effect(I::ZeroInit(bytes, align, slot))?;
            return self.read_place(slot, source);
        }
        if let Some(kind) = super::soft_float::kind(self.graph, source)? {
            return self.soft_constant(kind, 0, source);
        }
        let ty = types::storage(self.graph, source)?;
        let bits = if self.data_member_pointer(source) {
            u64::MAX
        } else {
            0
        };
        let value = if let Some(kind) = crate::lowir::literals::FloatKind::from_type(ty) {
            self.builder
                .constant(crate::lowir::literals::Constant::Float(kind, 0))?
        } else {
            self.builder
                .integer(if bits == u64::MAX { u128::MAX } else { 0 })?
        };
        Ok(RValue {
            value,
            ty,
            source,
            literal: false,
            bits: Some(bits),
        })
    }
    pub(super) fn block_name(&mut self, prefix: &str) -> Result<String, Error> {
        self.next_block = self
            .next_block
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        Ok(format!("{prefix}_{}", self.next_block))
    }
    pub(super) fn declaration(&mut self, id: ast::DeclId) -> Result<(), Error> {
        if let Some(entity) = self.graph.lower_declaration_storage(id) {
            self.local_entity(entity, None, None)?;
        }
        let declarators = match self.tree.declarations.get(id).kind {
            DeclKind::Simple { declarators, .. } => declarators,
            DeclKind::Empty
            | DeclKind::Alias { .. }
            | DeclKind::NamespaceAlias { .. }
            | DeclKind::Using { .. }
            | DeclKind::Enum { .. }
            | DeclKind::StaticAssert { .. }
            | DeclKind::Class(_) => return Ok(()),
            _ => return Err(Error::Missing("block declaration lowering")),
        };
        for declarator in self.tree.init_declarators.get(declarators) {
            self.local_variable(
                declarator.declarator,
                declarator.initializer,
                Some(declarator.tokens),
            )?;
        }
        Ok(())
    }
    pub(super) fn local_variable(
        &mut self,
        declarator: ast::DeclaratorId,
        initializer: Option<ast::Initializer>,
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<(), Error> {
        let entity = self.graph.lower_declared(declarator)?;
        if self.graph.lower_binding_alias(entity).is_some() {
            return Ok(());
        }
        if self.graph.lower_decomposition(entity).is_some() && initializer.is_none() {
            return Ok(());
        }
        self.local_entity(entity, initializer, tokens)
    }
    pub(super) fn begin_local_initializer(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        object: bool,
        temporaries: bool,
    ) -> Result<bool, Error> {
        let expression = self.lifetime.full_expression;
        let began = if !expression
            && self.constructor_expression(entity, initializer, temporaries)?
        {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment =
                super::lifetime::TemporarySegment::new(temporaries, false);
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
            true
        } else if object && !temporaries {
            false
        } else if let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id),
            ..
        }) = initializer
        {
            self.begin_full_expression(
                Some(id),
                super::temporary_scan::ExpressionUse::Initialization,
            )?
        } else if temporaries
            && self.initializer_throws(initializer)?
            && !self.lifetime.full_expression
        {
            self.lifetime.full_expression = true;
            self.lifetime.region_start = super::lifetime::RegionStart::Pending;
            self.lifetime.temporary_segment =
                super::lifetime::TemporarySegment::new(temporaries, false);
            self.ensure_insertion()?;
            self.lifetime.expression_start = Some(self.builder.position()?);
            true
        } else if temporaries && let Some(initializer) = initializer {
            // Parenthesized initializer arguments form one query. A
            // nonthrowing root result initializes the destination rather
            // than opening a segment; nested call results still do.
            self.begin_subobject_initializer(initializer, self.graph.lower_construction(entity))?
        } else {
            self.begin_lifetime_expression()
        };
        if began && let Some(initializer) = initializer {
            match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    self.begin_tracking(id, super::temporary_scan::ExpressionUse::Value)?;
                }
                ast::InitializerKind::Parenthesized(arguments)
                    if self.tree.expr_lists.get(arguments).len() == 1 =>
                {
                    for &id in self.tree.expr_lists.get(arguments) {
                        self.begin_tracking(
                            id,
                            super::temporary_scan::ExpressionUse::Initialization,
                        )?;
                    }
                }
                _ => (),
            }
        }
        Ok(began)
    }
    fn local_entity(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<(), Error> {
        if !self.graph.lower_automatic(entity) {
            return self.initialize_local_variable(entity, initializer, tokens);
        }
        let reachable = !self.terminated;
        let ty = self.local_object_type(entity)?;
        let object = matches!(
            self.graph.lower_type(ty)?,
            TypeView::Class(_) | TypeView::Array { .. }
        );
        let expression = self.lifetime.full_expression;
        let temporary_floor = self.lifetime.cleanups.len();
        // The selected local constructor owns the destination transfer. Its
        // initializer query counts only the source's evaluation temporaries.
        let temporaries = if object {
            self.local_initializer_temporaries(initializer, ty)?
        } else {
            self.initializer_temporaries(initializer)?
        };
        let began = self.begin_local_initializer(entity, initializer, object, temporaries)?;
        let construction = self.lifetime.construction;
        let construction_flow = self.lifetime.construction_flow;
        let construction_start = self.lifetime.construction_start;
        if object && !began && !self.lifetime.full_expression {
            self.lifetime.construction_flow = self.initializer_branches(initializer)?;
            self.ensure_insertion()?;
            self.lifetime.construction_start =
                Some((self.builder.position()?, self.lifetime.cleanups.len()));
        }
        self.lifetime.construction |= object;
        let saved_owner = self.reference_lifetimes.aggregate;
        if object && self.graph.lower_automatic(entity) {
            self.reference_lifetimes.aggregate = Some(entity);
        }
        let root = initializer.and_then(|initializer| match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => Some(id),
            ast::InitializerKind::Parenthesized(args) => {
                let arguments = self.tree.expr_lists.get(args);
                (arguments.len() == 1).then(|| arguments[0])
            }
            _ => None,
        });
        let result = self.with_full_root(root, |this| {
            this.initialize_local_variable(entity, initializer, tokens)
        });
        self.reference_lifetimes.aggregate = saved_owner;
        result?;
        // A conditional's private result remains alive through the outer copy,
        // including a nonthrowing copy. Its final destruction shares that region.
        if object
            && self.lifetime.full_expression
            && !self.lifetime.region
            && !self.lifetime.tracking.active
            && self.lifetime.cleanups.len() > temporary_floor
            && let Some(initializer) = initializer
        {
            let source = self.initializer_expression(initializer)?;
            let root = self.without_parentheses(source);
            if matches!(
                self.tree.expressions.get(root).kind,
                ast::ExprKind::Conditional { .. }
            ) && !self.graph.lower_expression(root)?.is_place
            {
                self.open_storage_completion_region(temporary_floor)?;
            }
        }
        self.destroy_local_temporaries(temporary_floor, entity)?;
        if object || !expression {
            self.close_lifetime_region()?;
        }
        self.lifetime.construction = construction;
        self.lifetime.construction_flow = construction_flow;
        self.lifetime.construction_start = construction_start;
        self.end_lifetime_expression(began)?;
        self.lifetime.full_expression = expression;
        if reachable && self.graph.lower_is_variable(entity) && self.graph.lower_automatic(entity) {
            self.register_local_destruction(entity)?;
        }
        Ok(())
    }
    fn initialize_local_variable(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<(), Error> {
        if !self.graph.lower_is_variable(entity) {
            return Ok(());
        }
        if !self.graph.lower_automatic(entity) {
            if self.graph.lower_program_storage(entity) {
                return Ok(());
            }
            return self.local_static(entity, initializer, tokens);
        }
        let ty = self.local_object_type(entity)?;
        let storage = types::local_storage(self.graph, entity, ty)?;
        let slot = if self.returned_local == Some(entity) {
            self.result_destination
                .ok_or(crate::sema::Error::Invariant(
                    "returned local without destination",
                ))?
        } else if let Some(name) = self.graph.lower_local_name(entity) {
            self.builder.slot(self.symbols.identifier(name)?, storage)?
        } else {
            self.generated_slot("__anonymous_union_storage", storage)?
        };
        self.locals.insert(entity, slot);
        if self.terminated {
            // unreachable_declaration retains the holder's identity but strips
            // automatic initialization, including a reference's binding.
            return Ok(());
        }
        if let TypeView::Reference { .. } = self.graph.lower_type(ty)? {
            let initializer = initializer.ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "reference without initializer",
            )))?;
            let expr = self.initializer_expression(initializer)?;
            let address = self.bind_extended_reference(expr, ty, entity);
            let address = address?;
            return self.effect(I::Store(
                crate::lowir::model::Facts::memory(false, true),
                Type::Ptr,
                address,
                slot,
            ));
        }
        if matches!(self.graph.lower_type(ty)?, TypeView::Array { .. }) {
            return self.local_array(entity, ty, slot, initializer);
        }
        if matches!(self.graph.lower_type(ty)?, TypeView::Class(_)) {
            return self.local_aggregate(entity, ty, slot, initializer);
        }
        if let Some(value) = self.graph.lower_variable(entity)?.initializer {
            let soft = super::soft_float::kind(self.graph, ty)?;
            let value = match value {
                crate::sema::lowering::StaticValue::Floating(_, _)
                    if soft.is_some_and(|kind| kind != super::soft_float::Kind::BFloat) =>
                {
                    None
                }
                crate::sema::lowering::StaticValue::Floating(_, bits) if soft.is_some() => Some(
                    self.soft_constant(
                        soft.ok_or(crate::sema::Error::Invariant("local soft kind"))?,
                        bits,
                        ty,
                    )?
                    .value,
                ),
                crate::sema::lowering::StaticValue::Integer(_)
                | crate::sema::lowering::StaticValue::WideInteger(_)
                | crate::sema::lowering::StaticValue::Floating(..) => {
                    Some(self.builder.constant(super::data::constant(value)?)?)
                }
                _ => None,
            };
            if let Some(value) = value {
                return self.write_place(slot, ty, value);
            }
        }
        self.initialize_scalar(ty, slot, initializer)
    }
    pub(super) fn initialize_scalar(
        &mut self,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        self.initialize_scalar_memory(ty, slot, initializer, types::memory(self.graph, ty)?)
    }
    pub(super) fn initialize_scalar_memory(
        &mut self,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
        memory: crate::lowir::model::Facts,
    ) -> Result<(), Error> {
        if let Some(initializer) = initializer {
            let value = match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    if matches!(self.tree.expressions.get(id).kind, ast::ExprKind::Braced { elements, .. } if self.tree.expr_lists.get(elements).is_empty())
                    {
                        self.zero(ty)?
                    } else {
                        self.operand(id, super::conversions::Context::Initialization)?
                    }
                }
                ast::InitializerKind::Parenthesized(args) => {
                    if let Some(&id) = self.tree.expr_lists.get(args).first() {
                        self.operand(id, super::conversions::Context::Initialization)?
                    } else {
                        self.zero(ty)?
                    }
                }
                _ => return Err(Error::Missing("special initializer lowering")),
            };
            let value = self.convert(value, ty, super::conversions::Context::Initialization)?;
            let value = self.fit(value, ty)?;
            if self.graph.lower_cv(ty) & 4 != 0 {
                return self.write_atomic_place(slot, ty, value.value);
            }
            self.effect(I::Store(
                memory,
                types::storage(self.graph, ty)?,
                value.value,
                slot,
            ))?;
        }
        Ok(())
    }
}

pub(super) fn signature(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    ty: TypeId,
    nothrow: bool,
) -> Result<crate::lowir::model::SignatureId, Error> {
    use crate::lowir::model::{Arity, Parameter, Signature};
    let TypeView::Function {
        result,
        parameters,
        variadic,
    } = graph.lower_type(ty)?
    else {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "indirect boundary without function type",
        )));
    };
    let mut values = Vec::with_capacity(parameters.len());
    let result_boundary = symbols.result_boundary(result)?;
    if result_boundary.indirect {
        values.push(Parameter {
            name: program.names.intern("arg0")?,
            ty: Type::Ptr,
            pass: Some(crate::lowir::vocabulary::Pass::IndirectResult),
            no_alias: false,
            object_bytes: types::layout(graph, result)?.0,
            classes: None,
        });
    }
    for (index, &ty) in parameters.iter().enumerate() {
        let (pass, object_bytes) = reference_boundary(graph, ty)?;
        values.push(Parameter {
            name: program.names.intern(&format!(
                "arg{}",
                index + usize::from(result_boundary.indirect)
            ))?,
            ty: parameter_type(graph, ty)?,
            pass,
            no_alias: false,
            object_bytes,
            classes: symbols.parameter_classes(ty)?,
        });
    }
    Ok(program.add_signature(Signature {
        parameters: values,
        result: result_boundary.ty,
        boundary: Boundary {
            result_classes: result_boundary.classes,
            unwind: if nothrow { Unwind::No } else { Unwind::Default },
            arity: if variadic {
                Arity::Variadic
            } else {
                Arity::Fixed
            },
            ..Boundary::default()
        },
    })?)
}

fn parameters_types(graph: &Graph, ty: TypeId) -> Result<&[TypeId], Error> {
    match graph.lower_type(ty)? {
        TypeView::Function { parameters, .. } => Ok(parameters),
        _ => Err(Error::Semantic(crate::sema::Error::Invariant(
            "function type missing",
        ))),
    }
}
pub(super) fn reference_boundary(
    graph: &Graph,
    ty: TypeId,
) -> Result<(Option<crate::lowir::vocabulary::Pass>, u64), Error> {
    if matches!(graph.lower_type(ty)?, TypeView::Class(class) if !graph.lower_direct_class_parameter(class)?)
    {
        return Ok((
            Some(crate::lowir::vocabulary::Pass::ByAddress),
            types::layout(graph, ty)?.0,
        ));
    }
    let TypeView::Reference { element, .. } = graph.lower_type(ty)? else {
        return Ok((None, 0));
    };
    let bytes = if graph.lower_complete_object(element)? {
        types::layout(graph, element)?.0
    } else {
        0
    };
    Ok((Some(crate::lowir::vocabulary::Pass::ByAddress), bytes))
}

fn parameter_type(graph: &Graph, ty: TypeId) -> Result<Type, Error> {
    if matches!(graph.lower_type(ty)?, TypeView::Class(class) if !graph.lower_direct_class_parameter(class)?)
    {
        Ok(Type::Ptr)
    } else {
        types::storage(graph, ty)
    }
}

fn parameter_no_alias(
    builtin: Option<crate::sema::lowering::BuiltinFunction>,
    index: usize,
) -> bool {
    matches!(
        builtin,
        Some(crate::sema::lowering::BuiltinFunction::Memcpy)
    ) && index < 2
}

pub(super) fn declared_effects(boundary: &mut Boundary, graph: &Graph, entity: Option<EntityId>) {
    use crate::sema::lowering::FunctionEffect;
    if entity.is_some_and(|entity| graph.lower_no_return(entity)) {
        boundary.returns = crate::lowir::model::Returns::NoReturn;
    }
    match entity.map_or(FunctionEffect::Default, |entity| {
        graph.lower_effects(entity)
    }) {
        FunctionEffect::Default => {}
        FunctionEffect::ReadOnly => {
            boundary.effects = Some(crate::lowir::vocabulary::Effects::ReadOnly);
        }
        FunctionEffect::ReadNone => {
            boundary.effects = Some(crate::lowir::vocabulary::Effects::ReadNone);
        }
    }
}

fn builtin_boundary(
    boundary: &mut Boundary,
    builtin: Option<crate::sema::lowering::BuiltinFunction>,
) {
    match builtin {
        Some(crate::sema::lowering::BuiltinFunction::Abort) => {
            boundary.unwind = Unwind::No;
            boundary.returns = crate::lowir::model::Returns::NoReturn;
        }
        Some(crate::sema::lowering::BuiltinFunction::Strlen) => {
            boundary.effects = Some(crate::lowir::vocabulary::Effects::ReadOnly);
        }
        _ => {}
    }
}

pub(super) fn declare(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    entry: super::construction::Entry,
) -> Result<(), Error> {
    let symbol = symbols.entry_symbol(program, entry)?;
    if program.symbol(symbol).declared() {
        return Ok(());
    }
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let signature = if let Some(signature) = program.forward_boundary(symbol) {
        signature
    } else {
        declaration_signature(program, symbols, graph, entry)?
    };
    let mut metadata = symbols.entry_metadata(program, entry, &name)?;
    // A declared ABI entry carries its function metadata, not the object-root
    // flag used to retain an emitted base or internal complete definition
    // (symbol_lowering.cpp::entry_symbol, program_lowering.cpp::emit_definition).
    if matches!(
        entry,
        super::construction::Entry::Constructor(_, _)
            | super::construction::Entry::DestructorBase(_)
    ) {
        metadata.flags &= !4;
    }
    let name = program.symbol(symbol).name;
    program.add(
        symbol,
        crate::lowir::model::Function {
            name,
            signature,
            metadata,
            body: None,
            location: None,
        },
    )?;
    Ok(())
}

pub(super) fn declaration_signature(
    program: &mut Program,
    symbols: &mut Symbols<'_, '_>,
    graph: &Graph,
    entry: super::construction::Entry,
) -> Result<crate::lowir::model::SignatureId, Error> {
    let facts = entry.facts(graph)?;
    let signature = signature(program, symbols, graph, facts.ty, facts.nothrow)?;
    if let Some(class) = facts.object {
        let first = usize::from(
            program
                .signature(signature)
                .parameters
                .first()
                .is_some_and(|p| p.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)),
        );
        let name = program.names.intern(&format!("arg{first}"))?;
        for index in first..program.signature_mut(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{}", index + 1))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
        let object_bytes = types::layout(graph, graph.lower_entity_type(class)?)?.0;
        program.signature_mut(signature).parameters.insert(
            first,
            crate::lowir::model::Parameter {
                name,
                ty: Type::Ptr,
                pass: None,
                object_bytes,
                no_alias: false,
                classes: None,
            },
        );
    }
    if entry.hidden_vtt(graph)? {
        let first = 1;
        let name = program.names.intern("arg1")?;
        program.signature_mut(signature).parameters.insert(
            first,
            crate::lowir::model::Parameter {
                name,
                ty: Type::Ptr,
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: None,
            },
        );
        for index in first + 1..program.signature(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{index}"))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
    }
    let boundary = program.signature_mut(signature);
    let distinct = entry
        .entity(graph)
        .is_some_and(|entity| graph.lower_copy_constructor(entity));
    for (index, parameter) in boundary.parameters.iter_mut().enumerate() {
        parameter.no_alias = distinct
            && (index == 0 || index == 1 + usize::from(entry.hidden_vtt(graph)?))
            || parameter_no_alias(facts.builtin, index);
    }
    builtin_boundary(&mut boundary.boundary, facts.builtin);
    declared_effects(&mut boundary.boundary, graph, entry.entity(graph));
    Ok(signature)
}
