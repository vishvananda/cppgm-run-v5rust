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
    pub entity: EntityId,
    pub next_static: u32,
    pub next_block: u32,
    pub jumps: Vec<super::statements::JumpTargets>,
    pub switch_blocks: HashMap<ast::StmtId, crate::lowir::model::BlockId>,
    pub label_memo: HashMap<ast::StmtId, bool>,
    pub labels: HashMap<crate::lex::identifiers::NameId, crate::lowir::model::BlockId>,
    pub folded_targets: Vec<crate::lowir::model::BlockId>,
    pub conditional_slot: Option<Value>,
    pub terminated: bool,
    pub result: TypeId,
    pub budget: StackBudget,
    pub stack_base: usize,
}
pub(super) fn lower<'a>(
    program: &mut Program,
    symbols: &mut Symbols<'a, '_>,
    graph: &'a Graph,
    tree: &'a ast::Tree,
    entity: EntityId,
    body: ast::BodyId,
    budget: StackBudget,
) -> Result<(), Error> {
    let facts = graph.lower_function(entity)?;
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
    let mut names = Vec::with_capacity(parameters.len());
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
    let symbol = symbols.function_symbol(program, entity)?;
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
    let metadata = symbols.metadata(program, entity, &name)?;
    let is_main = metadata.role == Some(crate::lowir::vocabulary::Role::Entry);
    let parameters: Vec<_> = names
        .iter()
        .map(|(name, ty)| (name.as_str(), *ty))
        .collect();
    let mut builder =
        FunctionBuilder::new(program, &name, &parameters, types::storage(graph, result)?)?;
    builder.metadata(metadata);
    for (index, &ty) in parameters_types(graph, facts.ty)?.iter().enumerate() {
        let (pass, bytes) = reference_boundary(graph, ty)?;
        builder.parameter_boundary(index, pass, bytes, parameter_no_alias(facts.builtin, index))?;
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
    let marker = 0u8;
    let mut function = Function {
        graph,
        tree,
        builder,
        symbols,
        locals: HashMap::new(),
        entity,
        next_static: 0,
        next_block: 0,
        jumps: Vec::new(),
        switch_blocks: HashMap::new(),
        label_memo: HashMap::new(),
        labels: HashMap::new(),
        folded_targets: Vec::new(),
        conditional_slot: None,
        terminated: false,
        result,
        budget,
        stack_base: std::ptr::from_ref(&marker).addr(),
    };
    for (index, &(entity, _, _)) in parameter_bindings.iter().enumerate() {
        let slot = if parameter_bindings[index].1.is_some() {
            function.builder.slot(&names[index].0, names[index].1)?
        } else {
            function.builder.parameter_slot(index, names[index].1)?
        };
        let incoming = function.builder.parameter(index)?;
        function.builder.effect(I::Store(
            // Prologue initialization uses an unqualified place. Subsequent
            // source accesses retain the parameter object's volatile fact.
            crate::lowir::model::Facts::memory(
                false,
                !matches!(names[index].1, Type::I8 | Type::U8),
            ),
            names[index].1,
            incoming,
            slot,
        ))?;
        function.locals.insert(entity, slot);
    }
    let ast::Body::Parsed(body) = tree.bodies.get(body) else {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "lowering deferred body",
        )));
    };
    if body.handlers.is_some() || !tree.mem_initializers.get(body.initializers).is_empty() {
        return Err(Error::Missing("exception and constructor lowering"));
    }
    function.statement(body.compound)?;
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
            function.builder.effect(I::Return(ty, value))?;
        }
    }
    function.builder.finish()?;
    Ok(())
}
impl Function<'_, '_, '_, '_> {
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
        let DeclKind::Simple { declarators, .. } = self.tree.declarations.get(id).kind else {
            return Err(Error::Missing("block declaration lowering"));
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
        let entity = self.graph.lower_declared(declarator)?;
        if !self.graph.lower_is_variable(entity) {
            return Ok(());
        }
        if !self.graph.lower_automatic(entity) {
            if self.graph.lower_namespace_storage(entity) {
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
            .slot(spelling, types::storage(self.graph, ty)?)?;
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
            let address =
                self.bind_reference(expr, ty, super::conversions::Context::Initialization)?;
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
            classes: None,
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
    let bytes = match graph.lower_type(element)? {
        TypeView::Function { .. } | TypeView::Array { bound: 0, .. } | TypeView::Unsupported => 0,
        _ => types::layout(graph, element)?.0,
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
    entity: EntityId,
) -> Result<(), Error> {
    let symbol = symbols.function_symbol(program, entity)?;
    if program.symbol(symbol).declared() {
        return Ok(());
    }
    let facts = graph.lower_function(entity)?;
    let name = program.names.get(program.symbol(symbol).name).to_owned();
    let signature = signature(program, graph, facts.ty, facts.nothrow)?;
    let boundary = program.signature_mut(signature);
    for (index, parameter) in boundary.parameters.iter_mut().enumerate() {
        parameter.no_alias = parameter_no_alias(facts.builtin, index);
    }
    builtin_boundary(&mut boundary.boundary, facts.builtin);
    let metadata = symbols.metadata(program, entity, &name)?;
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
