// Translated from sema/constant_construction.cpp. See NOTICE.
//! Materialization and construction consume analyzed initialization and constructor plans.
use super::{
    Error,
    analyzer::Analyzer,
    call_arguments::Arguments,
    constant_addresses::{Address, nonconstant},
    constexpr_frames::Frame,
    construction::ConstructorId,
    constructor_initialization::ConstructorArguments,
    expression_facts::{ClassInitialization, ValueCategory},
    graph::ScopeId,
    types::{Scalar, TypeKind},
};
use crate::{
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind, InitializerKind},
};

impl Analyzer<'_> {
    pub(super) fn materialize_constant_object(
        &mut self,
        id: ast::ExprId,
        mut target: Address,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let id = self.graph.expression_facts.root(id);
        let facts = self.graph.expression_facts.get(id)?;
        let location = self.tree.expressions.get(id).location;
        self.constant_step(location)?;
        target.designated = self.bare_type(target.designated)?;
        if facts.category != ValueCategory::Prvalue {
            let mut source = self.constant_place(id, scope)?;
            source.designated = self.bare_type(source.designated)?;
            return self.constexpr_frames.copy(target, source, location);
        }
        let initializer = match facts.class_initialization {
            Some(ClassInitialization::Aggregate(elements)) => {
                let arguments = Arguments::new(self.tree.expr_lists.get(elements));
                return self.constant_aggregate(
                    target,
                    arguments.as_slice(),
                    &mut 0,
                    scope,
                    location,
                );
            }
            Some(ClassInitialization::FromExpression(expr)) => {
                return self.initialize_constant_class_from(expr, target, scope);
            }
            Some(ClassInitialization::Constructed(arguments)) => {
                let args = self.constant_constructor_arguments(arguments);
                return self.constant_class_initialize(
                    target,
                    facts.constructor,
                    args.as_slice(),
                    args.as_slice().is_empty(),
                    scope,
                    location,
                );
            }
            None => match self.tree.expressions.get(id).kind {
                ExprKind::FunctionalCast {
                    arguments, braced, ..
                } => braced.map_or(
                    InitializerKind::Parenthesized(arguments),
                    InitializerKind::Braced,
                ),
                ExprKind::Braced { .. } => InitializerKind::Braced(id),
                ExprKind::Cast { operand, .. } => {
                    return self.constant_class_initialize(
                        target,
                        facts.constructor,
                        &[operand],
                        false,
                        scope,
                        location,
                    );
                }
                ExprKind::Call { arguments, .. } if facts.functional_call => {
                    InitializerKind::Parenthesized(arguments)
                }
                ExprKind::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    let selected = if self.constant_truth(condition, scope, false)? {
                        then_expr
                    } else {
                        else_expr
                    };
                    return self.materialize_constant_object(selected, target, scope);
                }
                ExprKind::Binary {
                    op, left, right, ..
                } if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Comma) => {
                    self.discard_constant(left, scope)?;
                    return self.materialize_constant_object(right, target, scope);
                }
                _ => return Err(Error::Missing("constexpr class result materialization")),
            },
        };
        self.construct_constant_object(
            target,
            Some(ast::Initializer {
                kind: initializer,
                location,
            }),
            facts.constructor,
            scope,
            location,
        )
    }
    fn initialize_constant_class_from(
        &mut self,
        expr: ast::ExprId,
        target: Address,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if let Some(super::copy_initialization::CopyAction::Constructor {
            constructor,
            elision: false,
            ..
        }) = self.graph.lower_class_copy(expr)?
        {
            return self.run_constant_constructor(
                target,
                constructor,
                &[expr],
                scope,
                self.tree.expressions.get(expr).location,
            );
        }
        self.materialize_constant_object(expr, target, scope)
    }
    fn constant_constructor_arguments(&self, arguments: ConstructorArguments) -> Arguments {
        match arguments {
            ConstructorArguments::Values(values) => {
                Arguments::new(self.tree.expr_lists.get(values))
            }
            ConstructorArguments::Single(value) => Arguments::new(&[value]),
            ConstructorArguments::Value => Arguments::Empty,
        }
    }
    pub(super) fn construct_constant_object(
        &mut self,
        target: Address,
        initializer: Option<ast::Initializer>,
        constructor: Option<ConstructorId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        self.constant_step(location)?;
        let Some(initializer) = initializer else {
            if let Some(constructor) = constructor
                && self.graph.constructors.get(constructor).nontrivial()
            {
                return self.run_constant_constructor(target, constructor, &[], scope, location);
            }
            return Ok(());
        };
        let values = match initializer.kind {
            InitializerKind::Assign(expr) | InitializerKind::Braced(expr) => {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind {
                    let values = Arguments::new(self.tree.expr_lists.get(elements));
                    if matches!(
                        self.graph.types.get(target.designated).kind,
                        TypeKind::Array { .. }
                    ) || self
                        .graph
                        .class_of(target.designated)
                        .is_some_and(|class| self.graph.aggregate_class(class))
                    {
                        return self.constant_aggregate(
                            target,
                            values.as_slice(),
                            &mut 0,
                            scope,
                            location,
                        );
                    }
                    values
                } else if self.graph.class_of(target.designated).is_some() && constructor.is_none()
                {
                    return self.initialize_constant_class_from(expr, target, scope);
                } else {
                    Arguments::new(&[expr])
                }
            }
            InitializerKind::Parenthesized(values) => {
                Arguments::new(self.tree.expr_lists.get(values))
            }
            _ => {
                return Err(nonconstant(
                    location,
                    "an initializer outside constant evaluation",
                ));
            }
        };
        let values = values.as_slice();
        if self.graph.class_of(target.designated).is_some() {
            return self.constant_class_initialize(
                target,
                constructor,
                values,
                values.is_empty(),
                scope,
                location,
            );
        }
        if values.len() > 1 {
            return Err(nonconstant(
                location,
                "a scalar initialized from several values",
            ));
        }
        if let TypeKind::Reference { element, .. } = self.graph.types.get(target.designated).kind {
            let Some(&expr) = values.first() else {
                return Err(nonconstant(location, "a reference without an initializer"));
            };
            let place = self.bind_constant_reference(expr, element, scope)?;
            return self.constexpr_frames.write(
                target,
                super::constant_addresses::TemporaryValue::Pointer(place),
                location,
            );
        }
        if let Some(&expr) = values.first() {
            let value = self.constant_operand_value(expr, target.designated, scope)?;
            self.store_constant_value(target, value, location)
        } else {
            self.constant_zero_object(target, location)
        }
    }
    fn constant_class_initialize(
        &mut self,
        target: Address,
        constructor: Option<ConstructorId>,
        values: &[ast::ExprId],
        value_initialize: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if value_initialize {
            let user = constructor
                .and_then(|id| self.graph.constructors.get(id).binding)
                .is_some_and(|binding| {
                    !matches!(
                        self.graph
                            .entity(self.graph.binding(binding).entity)
                            .special_definition,
                        Some(super::special_members::SpecialDefinition::Defaulted { .. })
                    )
                });
            if !user {
                self.constant_zero_object(target, location)?;
            }
            if let Some(id) = constructor
                && self.graph.constructors.get(id).nontrivial()
            {
                self.run_constant_constructor(target, id, values, scope, location)?;
            }
            return Ok(());
        }
        if let Some(id) = constructor {
            return self.run_constant_constructor(target, id, values, scope, location);
        }
        if let Some(&expr) = values.first() {
            return self.materialize_constant_object(expr, target, scope);
        }
        Ok(())
    }
    fn constant_aggregate(
        &mut self,
        target: Address,
        values: &[ast::ExprId],
        next: &mut usize,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.types.get(target.designated).kind {
            TypeKind::Array { element, bound, .. } => {
                let size = self.type_size(element, location)?.0;
                let ty = self.bare_type(element)?;
                for index in 0..bound {
                    let place = Address {
                        designated: ty,
                        offset: target
                            .offset
                            .wrapping_add(super::integral::signed(index.wrapping_mul(size))),
                        ..target
                    };
                    self.constant_aggregate_element(place, values, next, scope, location)?;
                }
            }
            TypeKind::Class(class) => {
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    let place = self.constant_member_slot(target, member, location)?;
                    self.constant_aggregate_element(place, values, next, scope, location)?;
                }
            }
            _ => {
                return Err(Error::Invariant(
                    "constant aggregate without aggregate type",
                ));
            }
        }
        Ok(())
    }
    fn constant_aggregate_element(
        &mut self,
        target: Address,
        values: &[ast::ExprId],
        next: &mut usize,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let Some(&expr) = values.get(*next) else {
            if let Some(class) = self.graph.class_of(target.designated)
                && !self.graph.aggregate_class(class)
            {
                let constructor = self.default_constructor(class, location)?;
                return self.constant_class_initialize(
                    target,
                    Some(constructor),
                    &[],
                    true,
                    scope,
                    location,
                );
            }
            return self.constant_zero_object(target, location);
        };
        let braced = matches!(
            self.tree.expressions.get(expr).kind,
            ExprKind::Braced { .. }
        );
        let aggregate = matches!(
            self.graph.types.get(target.designated).kind,
            TypeKind::Array { .. }
        ) || self
            .graph
            .class_of(target.designated)
            .is_some_and(|class| self.graph.aggregate_class(class));
        if aggregate
            && !braced
            && self
                .graph
                .expression_facts
                .get(expr)
                .is_ok_and(|facts| self.graph.class_of(facts.ty).is_none())
        {
            return self.constant_aggregate(target, values, next, scope, location);
        }
        *next += 1;
        let constructor = self.graph.expression_facts.get(expr)?.constructor;
        self.construct_constant_object(
            target,
            Some(ast::Initializer {
                kind: InitializerKind::Assign(expr),
                location: self.tree.expressions.get(expr).location,
            }),
            constructor,
            scope,
            location,
        )
    }
    pub(super) fn run_constant_constructor(
        &mut self,
        target: Address,
        id: ConstructorId,
        values: &[ast::ExprId],
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.constant_step(location)?;
        if self.graph.constructors.get(id).deleted {
            return Err(nonconstant(location, "a call of a deleted function"));
        }
        if self.graph.lower_constructor_copies_storage(id) {
            let Some(&source) = values.first() else {
                return Err(Error::Invariant("copy constructor without source"));
            };
            return self.materialize_constant_object(source, target, scope);
        }
        let binding = self.graph.constructors.get(id).binding;
        let class = self.graph.constructors.get(id).class;
        let returns = self.graph.types.scalar(Scalar::Void)?;
        let function = binding.map(|binding| self.graph.binding(binding).entity);
        if self.constexpr_frames.depth() >= 512 {
            return Err(nonconstant(location, "constexpr calls nested too deeply"));
        }
        let region;
        let frame = if let Some(function) = function {
            let entity = self.graph.entity(function);
            if !entity.constexpr
                && !matches!(
                    entity.special_definition,
                    Some(super::special_members::SpecialDefinition::Defaulted { .. })
                )
            {
                return Err(nonconstant(
                    location,
                    "a call of a function that is not constexpr",
                ));
            }
            if !self.constant_function_definition(function)? {
                return Err(nonconstant(
                    location,
                    "a constexpr function called before its definition",
                ));
            }
            let ty = self.graph.lower_entity_type(function)?;
            let parameters =
                self.bind_constant_parameters(function, ty, values, scope, location)?;
            // Argument temporaries belong to the caller; parameter objects end
            // with the constructor, before a later member call on its result.
            region = self.constexpr_frames.region();
            self.constant_function_frame(function, returns, parameters, Some(target), location)?
        } else {
            region = self.constexpr_frames.region();
            let mut frame = Frame::new(
                returns,
                self.graph
                    .entity(class)
                    .scope
                    .ok_or(Error::Invariant("constant class without scope"))?,
            );
            frame.object = Some(target);
            frame
        };
        let result = self.with_constant_frame(frame, |analyzer| {
            analyzer.constant_constructor_subobjects(target, id, location)?;
            if let Some(function) = function
                && let Some(body) = analyzer.graph.lower_function_body(function)
            {
                let ast::Body::Parsed(body) = *analyzer.tree.bodies.get(body) else {
                    return Err(Error::Invariant("unparsed constexpr constructor"));
                };
                if body.handlers.is_some() {
                    return Err(nonconstant(
                        location,
                        "a function-try-block in a constant expression",
                    ));
                }
                analyzer.constant_compound(body.compound)?;
            }
            Ok(())
        });
        self.constexpr_frames.release(region);
        result
    }
    fn constant_constructor_subobjects(
        &mut self,
        target: Address,
        id: ConstructorId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let scope = self.constexpr_frames.current()?.scope;
        if let Some(delegation) = self.graph.constructors.get(id).delegation {
            return self.construct_constant_object(
                target,
                Some(delegation.initializer),
                Some(delegation.constructor),
                scope,
                location,
            );
        }
        for index in 0..self.graph.constructors.get(id).bases.len() {
            let base = self.graph.constructors.get(id).bases[index];
            if base.forwards {
                return Err(Error::Missing("constexpr inherited constructor forwarding"));
            }
            let class = self.graph.constructors.get(id).class;
            let path = self.graph.retain_base_path(class, base.class)?;
            let place = self.constant_project_path(target, Some(path), location)?;
            self.construct_constant_object(
                place,
                base.initializer,
                base.constructor,
                scope,
                location,
            )?;
        }
        let mut next_subobject = 0;
        for index in 0..self.graph.constructors.get(id).members.len() {
            let (member, initializer) = self.graph.constructors.get(id).members[index];
            let place = self.constant_member_slot(target, member, location)?;
            let constructor = self
                .graph
                .constructors
                .get(id)
                .subobjects
                .get(next_subobject)
                .filter(|plan| plan.member == member)
                .map(|plan| plan.constructor);
            if constructor.is_some() {
                next_subobject += 1;
            }
            self.construct_constant_object(place, initializer, constructor, scope, location)?;
        }
        Ok(())
    }
}
