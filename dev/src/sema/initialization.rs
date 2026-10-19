// Translated from analyzer_initialization.cpp, analyzer_construction.cpp and
// analyzer_expressions.cpp::braced_list. See NOTICE.
//! Object/list initialization retains conversions on original element IDs.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::TemporaryValue,
    constants::Constant,
    expression_facts::ValueCategory as V,
    graph::{BindingId, ScopeId},
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::PostTokenKind,
    support::source::SourceOffset,
    syntax::{
        ast::{self, ExprKind, InitializerKind},
        storage::List,
    },
    types::FundamentalType as F,
};
impl Analyzer<'_> {
    pub(super) fn initialize_variable(
        &mut self,
        binding: BindingId,
        initializer: ast::Initializer,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let target = self.graph.publish_variable_type(binding)?;
        let entity = self.graph.binding(binding).entity;
        let saved = self.member_initializer_class;
        if self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance) {
            self.member_initializer_class = self
                .graph
                .entity(entity)
                .home
                .and_then(|home| self.graph.scope(home).owner);
        }
        let saved_initializing = self.initializing;
        if !self.graph.lower_automatic(entity) {
            self.initializing = Some(entity);
        }
        let completed = if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            self.initialize_class_object(target, initializer, scope, location)
                .map(|selected| {
                    if let Some((constructor, arguments)) = selected {
                        self.graph.entity_mut(entity).constructor = Some(constructor);
                        self.graph.entity_mut(entity).constructor_arguments = Some(arguments);
                    }
                    target
                })
        } else {
            self.object_initializer(initializer, scope, target, location)
        };
        self.initializing = saved_initializing;
        self.member_initializer_class = saved;
        let completed = completed?;
        self.graph.complete_variable_type(binding, completed)?;
        self.check_nothrow_initializer(self.graph.binding(binding).entity, initializer)?;
        Ok(completed)
    }
    pub(super) fn initial_object_type(
        &self,
        target: TypeId,
        initializer: Option<ast::Initializer>,
    ) -> TypeId {
        // A list may complete its outer bound; its element must already be complete.
        if self.semantic()
            && let TypeKind::Array {
                element,
                bound: 0,
                operand: None,
            } = self.graph.types.get(target).kind
            && initializer.is_some_and(|init| {
                matches!(init.kind,
                InitializerKind::Assign(id) | InitializerKind::Braced(id)
                    if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }))
            })
        {
            element
        } else {
            target
        }
    }
    pub(super) fn object_initializer(
        &mut self,
        initializer: ast::Initializer,
        scope: ScopeId,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            self.initialize_class_object(target, initializer, scope, location)?;
            return Ok(target);
        }
        let (id, direct) = match initializer.kind {
            InitializerKind::Assign(id) | InitializerKind::Braced(id)
                if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }) =>
            {
                let completed = self.list_initialize(id, target, scope)?;
                return Ok(
                    if matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
                        completed
                    } else {
                        target
                    },
                );
            }
            InitializerKind::Assign(id) => (id, false),
            InitializerKind::Parenthesized(arguments) => {
                let arguments = self.tree.expr_lists.get(arguments);
                if arguments.is_empty() {
                    if matches!(
                        self.graph.types.get(target).kind,
                        TypeKind::Reference { .. }
                    ) {
                        return Err(Error::at(location, "value-initialization of a reference"));
                    }
                    return Ok(target);
                }
                if arguments.len() != 1 {
                    return Err(Error::at(
                        location,
                        "direct-initialization from several values",
                    ));
                }
                (arguments[0], true)
            }
            _ => return Err(Error::Invariant("nonobject initializer")),
        };
        self.typed_expression(id, scope)?;
        if matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
            self.character_array(id, target, location)
        } else {
            self.convert_scalar(id, target, direct)?;
            Ok(target)
        }
    }
    pub(super) fn initialize_ranked_list_argument(
        &mut self,
        id: ast::ExprId,
        sequence: super::procedural_calls::ArgumentSequence,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.initialize_list_argument(
            id,
            sequence.conversion.target,
            scope,
            sequence.user_conversion,
        )
    }
    pub(super) fn initialize_list_argument(
        &mut self,
        id: ast::ExprId,
        parameter: TypeId,
        scope: ScopeId,
        selected: Option<super::procedural_calls::UserConversion>,
    ) -> Result<(), Error> {
        // A parameter default's list denotes the temporary object, rather
        // than binding a reference directly to its single source element.
        let target = match self.graph.types.get(parameter).kind {
            TypeKind::Reference { element, kind } => {
                if kind == RefKind::Lvalue && self.type_cv(element) != Cv::CONST {
                    return Err(Error::at(
                        self.tree.expressions.get(id).location,
                        "an lvalue reference to a type that is not const bound to a list's temporary",
                    ));
                }
                element
            }
            _ => parameter,
        };
        if self.initializer_list_element(target).is_some() {
            self.initialize_list_object(id, target, scope)?;
        } else if matches!(self.graph.types.get(target).kind, TypeKind::Array { .. }) {
            self.array_braced_list(id, target, scope)?;
        } else if matches!(self.graph.types.get(target).kind, TypeKind::Class(_))
            && let Some(super::procedural_calls::UserConversion::Constructor(binding)) = selected
        {
            let ExprKind::Braced { elements, .. } = self.tree.expressions.get(id).kind else {
                return Err(Error::Invariant("ranked class list without list"));
            };
            self.initialize_class_arguments(
                target,
                super::constructor_initialization::ClassInput {
                    arguments: if self.initializer_list_constructor(binding)? {
                        super::constructor_initialization::ConstructorArguments::Single(id)
                    } else {
                        super::constructor_initialization::ConstructorArguments::Values(elements)
                    },
                    list: Some(id),
                    converting: false,
                    copy_list: true,
                    context: super::constructor_initialization::ClassContext::Object,
                    selected: Some(binding),
                },
                scope,
                self.tree.expressions.get(id).location,
            )?;
        } else {
            let object = if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
                target
            } else {
                self.bare_type(target)?
            };
            self.list_initialize(id, object, scope)?;
        }
        let mut facts = self.graph.expression_facts.get(id)?;
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            // The list's reference temporary retains its target class cv,
            // unlike a standalone aggregate or functional-cast prvalue.
            facts.ty = target;
            super::record(&mut self.graph.expression_types, id.index(), target);
        }
        facts.category = V::Prvalue;
        self.graph.expression_facts.set(id, facts);
        Ok(())
    }

    pub(super) fn list_initialize(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let node = *self.tree.expressions.get(id);
        let ExprKind::Braced { elements, .. } = node.kind else {
            return Err(Error::Invariant("list initialization without list"));
        };
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(elements));
        let values = argument_buffer.as_slice();
        if let TypeKind::Reference { element, kind } = self.graph.types.get(target).kind {
            let single = values.first().copied().filter(|_| values.len() == 1);
            if let Some(single) = single
                && !matches!(
                    self.tree.expressions.get(single).kind,
                    ExprKind::Braced { .. }
                )
            {
                self.ensure_typed(single, scope)?;
                let source = self.graph.expression_facts.get(single)?.ty;
                if self.bare_type(element)? == self.bare_type(source)? {
                    self.initialize_element(single, target, scope, false)?;
                    return self.publish_list(id, target, V::Lvalue);
                }
            }
            if kind == RefKind::Lvalue && self.type_cv(element).0 != Cv::CONST.0 {
                return Err(Error::at(
                    node.location,
                    "an lvalue reference to a type that is not const bound to a list's temporary",
                ));
            }
            if self.initializer_list_element(element).is_some() {
                self.initialize_list_argument(id, target, scope, None)?;
                return Ok(self.graph.expression_facts.get(id)?.ty);
            }
            let object = if matches!(self.graph.types.get(element).kind, TypeKind::Array { .. }) {
                element
            } else {
                self.bare_type(element)?
            };
            let object = if matches!(self.graph.types.get(object).kind, TypeKind::Array { .. }) {
                self.array_braced_list(id, object, scope)?
            } else {
                self.list_initialize(id, object, scope)?
            };
            if self.graph.expression_facts.initializer_array(id).is_some() {
                return Ok(object);
            }
            if matches!(self.graph.types.get(object).kind, TypeKind::Array { .. }) {
                // Array list typing publishes element facts; the binding
                // still needs the whole array's prvalue category.
                return self.publish_list(id, object, V::Prvalue);
            }
            let mut facts = self.graph.expression_facts.get(id)?;
            if matches!(self.graph.types.get(element).kind, TypeKind::Class(_)) {
                // list_argument keeps the referenced class's cv on the
                // materialized object, including its aggregate helper key.
                facts.ty = element;
                self.graph.expression_facts.set(id, facts);
                super::record(&mut self.graph.expression_types, id.index(), element);
            }
            self.check_temporary_destruction(facts, scope, node.location)?;
            // Recursive object initialization already published the selected
            // aggregate/constructor facts. The reference binds that object;
            // publishing a fresh scalar fact here would erase its plan.
            return Ok(facts.ty);
        }
        if let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind {
            if values.len() == 1
                && self.character_element(element)
                && self.string_literal(values[0])
            {
                let single = values[0];
                self.ensure_typed(single, scope)?;
                let source = self.graph.expression_facts.get(single)?.ty;
                let TypeKind::Array {
                    element: literal,
                    bound: length,
                    ..
                } = self.graph.types.get(source).kind
                else {
                    return Err(Error::Invariant("string without array type"));
                };
                if !self.character_compatible(literal, element) {
                    return Err(Error::at(
                        node.location,
                        "array initialized from a string literal of another kind",
                    ));
                }
                if bound != 0 && length > bound {
                    return Err(Error::at(
                        node.location,
                        "string literal too long for the array",
                    ));
                }
                let target =
                    self.completed_array_type(target, if bound == 0 { length } else { bound })?;
                return self.publish_list(id, target, V::Lvalue);
            }
            return self.array_braced_list(id, target, scope);
        }
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            return self.class_list_initialize(id, target, scope);
        }
        if values.len() > 1 {
            return Err(Error::at(
                node.location,
                "list-initialization of a scalar from several values",
            ));
        }
        if let Some(&single) = values.first() {
            if matches!(
                self.tree.expressions.get(single).kind,
                ExprKind::Braced { .. }
            ) {
                return Err(Error::at(
                    self.tree.expressions.get(single).location,
                    "unsupported nested braced initializer",
                ));
            }
            self.initialize_element(single, target, scope, false)?;
        }
        self.publish_list(id, target, V::Lvalue)
    }
    pub(super) fn array_braced_list(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        self.check_stack()?;
        let node = *self.tree.expressions.get(id);
        let ExprKind::Braced { elements, .. } = node.kind else {
            return Err(Error::Invariant("array initialization without list"));
        };
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("array initialization without array"));
        };
        let argument_buffer =
            super::call_arguments::Arguments::new(self.tree.expr_lists.get(elements));
        let values = argument_buffer.as_slice();
        let mut next = 0;
        let mut count = 0u64;
        // O(explicit initializers), including a huge bound with a zero-filled tail.
        while next < values.len() && (bound == 0 || count < bound) {
            let before = next;
            self.aggregate_array_element(elements, &mut next, element, scope, node.location)?;
            if next == before {
                return Err(Error::Invariant("array initialization made no progress"));
            }
            count = count
                .checked_add(1)
                .ok_or(Error::Limit("array initializer size limit"))?;
        }
        if next < values.len() {
            return Err(Error::at(
                node.location,
                "too many initializers for the array",
            ));
        }
        if count < bound {
            self.missing_aggregate_element(element, scope, node.location)?;
        }
        let target = self.completed_array_type(target, if bound == 0 { count } else { bound })?;
        self.publish_list(id, target, V::Lvalue)
    }
    pub(super) fn aggregate_array_element(
        &mut self,
        elements: List<ast::ExprId>,
        next: &mut usize,
        target: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let Some(&id) = self.tree.expr_lists.get(elements).get(*next) else {
            self.missing_aggregate_element(target, scope, location)?;
            return Ok(());
        };
        if matches!(self.tree.expressions.get(id).kind, ExprKind::Braced { .. }) {
            *next += 1;
            self.list_initialize(id, target, scope)?;
        } else if let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind {
            if self.string_literal(id) {
                *next += 1;
                self.ensure_typed(id, scope)?;
                self.character_array(id, target, self.tree.expressions.get(id).location)?;
            } else {
                if bound == 0 {
                    return Err(Error::Missing("incomplete subarray initialization"));
                }
                let mut count = 0;
                while count < bound && *next < self.tree.expr_lists.get(elements).len() {
                    self.aggregate_array_element(elements, next, element, scope, location)?;
                    count += 1;
                }
            }
        } else if let TypeKind::Class(class) = self.graph.types.get(target).kind {
            self.ensure_typed(id, scope)?;
            if self.graph.class_of(self.graph.expression_facts.get(id)?.ty) == Some(class) {
                *next += 1;
                self.record_class_copy(
                    id,
                    target,
                    scope,
                    super::copy_initialization::CopyUse::Value { in_place: false },
                    self.tree.expressions.get(id).location,
                )?;
            } else if self.graph.aggregate_class(class) {
                self.aggregate_members(elements, next, class, scope, location)?;
            } else {
                *next += 1;
                let selected = self.initialize_class_arguments(
                    target,
                    super::constructor_initialization::ClassInput {
                        selected: None,
                        arguments: super::constructor_initialization::ConstructorArguments::Single(
                            id,
                        ),
                        list: None,
                        converting: true,
                        copy_list: false,
                        context: super::constructor_initialization::ClassContext::AggregateMember,
                    },
                    scope,
                    self.tree.expressions.get(id).location,
                )?;
                if let Some(selected) = selected {
                    self.graph
                        .expression_facts
                        .set_class_conversion(id, selected);
                }
            }
        } else {
            *next += 1;
            self.initialize_element(id, target, scope, false)?;
        }
        Ok(())
    }
    pub(super) fn initialize_element(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
        direct: bool,
    ) -> Result<(), Error> {
        if matches!(self.graph.types.get(target).kind, TypeKind::Class(_)) {
            return Err(Error::Missing("class element initialization"));
        }
        self.ensure_typed(id, scope)?;
        self.convert_scalar(id, target, direct)?;
        if self.parenthesized_elements.contains(&id) {
            return Ok(());
        }
        self.check_narrowing(id, target, scope)
    }
    pub(super) fn ensure_typed(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<(), Error> {
        if self.graph.expression_facts.get(id).is_err() {
            self.typed_expression(id, scope)?;
        }
        Ok(())
    }
    pub(super) fn publish_list(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        category: V,
    ) -> Result<TypeId, Error> {
        let facts = self.new_expression_facts(ty, category)?;
        self.graph.expression_facts.set(id, facts);
        super::record(&mut self.graph.expression_types, id.index(), facts.ty);
        Ok(facts.ty)
    }
    fn completed_array_type(&mut self, target: TypeId, bound: u64) -> Result<TypeId, Error> {
        let TypeKind::Array { element, .. } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("completing nonarray bound"));
        };
        self.graph.types.intern(
            TypeKind::Array {
                element,
                bound,
                operand: None,
            },
            Cv::default(),
        )
    }
    pub(super) fn string_literal(&self, mut id: ast::ExprId) -> bool {
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        matches!(
            self.tree.expressions.get(id).kind,
            ExprKind::Literal(ast::Literal {
                kind: PostTokenKind::String { suffix: None, .. },
                ..
            })
        )
    }
    pub(super) fn character_element(&self, ty: TypeId) -> bool {
        matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(
                Scalar::SignedChar
                    | Scalar::UnsignedChar
                    | Scalar::Basic(F::Char | F::Char16T | F::Char32T | F::WcharT)
            )
        )
    }
    fn character_compatible(&self, literal: TypeId, element: TypeId) -> bool {
        self.character_element(element)
            && (self.graph.types.get(literal).kind == self.graph.types.get(element).kind
                || matches!(
                    self.graph.types.get(literal).kind,
                    TypeKind::Scalar(Scalar::Basic(F::Char))
                ) && matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Scalar(Scalar::SignedChar | Scalar::UnsignedChar)
                ))
    }
    fn character_array(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let TypeKind::Array { element, bound, .. } = self.graph.types.get(target).kind else {
            return Err(Error::Invariant("character initialization without array"));
        };
        let source = self.graph.expression_facts.get(id)?.ty;
        let TypeKind::Array {
            element: literal,
            bound: length,
            ..
        } = self.graph.types.get(source).kind
        else {
            return Err(Error::at(location, "array initialized from an expression"));
        };
        if !self.string_literal(id) || !self.character_compatible(literal, element) {
            return Err(Error::at(location, "array initialized from an expression"));
        }
        if bound != 0 && length > bound {
            return Err(Error::at(location, "string literal too long for the array"));
        }
        Ok(target)
    }
    pub(super) fn check_narrowing(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        // A conversion function's result precedes the second standard
        // conversion. Narrow that value, not the original class operand.
        let source = self
            .graph
            .expression_facts
            .function_conversion(id)
            .map_or(self.graph.expression_facts.get(id)?.ty, |step| step.result);
        let target = if let TypeKind::Reference { element, .. } = self.graph.types.get(target).kind
        {
            element
        } else {
            target
        };
        let floating = |ty| match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Basic(F::Float)) => Some(24u32),
            TypeKind::Scalar(Scalar::Basic(F::Double)) => Some(53),
            TypeKind::Scalar(Scalar::Basic(F::LongDouble)) => Some(64),
            _ => None,
        };
        let sf = floating(source);
        let tf = floating(target);
        let narrowing = if sf.is_some() && self.integral_kind(target).is_some() {
            true
        } else if let (Some(to), Some(from)) =
            (self.integral_kind(target), self.integral_kind(source))
        {
            if to.bits < from.bits
                || to.bits == from.bits && to.signed != from.signed
                || !to.signed && from.signed
            {
                self.narrowing_integer(id, scope)?
                    .is_none_or(|value| !to.represents(value.bits, from))
            } else {
                false
            }
        } else if let Some(precision) = tf.filter(|_| self.integral_kind(source).is_some()) {
            self.narrowing_integer(id, scope)?.is_none_or(|value| {
                let negative = self.integral_kind(value.ty).is_some_and(|kind| kind.signed)
                    && super::integral::signed(value.bits) < 0;
                let magnitude = if negative {
                    value.bits.wrapping_neg()
                } else {
                    value.bits
                };
                magnitude != 0
                    && 64 - magnitude.leading_zeros() - magnitude.trailing_zeros() > precision
            })
        } else if let (Some(from), Some(to)) = (sf, tf) {
            if to < from {
                self.floating_list_out_of_range(id, target, scope)?
            } else {
                false
            }
        } else {
            false
        };
        if narrowing {
            Err(Error::at(
                self.tree.expressions.get(id).location,
                "narrowing conversion in list-initialization",
            ))
        } else {
            Ok(())
        }
    }
    fn narrowing_value(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<TemporaryValue>, Error> {
        // Evaluate the already selected conversion only up to its returned
        // scalar. Applying the second conversion first would hide truncation.
        let result = self.in_constant_evaluation(|analyzer| {
            if analyzer
                .graph
                .expression_facts
                .function_conversion(id)
                .is_some()
            {
                let returned = analyzer.constant_conversion_result(id, scope)?;
                analyzer.returned_scalar(returned, analyzer.tree.expressions.get(id).location)
            } else {
                analyzer.scalar_value(id, scope)
            }
        });
        match result {
            Ok(value) => Ok(Some(value)),
            Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    fn narrowing_integer(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<Option<Constant>, Error> {
        Ok(match self.narrowing_value(id, scope)? {
            Some(TemporaryValue::Integer(value)) => Some(value),
            _ => None,
        })
    }
    fn floating_list_out_of_range(
        &mut self,
        id: ast::ExprId,
        target: TypeId,
        scope: ScopeId,
    ) -> Result<bool, Error> {
        let Some(TemporaryValue::Floating(value)) = self.narrowing_value(id, scope)? else {
            return Ok(true);
        };
        let format = self
            .float_format(target)
            .ok_or(Error::Invariant("floating list target"))?;
        // The oracle checks isinf after target rounding. NaNs stay in range;
        // finite overflow and an already infinite source both narrow.
        Ok(value.value.is_infinite() || value.value.rounded(format).is_none())
    }
}
