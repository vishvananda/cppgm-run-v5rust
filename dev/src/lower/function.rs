// Translated from lower/function_lowering.cpp and statement_lowering.cpp. See NOTICE.
//! Each body owns local places, temporary numbering, and byte-budget recursion guards.
use super::{Error, symbols::Symbols, types};
use crate::{
    lowir::{
        builder::FunctionBuilder,
        model::{Boundary, InstructionKind as I, Program, Unwind, Value},
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
    pub next_block: u32,
    pub jumps: Vec<super::statements::JumpTargets>,
    pub switch_blocks: HashMap<ast::StmtId, crate::lowir::model::BlockId>,
    pub label_memo: HashMap<ast::StmtId, bool>,
    pub next_slot: u32,
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
    if matches!(graph.lower_type(result)?, TypeView::Reference) {
        return Err(Error::Missing("reference return lowering"));
    }
    if variadic {
        return Err(Error::Missing("variadic definition lowering"));
    }
    let parameter_bindings: Vec<_> = graph.lower_parameters(facts.scope).collect();
    if parameters.len() != parameter_bindings.len() {
        return Err(Error::Semantic(crate::sema::Error::Invariant(
            "function parameter binding mismatch",
        )));
    }
    let mut names = Vec::with_capacity(parameters.len());
    for &(_, name, ty) in &parameter_bindings {
        if matches!(graph.lower_type(ty)?, TypeView::Reference) {
            return Err(Error::Missing("reference parameter lowering"));
        }
        names.push((
            if let Some(name) = name {
                symbols.identifier(name)?.to_owned()
            } else {
                return Err(Error::Missing("unnamed parameter lowering"));
            },
            types::storage(graph, ty)?,
        ));
    }
    let symbol = symbols.function_symbol(program, entity)?;
    if program.symbol(symbol).declared() {
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
    let parameters: Vec<_> = names
        .iter()
        .map(|(name, ty)| (name.as_str(), *ty))
        .collect();
    let mut builder =
        FunctionBuilder::new(program, &name, &parameters, types::storage(graph, result)?)?;
    builder.metadata(metadata);
    builder.boundary(Boundary {
        unwind: if facts.nothrow {
            Unwind::No
        } else {
            Unwind::Default
        },
        ..Boundary::default()
    });
    builder.block("entry")?;
    let marker = 0u8;
    let mut function = Function {
        graph,
        tree,
        builder,
        symbols,
        locals: HashMap::new(),
        next_block: 0,
        jumps: Vec::new(),
        switch_blocks: HashMap::new(),
        label_memo: HashMap::new(),
        next_slot: 0,
        terminated: false,
        result,
        budget,
        stack_base: std::ptr::from_ref(&marker).addr(),
    };
    for (index, &(entity, _, _)) in parameter_bindings.iter().enumerate() {
        let slot = function.builder.slot(&names[index].0, names[index].1)?;
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
        let value = if ty == Type::Void {
            None
        } else {
            Some(function.zero(result)?.value)
        };
        function.builder.effect(I::Return(ty, value))?;
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
            return Err(Error::Missing("local static storage lowering"));
        }
        let ty = self.graph.lower_entity_type(entity)?;
        if matches!(self.graph.lower_type(ty)?, TypeView::Reference) {
            return Err(Error::Missing("local reference lowering"));
        }
        let name = self.graph.lower_local_name(entity).ok_or(Error::Semantic(
            crate::sema::Error::Invariant("local without name"),
        ))?;
        let spelling = self.symbols.identifier(name)?;
        let slot = self
            .builder
            .slot(spelling, types::storage(self.graph, ty)?)?;
        self.locals.insert(entity, slot);
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
    if matches!(graph.lower_type(result)?, TypeView::Reference) {
        return Err(Error::Missing("reference call result boundary lowering"));
    }
    let mut values = Vec::with_capacity(parameters.len());
    for (index, &ty) in parameters.iter().enumerate() {
        if matches!(graph.lower_type(ty)?, TypeView::Reference) {
            return Err(Error::Missing("reference call boundary lowering"));
        }
        values.push(Parameter {
            name: program.names.intern(&format!("arg{index}"))?,
            ty: types::storage(graph, ty)?,
            pass: None,
            no_alias: false,
            object_bytes: 0,
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
