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
    pub bit_initialization: super::bit_fields::Initialization,
    pub named_path: Vec<super::aggregates::Step>,
    pub aggregation: super::aggregate_cleanup::State,
    pub entity: Option<EntityId>,
    pub this_slot: Option<Value>,
    pub persistent_references: bool,
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
    pub destruction: super::destruction::Destruction,
    pub construction: super::constructor_cleanup::Construction,
    pub terminated: bool,
    pub result: Option<TypeId>,
    pub budget: StackBudget,
    pub stack_base: usize,
}
pub(super) fn lower<'a>(
    program: &mut Program,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    entry: super::construction::Entry,
    body: Option<ast::BodyId>,
    budget: StackBudget,
) -> Result<(), Error> {
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
            reserved.insert(symbols.identifier(name)?.to_owned());
        }
    }
    let object_count = usize::from(facts.object.is_some());
    let mut names = Vec::with_capacity(parameters.len() + object_count);
    if object_count != 0 {
        names.push(("this".to_owned(), Type::Ptr));
    }
    for &(_, name, ty) in &parameter_bindings {
        names.push((
            if let Some(name) = name {
                symbols.identifier(name)?.to_owned()
            } else {
                let mut name = format!("__param{}", names.len());
                while reserved.contains(&name) {
                    name.push('_');
                }
                name
            },
            types::storage(graph, ty)?,
        ));
    }
    let symbol = symbols.entry_symbol(program, entry)?;
    if program.function_defined(symbol) {
        let declaration =
            facts
                .definition
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "function definition without source",
                )))?;
        return Err(Error::Semantic(crate::sema::Error::Diagnostic {
            location: tree.declarations.get(declaration).location,
            message: "redefinition of a function across translation units",
        }));
    }
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let mut metadata = symbols.entry_metadata(program, entry, &name)?;
    if matches!(
        facts.name,
        crate::sema::lowering::FunctionName::Destructor(_)
    ) && facts.internal
    {
        metadata.flags |= 4;
    }
    let is_main = metadata.role == Some(crate::lowir::vocabulary::Role::Entry);
    let parameters: Vec<_> = names
        .iter()
        .map(|(name, ty)| (name.as_str(), *ty))
        .collect();
    let mut builder =
        FunctionBuilder::new(program, &name, &parameters, types::storage(graph, result)?)?;
    builder.metadata(metadata);
    if let Some(class) = facts.object {
        builder.parameter_boundary(
            0,
            None,
            types::layout(graph, graph.lower_entity_type(class)?)?.0,
            false,
        )?;
    }
    for (index, &ty) in parameters_types(graph, facts.ty)?.iter().enumerate() {
        let (pass, bytes) = reference_boundary(graph, ty)?;
        builder.parameter_classes(index + object_count, symbols.parameter_classes(ty)?)?;
        builder.parameter_boundary(
            index + object_count,
            pass,
            bytes,
            parameter_no_alias(facts.builtin, index),
        )?;
    }
    let mut boundary = Boundary {
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
    builder.boundary(boundary);
    builder.block("entry")?;
    let mut function = Function::new(graph, tree, builder, symbols, entity, Some(result), budget);
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
    if facts.nothrow {
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
            function.open_function_guard()?;
        }
    }
    if object_count != 0 {
        let slot = function.builder.slot("this", Type::Ptr)?;
        let incoming = function.builder.parameter(0)?;
        function.builder.effect(I::Store(
            crate::lowir::model::Facts::memory(false, true),
            Type::Ptr,
            incoming,
            slot,
        ))?;
        function.this_slot = Some(slot);
    }
    for (ordinal, &(entity, _, _)) in parameter_bindings.iter().enumerate() {
        let index = ordinal + object_count;
        let slot = if parameter_bindings[ordinal].1.is_some() {
            function.builder.slot(&names[index].0, names[index].1)?
        } else {
            function.builder.parameter_slot(index, names[index].1)?
        };
        let incoming = function.builder.parameter(index)?;
        function.initialize_parameter(parameter_bindings[ordinal].2, incoming, slot)?;
        function.locals.insert(entity, slot);
    }
    if let super::construction::Entry::Constructor(id, variant) = entry {
        let location = body.map_or(crate::support::source::SourceOffset(0), |body| {
            tree.statements.get(body.compound).location
        });
        function.initialize_constructor(id, variant, location)?;
    }
    let destructor = entity.filter(|_| {
        matches!(
            facts.name,
            crate::sema::lowering::FunctionName::Destructor(_)
        )
    });
    if let Some(entity) = destructor {
        function.begin_destructor(entity)?;
    }
    if let Some(body) = body {
        if body.handlers.is_some() {
            return Err(Error::Missing("exception lowering"));
        }
        function.statement(body.compound)?;
    }
    function.finish_destructor()?;
    if !function.terminated {
        let ty = types::storage(graph, result)?;
        if ty != Type::Void
            && !is_main
            && function
                .builder
                .insertion_targeted(&function.folded_targets)?
        {
            function.builder.effect(I::Unreachable)?;
        } else {
            let value = if ty == Type::Void {
                None
            } else {
                Some(function.zero(result)?.value)
            };
            function.effect(I::Return(ty, value))?;
        }
    }
    function.finish_constructor_cleanups()?;
    function.place_cleanup_chains(true)?;
    function.place_destructor_terminates()?;
    function.place_function_guard()?;
    function.builder.finish()?;
    symbols.finish_entry(program, entry, symbol)?;
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
            bit_initialization: super::bit_fields::Initialization::default(),
            named_path: Vec::new(),
            aggregation: super::aggregate_cleanup::State::default(),
            entity,
            this_slot: None,
            persistent_references: false,
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
            destruction: super::destruction::Destruction::default(),
            construction: super::constructor_cleanup::Construction::default(),
            terminated: false,
            result,
            budget,
            stack_base: std::ptr::from_ref(&marker).addr(),
        }
    }
}
impl Function<'_, '_, '_, '_> {
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
                let (bytes, align) = types::layout(self.graph, ty)?;
                self.effect(I::CopyObject(bytes, align, incoming, address))?;
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
        let slot = self
            .this_slot
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "implicit object without this slot",
            )))?;
        self.emit(
            I::Load(
                crate::lowir::model::Facts::memory(false, true),
                Type::Ptr,
                slot,
            ),
            source,
        )
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
        if kind.terminator() && self.lifetime.full_expression {
            self.close_lifetime_region()?;
        }
        if matches!(kind, I::Return(..)) && !self.construction.placing {
            self.end_constructor_region()?;
        }
        if matches!(kind, I::Return(..)) && self.lifetime.function_guard.is_some() {
            self.builder
                .effect(I::Runtime(crate::lowir::model::RuntimeInstruction::EhEnd))?;
        }
        self.ensure_insertion()?;
        Ok(self.builder.effect(kind)?)
    }
    pub(super) fn zero(&mut self, source: TypeId) -> Result<RValue, Error> {
        let ty = types::storage(self.graph, source)?;
        let value = if let Some(kind) = crate::lowir::literals::FloatKind::from_type(ty) {
            self.builder
                .constant(crate::lowir::literals::Constant::Float(kind, 0))?
        } else {
            self.builder.integer(0)?
        };
        Ok(RValue {
            value,
            ty,
            source,
            literal: false,
            bits: Some(0),
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
            self.local_variable(declarator.declarator, declarator.initializer)?;
        }
        Ok(())
    }
    pub(super) fn local_variable(
        &mut self,
        declarator: ast::DeclaratorId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let reachable = !self.terminated;
        let entity = self.graph.lower_declared(declarator)?;
        let ty = self.graph.lower_entity_type(entity)?;
        let object = matches!(
            self.graph.lower_type(ty)?,
            TypeView::Class(_) | TypeView::Array { .. }
        );
        let expression = self.lifetime.full_expression;
        let began = if object {
            false
        } else {
            self.begin_lifetime_expression()
        };
        let construction = self.lifetime.construction;
        self.lifetime.construction |= object;
        let saved_owner = self.reference_lifetimes.aggregate;
        if object && self.graph.lower_automatic(entity) {
            self.reference_lifetimes.aggregate = Some(entity);
        }
        let result = self.initialize_local_variable(declarator, initializer);
        self.reference_lifetimes.aggregate = saved_owner;
        result?;
        if object || !expression {
            self.close_lifetime_region()?;
        }
        self.lifetime.construction = construction;
        self.end_lifetime_expression(began)?;
        self.lifetime.full_expression = expression;
        if reachable && self.graph.lower_is_variable(entity) && self.graph.lower_automatic(entity) {
            self.register_local_destruction(entity)?;
        }
        Ok(())
    }
    fn initialize_local_variable(
        &mut self,
        declarator: ast::DeclaratorId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let entity = self.graph.lower_declared(declarator)?;
        if !self.graph.lower_is_variable(entity) {
            return Ok(());
        }
        if !self.graph.lower_automatic(entity) {
            if self.graph.lower_program_storage(entity) {
                return Ok(());
            }
            return self.local_static(entity, initializer);
        }
        let ty = self.graph.lower_entity_type(entity)?;
        let name = self.graph.lower_local_name(entity).ok_or(Error::Semantic(
            crate::sema::Error::Invariant("local without name"),
        ))?;
        let spelling = self.symbols.identifier(name)?;
        let slot = self
            .builder
            .slot(spelling, types::local_storage(self.graph, entity, ty)?)?;
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
            let address = self.bind_extended_reference(expr, ty, entity)?;
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
        self.initialize_scalar(ty, slot, initializer)
    }
    pub(super) fn initialize_scalar(
        &mut self,
        ty: TypeId,
        slot: Value,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        if let Some(initializer) = initializer {
            let value = match initializer.kind {
                ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => {
                    self.operand(id, super::conversions::Context::Initialization)?
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
            self.effect(I::Store(
                types::memory(self.graph, ty)?,
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
    for (index, &ty) in parameters.iter().enumerate() {
        let (pass, object_bytes) = reference_boundary(graph, ty)?;
        values.push(Parameter {
            name: program.names.intern(&format!("arg{index}"))?,
            ty: types::storage(graph, ty)?,
            pass,
            no_alias: false,
            object_bytes,
            classes: symbols.parameter_classes(ty)?,
        });
    }
    Ok(program.add_signature(Signature {
        parameters: values,
        result: types::storage(graph, result)?,
        boundary: Boundary {
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

fn parameter_no_alias(
    builtin: Option<crate::sema::lowering::BuiltinFunction>,
    index: usize,
) -> bool {
    matches!(
        builtin,
        Some(crate::sema::lowering::BuiltinFunction::Memcpy)
    ) && index < 2
}

fn builtin_boundary(
    boundary: &mut Boundary,
    builtin: Option<crate::sema::lowering::BuiltinFunction>,
) {
    if matches!(
        builtin,
        Some(crate::sema::lowering::BuiltinFunction::Strlen)
    ) {
        boundary.effects = Some(crate::lowir::vocabulary::Effects::ReadOnly);
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
    let facts = entry.facts(graph)?;
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let signature = signature(program, symbols, graph, facts.ty, facts.nothrow)?;
    if let Some(class) = facts.object {
        let name = program.names.intern("arg0")?;
        for index in 0..program.signature_mut(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{}", index + 1))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
        let object_bytes = types::layout(graph, graph.lower_entity_type(class)?)?.0;
        program.signature_mut(signature).parameters.insert(
            0,
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
    let boundary = program.signature_mut(signature);
    for (index, parameter) in boundary.parameters.iter_mut().enumerate() {
        parameter.no_alias = parameter_no_alias(facts.builtin, index);
    }
    builtin_boundary(&mut boundary.boundary, facts.builtin);
    let mut metadata = symbols.entry_metadata(program, entry, &name)?;
    // A declared ABI entry carries its function metadata, not the object-root
    // flag used to retain an emitted base definition (symbol_lowering.cpp).
    if matches!(
        entry,
        super::construction::Entry::Constructor(_, super::construction::Variant::Base)
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
