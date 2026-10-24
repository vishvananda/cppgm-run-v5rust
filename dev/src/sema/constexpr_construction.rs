// Translated from sema/constant_construction.cpp. See NOTICE.
//! Materialization and construction consume analyzed initialization and constructor plans.
use super::{
    Error,
    analyzer::Analyzer,
    call_arguments::Arguments,
    constant_addresses::{Address, nonconstant},
    constexpr_calls::ConstantArguments,
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
            return self.copy_constant_object(target, source, location);
        }
        if facts.constructor.is_none()
            && self.graph.expression_facts.initializer_array(id).is_some()
        {
            return self.constant_list_object(id, target, scope, location);
        }
        if facts.literal_call.is_some()
            || facts.call_form == super::expression_facts::CallForm::Operator
        {
            self.constant_call_into(id, scope, Some(target))?;
            return Ok(());
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
                    self.graph.lower_aggregate_union_member(elements),
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
                ExprKind::Call { .. } => {
                    self.constant_call_into(id, scope, Some(target))?;
                    return Ok(());
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
                    return self.initialize_constant_class_from(selected, target, scope);
                }
                ExprKind::Binary {
                    op, left, right, ..
                } if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Comma) => {
                    self.discard_constant(left, scope)?;
                    return self.materialize_constant_object(right, target, scope);
                }
                _ => {
                    return Err(nonconstant(
                        location,
                        "a class prvalue outside constant evaluation",
                    ));
                }
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
    pub(super) fn initialize_constant_class_from(
        &mut self,
        expr: ast::ExprId,
        target: Address,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.initialize_constant_class_at(
            expr,
            target,
            scope,
            self.tree.expressions.get(expr).location,
        )
    }
    fn initialize_constant_class_at(
        &mut self,
        expr: ast::ExprId,
        target: Address,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let Some((constructor, arguments)) = self.graph.expression_facts.class_conversion(expr)
            && self.graph.class_of(target.designated)
                == Some(self.graph.constructors.get(constructor).class)
        {
            let arguments = self.constant_constructor_arguments(arguments);
            return self.run_constant_constructor(
                target,
                constructor,
                arguments.as_slice(),
                scope,
                self.tree.expressions.get(expr).location,
            );
        }
        if let Some(super::copy_initialization::CopyAction::Storage { reference, .. }) =
            self.graph.lower_class_copy(expr)?
        {
            let facts = self
                .graph
                .expression_facts
                .get(self.graph.expression_facts.root(expr))?;
            if facts.category == ValueCategory::Prvalue
                && self.graph.class_of(facts.ty) == self.graph.class_of(target.designated)
            {
                return self.materialize_constant_object(expr, target, scope);
            }
            let source = ConstantArguments::Written(&[expr]).first_place(self, reference, scope)?;
            return self.copy_constant_object(target, source, location);
        }
        if let Some(super::copy_initialization::CopyAction::Constructor {
            constructor,
            elision: false,
            ..
        }) = self.graph.lower_class_copy(expr)?
        {
            return self.run_constant_constructor(target, constructor, &[expr], scope, location);
        }
        self.materialize_constant_object(expr, target, scope)
    }
    pub(super) fn constant_constructor_arguments(
        &self,
        arguments: ConstructorArguments,
    ) -> Arguments {
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
                if matches!(
                    self.graph.types.get(target.designated).kind,
                    TypeKind::Array { .. }
                ) {
                    return self.constant_default_array(target, constructor, scope, location);
                }
                return self.run_constant_constructor(target, constructor, &[], scope, location);
            }
            return Ok(());
        };
        if matches!(
            self.graph.types.get(target.designated).kind,
            TypeKind::Array { .. }
        ) && let Some(expr) = self.single_initializer(initializer)
            && self.constant_string_array(target, expr, location)?
        {
            return Ok(());
        }
        let values = match initializer.kind {
            InitializerKind::Assign(expr) | InitializerKind::Braced(expr) => {
                if let ExprKind::Braced { elements, .. } = self.tree.expressions.get(expr).kind {
                    if constructor.is_none()
                        && matches!(
                            self.graph.types.get(target.designated).kind,
                            TypeKind::Class(_)
                        )
                        && self
                            .graph
                            .expression_facts
                            .initializer_array(expr)
                            .is_some()
                    {
                        return self.constant_list_object(expr, target, scope, location);
                    }
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
                            self.graph.lower_list_union_member(expr),
                        );
                    }
                    values
                } else if self.graph.class_of(target.designated).is_some()
                    && (constructor.is_none()
                        || self.bare_type(self.graph.expression_facts.get(expr)?.ty)?
                            == self.bare_type(target.designated)?)
                {
                    if let Some(constructor) = constructor
                        && matches!(
                            self.graph.lower_class_copy(expr)?,
                            Some(super::copy_initialization::CopyAction::Constructor {
                                elision: false,
                                ..
                            })
                        )
                    {
                        return self.run_constant_constructor(
                            target,
                            constructor,
                            &[expr],
                            scope,
                            location,
                        );
                    }
                    return self.initialize_constant_class_at(expr, target, scope, location);
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
                super::constant_addresses::TemporaryValue::Pointer(place.pointer()),
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
    fn constant_default_array(
        &mut self,
        target: Address,
        constructor: ConstructorId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let mut element = target.designated;
        while let TypeKind::Array { element: child, .. } = self.graph.types.get(element).kind {
            element = child;
        }
        let size = self.type_size(element, location)?.0;
        let total = self.type_size(target.designated, location)?.0;
        let mut offset = 0;
        while size != 0 && offset < total {
            let place = Address {
                designated: element,
                offset: target.offset.wrapping_add(super::integral::signed(offset)),
                ..target
            };
            self.run_constant_constructor(place, constructor, &[], scope, location)?;
            offset = offset
                .checked_add(size)
                .ok_or(Error::Limit("constant array offset"))?;
        }
        Ok(())
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
            return self.initialize_constant_class_from(expr, target, scope);
        }
        Ok(())
    }
    fn constant_list_object(
        &mut self,
        id: ast::ExprId,
        target: Address,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let array = self
            .graph
            .expression_facts
            .initializer_array(id)
            .ok_or(Error::Invariant("constant list backing type"))?;
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(array).kind else {
            return Err(Error::Invariant("constant list backing array"));
        };
        if bound == 0 {
            return self.constant_zero_object(target, location);
        }
        let class = self
            .graph
            .class_of(target.designated)
            .ok_or(Error::Invariant("constant list wrapper"))?;
        let [first, second, ..] = self.graph.entity(class).members.as_slice() else {
            return Err(nonconstant(
                location,
                "a std::initializer_list without its array's address and size members",
            ));
        };
        let members = [*first, *second];
        let mut elements = self.allocate_constant_object(array, None, location)?;
        let ExprKind::Braced {
            elements: values, ..
        } = self.tree.expressions.get(id).kind
        else {
            return Err(Error::Invariant("constant list syntax"));
        };
        let values = Arguments::new(self.tree.expr_lists.get(values));
        self.constant_aggregate(elements, values.as_slice(), &mut 0, scope, location, None)?;
        elements.designated = element;
        let first = self.constant_member_slot(target, members[0], location)?;
        let second = self.constant_member_slot(target, members[1], location)?;
        self.store_constant_value(
            first,
            super::constant_addresses::TemporaryValue::Pointer(elements.pointer()),
            location,
        )?;
        self.store_constant_value(
            second,
            super::constant_addresses::TemporaryValue::Integer(super::constants::Constant {
                high: 0,
                bits: bound,
                ty: second.designated,
            }),
            location,
        )
    }

    fn constant_aggregate(
        &mut self,
        target: Address,
        values: &[ast::ExprId],
        next: &mut usize,
        scope: ScopeId,
        location: SourceOffset,
        selection: Option<super::graph::EntityId>,
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
                let selected = (self.graph.entity(class).key
                    == crate::lex::vocabulary::Keyword::Union)
                    .then(|| {
                        self.graph
                            .lower_selected_union_member(class, selection)
                            .or_else(|| self.graph.union_list_member(class, *next >= values.len()))
                    })
                    .flatten();
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member)
                        || selected.is_some_and(|selected| member != selected)
                    {
                        continue;
                    }
                    let place = self.constant_member_slot(target, member, location)?;
                    self.constexpr_frames.activate(place, location)?;
                    self.constant_aggregate_element(place, values, next, scope, location)?;
                    if self.graph.entity(class).key == crate::lex::vocabulary::Keyword::Union {
                        break;
                    }
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
            if matches!(
                self.graph.types.get(target.designated).kind,
                TypeKind::Reference { .. }
            ) {
                return Err(nonconstant(
                    location,
                    "a reference member without an initializer",
                ));
            }
            let mut element = target.designated;
            while let TypeKind::Array { element: child, .. } = self.graph.types.get(element).kind {
                element = child;
            }
            if let Some(class) = self.graph.class_of(element) {
                if self.graph.aggregate_class(class) {
                    // The frozen interpreter zeroes an omitted array of
                    // aggregates, but visits an omitted class's members.
                    if element == target.designated {
                        return self.constant_aggregate(target, &[], &mut 0, scope, location, None);
                    }
                } else {
                    let constructor = self.default_constructor(class, location)?;
                    return self.constant_value_initialize(target, constructor, scope, location);
                }
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
        if aggregate && !braced && !self.initializes_subaggregate(target.designated, expr)? {
            return self.constant_aggregate(target, values, next, scope, location, None);
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
        )?;
        if matches!(
            self.graph.types.get(target.designated).kind,
            TypeKind::Reference { .. }
        ) && self.graph.aggregate_constants.static_target(target)
            && self.constant_binds_temporary(expr)?
        {
            let super::constant_addresses::TemporaryValue::Pointer(place) =
                self.constexpr_frames.read(target, location)?
            else {
                return Err(Error::Invariant("reference member without address"));
            };
            let place = self.extend_constant_temporary(place, location)?;
            self.constexpr_frames.write(
                target,
                super::constant_addresses::TemporaryValue::Pointer(place.pointer()),
                location,
            )?;
        }
        Ok(())
    }
    fn constant_value_initialize(
        &mut self,
        target: Address,
        constructor: ConstructorId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        if let TypeKind::Array { element, bound, .. } = self.graph.types.get(target.designated).kind
        {
            let size = self.type_size(element, location)?.0;
            let element = self.bare_type(element)?;
            for index in 0..bound {
                let place = Address {
                    designated: element,
                    offset: target
                        .offset
                        .wrapping_add(super::integral::signed(index.wrapping_mul(size))),
                    ..target
                };
                self.constant_value_initialize(place, constructor, scope, location)?;
            }
            return Ok(());
        }
        self.constant_class_initialize(target, Some(constructor), &[], true, scope, location)
    }
    pub(super) fn run_constant_constructor(
        &mut self,
        target: Address,
        id: ConstructorId,
        values: &[ast::ExprId],
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.run_constant_constructor_inputs(
            target,
            id,
            ConstantArguments::Written(values),
            scope,
            location,
        )
    }
    fn run_constant_constructor_inputs(
        &mut self,
        target: Address,
        id: ConstructorId,
        values: ConstantArguments<'_>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.constant_step(location)?;
        if self.graph.constructors.get(id).deleted {
            return Err(nonconstant(location, "a call of a deleted function"));
        }
        if self.graph.lower_constructor_copies_storage(id) {
            return match values {
                ConstantArguments::Values(_) => Err(Error::Invariant(
                    "literal values used for a copy constructor",
                )),
                ConstantArguments::Written(values) => {
                    let &source = values
                        .first()
                        .ok_or(Error::Invariant("copy constructor without source"))?;
                    let facts = self
                        .graph
                        .expression_facts
                        .get(self.graph.expression_facts.root(source))?;
                    if facts.category == ValueCategory::Prvalue
                        && self.graph.class_of(facts.ty)
                            == Some(self.graph.constructors.get(id).class)
                    {
                        self.materialize_constant_object(source, target, scope)
                    } else {
                        let parameter = self
                            .graph
                            .types
                            .function_parameter(self.graph.constructors.get(id).callable_type, 0)?;
                        let from = ConstantArguments::Written(values)
                            .first_place(self, parameter, scope)?;
                        self.copy_constant_object(target, from, location)
                    }
                }
                ConstantArguments::Bound(values) => {
                    let &source = values
                        .first()
                        .ok_or(Error::Invariant("copy constructor without source"))?;
                    self.copy_constant_object(target, source, location)
                }
            };
        }
        if self.graph.constructors.get(id).copy.is_some() {
            let parameter = self
                .graph
                .types
                .function_parameter(self.graph.constructors.get(id).callable_type, 0)?;
            let source = values.first_place(self, parameter, scope)?;
            return self.copy_constant_subobjects(id, target, source, location);
        }
        let binding = self.graph.constructors.get(id).binding;
        // default_definition marks a later default as synthesized, but not
        // defaulted. Unlike the synthesized copy path above, evaluable() still
        // requires its non-copy constructor to have been declared constexpr.
        if let Some(binding) = binding {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            if !function.constexpr
                && matches!(
                    function.special_definition,
                    Some(super::special_members::SpecialDefinition::Defaulted { first: false, .. })
                )
            {
                return Err(nonconstant(
                    location,
                    "a call of a function that is not constexpr",
                ));
            }
        }
        let class = self.graph.constructors.get(id).class;
        let returns = self.graph.types.scalar(Scalar::Void)?;
        let function = binding
            .map(|binding| self.graph.binding(binding).entity)
            .filter(|&entity| {
                !matches!(
                    self.graph.entity(entity).special_definition,
                    Some(super::special_members::SpecialDefinition::Defaulted { .. })
                )
            });
        if self.constexpr_frames.depth() >= 512 {
            return Err(nonconstant(location, "constexpr calls nested too deeply"));
        }
        let region;
        let frame = if let Some(function) = function {
            let entity = self.graph.entity(function);
            if !entity.constexpr
                && self.graph.inherited_constructors.source(function).is_none()
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
            if self.graph.inherited_constructors.source(function).is_none()
                && !self.constant_function_definition(function)?
            {
                return Err(nonconstant(
                    location,
                    "a constexpr function called before its definition",
                ));
            }
            self.check_constant_constructor_object(target, class, location)?;
            let ty = self.graph.lower_entity_type(function)?;
            let parameters =
                self.bind_constant_inputs(function, ty, (values, false), scope, location)?;
            // Argument temporaries belong to the caller; parameter objects end
            // with the constructor, before a later member call on its result.
            region = self.constexpr_frames.region();
            self.constant_function_frame(function, returns, parameters, Some(target), location)?
        } else {
            self.check_constant_constructor_object(target, class, location)?;
            region = self.constexpr_frames.region();
            let mut frame = Frame::new(
                returns,
                self.graph
                    .entity(class)
                    .scope
                    .ok_or(Error::Invariant("constant class without scope"))?,
            );
            frame.object = Some(target);
            frame.source_function =
                Some(super::predefined_names::SourceFunction::ImplicitConstructor(class));
            frame
        };
        let parameters = frame.parameters.clone();
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
        if let Some(function) = function {
            let ty = self.graph.lower_entity_type(function)?;
            for (index, parameter) in parameters.into_iter().enumerate() {
                if !matches!(
                    self.graph
                        .types
                        .get(self.graph.types.function_parameter(ty, index)?)
                        .kind,
                    TypeKind::Reference { .. }
                ) {
                    self.constexpr_frames.end_lifetime(parameter);
                }
            }
        }
        self.constexpr_frames.release(region);
        result
    }
    fn check_constant_constructor_object(
        &mut self,
        target: Address,
        class: super::graph::EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if !self
            .constexpr_frames
            .initializing_declaration(target, location)?
            && !self.constant_literal_type(self.graph.lower_entity_type(class)?)?
        {
            return Err(nonconstant(
                location,
                "a constructor of a class that is not a literal type in a constant expression",
            ));
        }
        Ok(())
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
            let class = self.graph.constructors.get(id).class;
            // The ordered plan names a direct base, even if that same class
            // also occurs indirectly. This is not an unambiguous base conversion.
            let edge = self
                .graph
                .entity(class)
                .bases
                .get(index)
                .copied()
                .filter(|edge| edge.class == base.class)
                .ok_or(Error::Invariant(
                    "constructor base plan without direct edge",
                ))?;
            let place = Address {
                offset: target.offset.wrapping_add(super::integral::signed(
                    edge.offset
                        .ok_or(Error::Invariant("constructor base without layout"))?,
                )),
                designated: self.graph.lower_entity_type(base.class)?,
                ..target
            };
            if base.forwards {
                let parameters = self.constexpr_frames.current()?.parameters.clone();
                self.run_constant_constructor_inputs(
                    place,
                    base.constructor
                        .ok_or(Error::Invariant("forwarded base without constructor"))?,
                    ConstantArguments::Bound(&parameters),
                    scope,
                    location,
                )?;
                continue;
            }
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
            let anonymous = self
                .graph
                .entity(member)
                .ty
                .and_then(|ty| self.graph.class_of(ty))
                .is_some_and(|class| self.graph.anonymous_member_storage(class).is_some());
            if anonymous {
                if let Some(child) = constructor {
                    if place.variant().is_some() && self.constant_plan_initializes(child) {
                        self.constexpr_frames.activate(place, location)?;
                    }
                    self.constant_constructor_subobjects(place, child, location)?;
                }
                continue;
            }
            if initializer.is_none()
                && constructor.is_none_or(|id| !self.graph.constructors.get(id).nontrivial())
            {
                continue;
            }
            if place.variant().is_some()
                && !matches!(
                    self.graph.types.get(place.designated).kind,
                    TypeKind::Reference { .. }
                )
            {
                self.constexpr_frames.activate(place, location)?;
            }
            let written = self.graph.constructors.get(id).written_members[index];
            let function = self.graph.lower_constructor_entity(id);
            let body_site = function
                .and_then(|function| self.graph.lower_function_body(function))
                .and_then(|body| {
                    if let ast::Body::Parsed(body) = self.tree.bodies.get(body) {
                        Some(self.tree.statements.get(body.compound).location)
                    } else {
                        None
                    }
                });
            self.with_source_site((!written).then_some(body_site).flatten(), |analyzer| {
                analyzer.construct_constant_object(place, initializer, constructor, scope, location)
            })?;
        }
        Ok(())
    }
    fn copy_constant_subobjects(
        &mut self,
        id: ConstructorId,
        target: Address,
        source: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let count = self
            .graph
            .constructors
            .get(id)
            .copy
            .as_ref()
            .ok_or(Error::Invariant("copy without subobject plan"))?
            .len();
        for index in 0..count {
            let action = self
                .graph
                .constructors
                .get(id)
                .copy
                .as_ref()
                .ok_or(Error::Invariant("lost constant copy plan"))?[index];
            let (to, from) = match action.target {
                super::copy_construction::CopyTarget::VirtualBase(base) => {
                    let class = self.graph.constructors.get(id).class;
                    self.class_layout(class, location)?;
                    let offset = super::integral::signed(
                        self.graph
                            .lower_virtual_base_offset(class, base)
                            .ok_or(Error::Invariant("constant copy virtual base layout"))?,
                    );
                    let ty = self.bare_type(action.ty)?;
                    (
                        Address {
                            offset: target.offset.wrapping_add(offset),
                            designated: ty,
                            ..target
                        },
                        Address {
                            offset: source.offset.wrapping_add(offset),
                            designated: ty,
                            ..source
                        },
                    )
                }
                super::copy_construction::CopyTarget::Base(index) => {
                    let class = self.graph.constructors.get(id).class;
                    self.class_layout(class, location)?;
                    let edge = self.graph.entity(class).bases[index as usize];
                    let offset = super::integral::signed(
                        edge.offset
                            .ok_or(Error::Invariant("copy base without offset"))?,
                    );
                    let ty = self.bare_type(action.ty)?;
                    (
                        Address {
                            offset: target.offset.wrapping_add(offset),
                            designated: ty,
                            ..target
                        },
                        Address {
                            offset: source.offset.wrapping_add(offset),
                            designated: ty,
                            ..source
                        },
                    )
                }
                super::copy_construction::CopyTarget::Member(member) => {
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    (
                        self.constant_member_slot(target, member, location)?,
                        self.constant_member_slot(source, member, location)?,
                    )
                }
            };
            if let Some(child) = action
                .constructor
                .filter(|&id| self.graph.constructors.get(id).nontrivial())
            {
                self.run_constant_constructor_inputs(
                    to,
                    child,
                    ConstantArguments::Bound(&[from]),
                    self.graph.global,
                    location,
                )?;
            } else {
                self.copy_constant_object(to, from, location)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{analyzer::Analyzer, constants::ConstantState, lookup::Filter},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::{SyntaxUnit, ast},
    };

    #[test]
    fn inherited_parameters_and_selected_member_copies_keep_local_ownership() {
        let source = SourceBuffer::new(
            br"
            struct S {
                int n;
                constexpr S(int x):n(x){}
                constexpr S(const S& other):n(other.n+1){}
            };
            struct B {int n; constexpr B(S s):n(s.n){}};
            struct D:B {using B::B;};
            struct M {S s; constexpr M():s(3){}};
            constexpr int f(int n) {
                S source(n); D d(source); M a; M b=a;
                b.s.n+=2;
                return d.n*100+b.s.n*10+a.s.n;
            }
            constexpr int a=f(3);
            constexpr int b=f(7);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"constructor-ownership").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names = [b"a", b"b"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        for (name, bits) in names.into_iter().zip([463, 863]) {
            let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
            assert!(
                matches!(analyzer.graph.entity(analyzer.graph.binding(binding).entity).constant,
                ConstantState::Value(value) if value.bits == bits)
            );
        }
        let declaration = *analyzer.tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let syntax = analyzer.tree.expressions.mark();
        for _ in 0..3 {
            assert_eq!(
                analyzer
                    .constant_value(expr, scope, true)
                    .unwrap()
                    .unwrap()
                    .bits,
                863
            );
            assert_eq!(analyzer.tree.expressions.mark(), syntax);
            assert!(!analyzer.constexpr_frames.active());
            assert!(analyzer.constant_evaluation.is_none());
        }
    }

    #[test]
    fn later_defaulted_constructor_keeps_declared_constant_eligibility() {
        let source = SourceBuffer::new(
            br"
            struct Late { Late() noexcept; };
            Late::Late() noexcept = default;
            struct Constant { constexpr Constant() noexcept; };
            constexpr Constant::Constant() noexcept = default;
            Late dynamic;
            Constant constant;
            "
            .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"later-default").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let syntax = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let graph = analyzer.analyze(declarations).unwrap();
        assert_eq!(tree.expressions.mark(), syntax);
        for (name, constant) in [
            (b"dynamic".as_slice(), false),
            (b"constant".as_slice(), true),
        ] {
            let binding = graph
                .own(
                    graph.global,
                    cursor.intern_name(name).unwrap(),
                    Filter::Ordinary,
                )
                .unwrap();
            let entity = graph.binding(binding).entity;
            assert_eq!(graph.lower_array_image(entity).is_some(), constant);
            let constructor = graph.lower_construction(entity).unwrap();
            assert!(!graph.lower_constructor_trivial(constructor));
        }
    }
}
