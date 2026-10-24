// Translated from sema/{constant_evaluator,analyzer_traits}.cpp and layout.cpp. See NOTICE.
//! Unevaluated operands consume declared types; they never demand an object construction.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    graph::{BindingId, BindingKind, ScopeId},
    lookup::Filter,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::pp_lexer::Punctuator as P,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
    types::FundamentalType as F,
};
impl Analyzer<'_> {
    pub(super) fn typeof_expression(
        &mut self,
        id: ast::ExprId,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
    ) -> Result<super::expression_facts::ExprFacts, Error> {
        use super::expression_facts::ValueCategory as V;
        let (ty, category) = if let Some(operand) = operand {
            // The enclosing decltype establishes the unevaluated context. Its
            // final-call exemption belongs to this query, not its operand:
            // typeof still checks destruction of a class prvalue operand.
            let facts = self.typed_expression(operand, scope)?;
            self.check_resolved_function(operand)?;
            self.check_temporary_destruction(facts, scope, self.tree.expressions.get(id).location)?;
            (facts.ty, V::Prvalue)
        } else {
            // Facts never hold reference types. The enclosing decltype puts
            // a type-id's reference back from this category.
            let ty =
                self.type_syntax(ty.ok_or(Error::Invariant("typeof without operand"))?, scope)?;
            match self.graph.types.get(ty).kind {
                TypeKind::Reference { element, kind } => (
                    element,
                    if kind == RefKind::Lvalue {
                        V::Lvalue
                    } else {
                        V::Xvalue
                    },
                ),
                _ => (ty, V::Prvalue),
            }
        };
        self.new_expression_facts(ty, category)
    }

    pub(super) fn semantic_size(
        &mut self,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        parenthesized: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<super::expression_facts::ExprFacts, Error> {
        let mut ty = if let Some(ty) = ty {
            Some(self.type_syntax(ty, scope)?)
        } else {
            None
        };
        if ty.is_none() && parenthesized {
            let operand = operand.ok_or(Error::Invariant("sizeof without operand"))?;
            if let ExprKind::Id(name) = self.tree.expressions.get(operand).kind {
                let found = self.lookup_result(name, scope, Filter::Ordinary)?;
                if let Some(binding) = found.binding
                    && matches!(
                        self.graph.binding(binding).kind,
                        BindingKind::Type | BindingKind::Alias
                    )
                {
                    ty = Some(self.binding_expression_type(binding)?);
                }
            }
            if ty.is_none() && self.empty_query_cast(operand, scope)? {
                return Err(Error::at(location, "sizeof of a function type"));
            }
        }
        let ty = if let Some(ty) = ty {
            ty
        } else {
            let operand = operand.ok_or(Error::Invariant("sizeof without operand"))?;
            self.unevaluated += 1;
            let result = self.typed_expression(operand, scope).and_then(|mut facts| {
                if facts.is_bit_field() {
                    return Err(Error::at(location, "sizeof of a bit-field"));
                }
                if let Some(binding) = self.ordinary_function_operand(facts) {
                    self.select_function_designation(operand, binding)?;
                    facts = self.graph.expression_facts.get(operand)?;
                }
                if facts.functions.is_some()
                    || matches!(
                        self.graph.types.get(facts.ty).kind,
                        TypeKind::Function { .. }
                    )
                {
                    return Err(Error::at(location, "sizeof of a function"));
                }
                self.check_temporary_destruction(facts, scope, location)?;
                Ok(facts.ty)
            });
            self.unevaluated -= 1;
            result?
        };
        let ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Function { .. }
                | TypeKind::Scalar(Scalar::Void)
                | TypeKind::Array { bound: 0, .. }
        ) {
            return Err(Error::at(
                location,
                "sizeof of an incomplete or function type",
            ));
        }
        let (size, _) = self.semantic_type_size(ty, location)?;
        let target = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
        let mut facts =
            self.new_expression_facts(target, super::expression_facts::ValueCategory::Prvalue)?;
        facts.query_value = Some(size);
        facts.query_kind = self.query_kind(ty);
        Ok(facts)
    }

    fn empty_query_cast(&mut self, id: ast::ExprId, scope: ScopeId) -> Result<bool, Error> {
        match self.tree.expressions.get(id).kind {
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => Ok(arguments.is_empty() && braced.is_none()),
            ExprKind::Call { callee, arguments } if arguments.is_empty() => {
                let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
                    return Ok(false);
                };
                let found = self.lookup_call_result(name, scope)?;
                Ok(found.binding.is_some_and(|binding| {
                    matches!(
                        self.graph.binding(binding).kind,
                        BindingKind::Type | BindingKind::Alias
                    )
                }))
            }
            _ => Ok(false),
        }
    }

    pub(super) fn semantic_alignment(
        &mut self,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<super::expression_facts::ExprFacts, Error> {
        let type_operand = ty.is_some();
        let syntax_type = ty;
        let mut ty = if let Some(ty) = ty {
            self.type_syntax(ty, scope)?
        } else {
            let operand = operand.ok_or(Error::Invariant("alignof without operand"))?;
            self.unevaluated += 1;
            let result = self.typed_expression(operand, scope).and_then(|facts| {
                self.check_resolved_function(operand)?;
                Ok(facts)
            });
            self.unevaluated -= 1;
            let facts = result?;
            facts.ty
        };
        let query_kind = if type_operand {
            self.query_kind(ty)
        } else {
            super::expression_facts::QueryKind::Ordinary
        };
        if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        if !self.query_complete(ty)? {
            return Err(Error::at(
                location,
                "alignof of an incomplete or function type",
            ));
        }
        let (_, natural) = self.semantic_type_size(ty, location)?;
        let aliased = if let Some(syntax) = syntax_type {
            self.graph.alias_alignments.type_operand(syntax)
        } else {
            operand.map_or(Ok(0), |operand| self.expression_alias_alignment(operand))?
        };
        let alignment = if aliased == 0 {
            natural
        } else {
            u64::from(aliased)
        };
        let ty = self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?;
        let mut facts =
            self.new_expression_facts(ty, super::expression_facts::ValueCategory::Prvalue)?;
        facts.query_value = Some(alignment);
        facts.query_kind = query_kind;
        Ok(facts)
    }
    fn query_kind(&self, ty: TypeId) -> super::expression_facts::QueryKind {
        if matches!(self.graph.types.get(ty).kind, TypeKind::Class(entity)
            if self.graph.class_templates.instance(entity).is_some())
        {
            super::expression_facts::QueryKind::Specialization
        } else {
            super::expression_facts::QueryKind::Ordinary
        }
    }
    pub(super) fn binding_expression_type(&self, binding: BindingId) -> Result<TypeId, Error> {
        let binding = self.graph.binding(binding);
        let ty = if binding.kind == BindingKind::Variable
            || self.evaluating_noexcept && binding.kind == BindingKind::Parameter
        {
            self.graph.entity(binding.entity).ty
        } else {
            binding.ty
        };
        let ty = ty.ok_or(Error::Invariant("expression binding without type"))?;
        Ok(
            if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
                element
            } else {
                ty
            },
        )
    }
    fn operand_type(&mut self, expr: ast::ExprId, scope: ScopeId) -> Result<TypeId, Error> {
        let mut inner = expr;
        while let ExprKind::Paren(id) = self.tree.expressions.get(inner).kind {
            inner = id;
        }
        if let ExprKind::Id(name) = self.tree.expressions.get(inner).kind {
            let binding = self.lookup(name, scope, Filter::Ordinary)?;
            if matches!(
                self.graph.binding(binding).kind,
                BindingKind::Type
                    | BindingKind::Alias
                    | BindingKind::Variable
                    | BindingKind::Parameter
                    | BindingKind::Function
            ) {
                return self.binding_expression_type(binding);
            }
        }
        if matches!(self.tree.expressions.get(inner).kind, ExprKind::Call { .. }) {
            // The frozen constant evaluator does not type a call in a
            // template definition. Its retained bound will type the copied
            // call, including required defaults, on concrete substitution.
            if self.template_depth != 0 {
                return Err(Error::NonConstant {
                    failure: super::ConstantFailure::Reported,
                    location: self.tree.expressions.get(inner).location,
                    message: "not an integral constant expression",
                });
            }
            return self.query_call_type(inner, scope);
        }
        if self.evaluating_noexcept {
            return self.exception_operand_type(expr, scope);
        }
        self.evaluate_constant(expr, scope, true)
            .map(|value| value.ty)
    }
    pub(super) fn constant_query(
        &mut self,
        ty: Option<ast::TypeId>,
        operand: Option<ast::ExprId>,
        scope: ScopeId,
        alignment: bool,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let ty = if let Some(ty) = ty {
            self.type_syntax(ty, scope)?
        } else {
            self.operand_type(
                operand.ok_or(Error::Invariant("query without operand"))?,
                scope,
            )?
        };
        let mut ty = if let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind {
            element
        } else {
            ty
        };
        if self.template_depth != 0 && self.exception_dependent_type(ty) {
            return Err(Error::NonConstant {
                failure: super::ConstantFailure::Reported,
                location,
                message: if alignment {
                    "a value-dependent alignof"
                } else {
                    "a value-dependent sizeof"
                },
            });
        }
        if alignment {
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
        }
        if self.evaluating_noexcept && !self.query_complete(ty)? {
            return Err(Error::at(
                location,
                if alignment {
                    "alignof of an incomplete or function type"
                } else {
                    "sizeof of an incomplete or function type"
                },
            ));
        }
        let (size, align) = self.type_size(ty, location)?;
        Ok(Constant {
            high: 0,
            bits: if alignment { align } else { size },
            ty: self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?,
        })
    }
    pub(super) fn query_complete(&mut self, mut ty: TypeId) -> Result<bool, Error> {
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.graph.types.get(ty).kind
        {
            if operand.is_some() {
                return Ok(false);
            }
            if bound == 0 {
                return Ok(false);
            }
            ty = element;
        }
        Ok(match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Void) | TypeKind::Function { .. } => false,
            TypeKind::Vector {
                element, operand, ..
            } => operand.is_none() && !self.graph.dependent_type(element),
            TypeKind::Class(entity) => self.complete_class(entity)?,
            TypeKind::Enum(entity) => self
                .graph
                .entity(entity)
                .enumeration
                .is_some_and(|info| info.underlying.is_some()),
            _ => true,
        })
    }
    // layout.cpp diagnoses a dependent size in ordinary semantic analysis;
    // the constant evaluator retains its distinct non-constant result.
    fn semantic_type_size(
        &mut self,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        self.type_size(ty, location).map_err(|error| match error {
            Error::NonConstant {
                location,
                message: "size of a dependent type",
                ..
            } if self.template_depth == 0 => {
                Error::at(location, "size of a dependent type is not supported here")
            }
            error => error,
        })
    }
    pub(super) fn type_size(
        &mut self,
        mut ty: TypeId,
        location: SourceOffset,
    ) -> Result<(u64, u64), Error> {
        let mut count = Some(1u64);
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.graph.types.get(ty).kind
        {
            if operand.is_some() {
                return Err(Error::NonConstant {
                    failure: super::ConstantFailure::Reported,
                    location,
                    message: "size of a dependent type",
                });
            }
            if bound == 0 {
                return Err(Error::at(location, "size of an array of unknown bound"));
            }
            // C++ checks an element's size before multiplication overflow.
            // Remember overflow while descending; diagnose the element first.
            count = count.and_then(|count| count.checked_mul(bound));
            ty = element;
        }
        let atomic = self.graph.types.get(ty).cv.0 & super::types::Cv::ATOMIC.0 != 0;
        while let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            ty = self
                .graph
                .entity(entity)
                .enumeration
                .and_then(|info| info.underlying)
                .ok_or_else(|| Error::at(location, "size of an incomplete enumeration"))?;
        }
        let mut alignment = None;
        let mut size = match self.graph.types.get(ty).kind {
            TypeKind::Vector {
                element,
                bound,
                operand,
                ..
            } => {
                if operand.is_some() {
                    return Err(Error::NonConstant {
                        failure: super::ConstantFailure::Reported,
                        location,
                        message: "size of a dependent type",
                    });
                }
                let bytes = if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Scalar(Scalar::Bool)
                ) {
                    bound.div_ceil(8)
                } else {
                    self.type_size(element, location)?
                        .0
                        .checked_mul(bound)
                        .ok_or(Error::Limit("target vector size"))?
                };
                let size = bytes
                    .checked_next_power_of_two()
                    .ok_or(Error::Limit("target vector size"))?;
                alignment = Some(size);
                size
            }
            TypeKind::Complex(element) => {
                let (size, align) = self.type_size(element, location)?;
                alignment = Some(align);
                size.checked_mul(2)
                    .ok_or(Error::Limit("target complex size"))?
            }
            TypeKind::Auto => {
                return Err(Error::Invariant("layout requested before auto deduction"));
            }
            TypeKind::Scalar(Scalar::Basic(scalar)) => {
                u64::try_from(scalar.size()).map_err(|_| Error::Limit("target scalar size"))?
            }
            TypeKind::Scalar(Scalar::Void) => return Err(Error::at(location, "size of void")),
            TypeKind::Scalar(Scalar::Bool | Scalar::SignedChar | Scalar::UnsignedChar) => 1,
            TypeKind::Scalar(Scalar::Short | Scalar::UnsignedShort) => 2,
            TypeKind::Scalar(Scalar::Nullptr)
            | TypeKind::Pointer(_)
            | TypeKind::BlockPointer(_)
            | TypeKind::Reference { .. } => 8,
            TypeKind::MemberPointer { element, .. } => {
                alignment = Some(8);
                if matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) {
                    16
                } else {
                    8
                }
            }
            TypeKind::Function { .. } => {
                return Err(Error::at(location, "size of a function type"));
            }
            TypeKind::Class(entity) => {
                if !self.complete_class(entity)? {
                    return Err(Error::at(location, "size of an incomplete class"));
                }
                let (size, align) = self.class_layout(entity, location)?;
                alignment = Some(align);
                size
            }
            TypeKind::Template(_) | TypeKind::Value { .. } | TypeKind::Address { .. } => {
                return Err(Error::Invariant("size query on a template value"));
            }
            TypeKind::Enum(_) => return Err(Error::Invariant("enum query not unwound")),
            TypeKind::Array { .. } => return Err(Error::Invariant("array query not unwound")),
            TypeKind::TemplateParameter { .. }
            | TypeKind::Pack(_)
            | TypeKind::Expansion(_)
            | TypeKind::BoundPack { .. } => {
                return Err(Error::NonConstant {
                    failure: super::ConstantFailure::Reported,
                    location,
                    message: "size of a dependent type",
                });
            }
        };
        if atomic && size <= 16 {
            size = size.max(1).next_power_of_two();
            alignment = Some(size);
        }
        Ok((
            count
                .and_then(|count| size.checked_mul(count))
                .ok_or_else(|| Error::at(location, "array too large"))?,
            alignment.unwrap_or(size),
        ))
    }
    pub(super) fn query_call_type(
        &mut self,
        id: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        if self.evaluating_noexcept
            && let Some(ty) = self.graph.expression_type(id)
        {
            self.graph.call_binding(id)?;
            return Ok(ty);
        }
        let node = self.tree.expressions.get(id);
        let location = node.location;
        let ExprKind::Call {
            mut callee,
            arguments,
        } = node.kind
        else {
            return Err(Error::Invariant("call query without call"));
        };
        while let ExprKind::Paren(inner) = self.tree.expressions.get(callee).kind {
            callee = inner;
        }
        let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
            return Err(Error::Missing("unevaluated indirect call typing"));
        };
        let binding = if self.evaluating_noexcept
            && let Some(binding) = self.graph.resolved_name(name)
        {
            binding
        } else {
            self.lookup(name, scope, Filter::Ordinary)
                .map_err(|error| match error {
                    Error::Diagnostic {
                        location,
                        message: "undeclared name",
                    } if self.evaluating_noexcept => {
                        Error::at(location, "undeclared function name")
                    }
                    other => other,
                })?
        };
        if matches!(
            self.graph.binding(binding).kind,
            BindingKind::Type | BindingKind::Alias
        ) {
            return Err(Error::Missing(
                "unevaluated class construction and functional casts",
            ));
        }
        let mut ty = self.binding_expression_type(binding)?;
        if let TypeKind::Pointer(element) = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Function {
            result,
            ref parameters,
            ..
        } = self.graph.types.get(ty).kind
        else {
            return Err(Error::at(
                location,
                "called object is not a function or pointer to function",
            ));
        };
        if !arguments.is_empty() || !parameters.is_empty() {
            return Err(Error::Missing("call argument conversions and defaults"));
        }
        super::record(&mut self.graph.expression_types, callee.index(), ty);
        super::record(&mut self.graph.expression_types, id.index(), result);
        self.graph.record_call_binding(id, binding);
        Ok(result)
    }
    pub(super) fn decltype_type(
        &mut self,
        operand: ast::ExprId,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let mut bare = operand;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(bare).kind {
            bare = inner;
        }
        if self.template_depth != 0
            && !matches!(self.tree.expressions.get(bare).kind, ExprKind::Id(_))
        {
            return self.dependent_operand(operand, scope, self.semantic());
        }
        if self.semantic() {
            let mut inner = operand;
            while let ExprKind::Paren(expr) = self.tree.expressions.get(inner).kind {
                inner = expr;
            }
            if !matches!(self.tree.expressions.get(inner).kind, ExprKind::Id(_)) {
                // Only the final call of a decltype operand leaves its class
                // result incomplete; argument calls and comma prefixes do not.
                let mut tail = inner;
                loop {
                    tail = match self.tree.expressions.get(tail).kind {
                        ExprKind::Paren(inner) => inner,
                        ExprKind::Binary { op, right, .. }
                            if op.token.as_punctuator() == Some(P::Comma) =>
                        {
                            right
                        }
                        _ => break,
                    };
                }
                let saved_call = self.decltype_call;
                self.decltype_call = matches!(
                    self.tree.expressions.get(tail).kind,
                    ExprKind::Call { .. }
                        | ExprKind::TypeTrait { .. }
                        | ExprKind::Typeof { .. }
                        | ExprKind::TypeTransform { .. }
                        | ExprKind::TypePredicate { .. }
                        | ExprKind::OffsetOf { .. }
                        | ExprKind::OffsetOfBase(_)
                )
                .then_some(tail);
                self.unevaluated += 1;
                let result = self.typed_expression(operand, scope).and_then(|facts| {
                    self.check_resolved_function(operand)?;
                    Ok(facts)
                });
                self.unevaluated -= 1;
                self.decltype_call = saved_call;
                let facts = result?;
                let mut tail = self.graph.expression_facts.root(operand);
                while let ExprKind::Binary { op, right, .. } = self.tree.expressions.get(tail).kind
                {
                    if op.token.as_punctuator() != Some(P::Comma) {
                        break;
                    }
                    tail = self.graph.expression_facts.root(right);
                }
                if !matches!(
                    self.tree.expressions.get(tail).kind,
                    ExprKind::Call { .. }
                        | ExprKind::TypeTrait { .. }
                        | ExprKind::Typeof { .. }
                        | ExprKind::TypeTransform { .. }
                        | ExprKind::TypePredicate { .. }
                        | ExprKind::OffsetOf { .. }
                        | ExprKind::OffsetOfBase(_)
                ) {
                    self.check_temporary_destruction(
                        facts,
                        scope,
                        self.tree.expressions.get(operand).location,
                    )?;
                }
                if matches!(
                    self.tree.expressions.get(operand).kind,
                    ExprKind::Member { .. }
                ) && let Some(binding) = facts.member
                    && self.graph.binding(binding).kind == BindingKind::Variable
                {
                    return self
                        .graph
                        .entity(self.graph.binding(binding).entity)
                        .ty
                        .ok_or(Error::Invariant("decltype member without declared type"));
                }
                return match facts.category {
                    super::expression_facts::ValueCategory::Prvalue => Ok(facts.ty),
                    super::expression_facts::ValueCategory::Lvalue => {
                        self.graph.types.reference(facts.ty, RefKind::Lvalue)
                    }
                    super::expression_facts::ValueCategory::Xvalue => {
                        self.graph.types.reference(facts.ty, RefKind::Rvalue)
                    }
                };
            }
        }
        let mut expr = operand;
        let mut parenthesized = false;
        while let ExprKind::Paren(inner) = self.tree.expressions.get(expr).kind {
            expr = inner;
            parenthesized = true;
        }
        if let ExprKind::Id(name) = self.tree.expressions.get(expr).kind {
            let result = self.lookup_result(name, scope, Filter::Ordinary)?;
            if result.functions.as_slice().len() > 1 {
                return Err(Error::at(
                    self.tree.expressions.get(expr).location,
                    "decltype of an overloaded function",
                ));
            }
            let found = result.binding.ok_or_else(|| {
                Error::at(self.tree.expressions.get(expr).location, "undeclared name")
            })?;
            let binding = self.graph.binding(found);
            if self.template_depth != 0
                && (self.graph.dependent_members.get(binding.entity).is_some()
                    || self
                        .graph
                        .entity(binding.entity)
                        .template_parameter
                        .is_some())
            {
                return self.dependent_operand(operand, scope, false);
            }
            let ty = match binding.kind {
                BindingKind::Variable => self.graph.entity(binding.entity).ty,
                BindingKind::Enumerator => {
                    if let Some(argument) = self.graph.constant_addresses.argument(binding.entity)
                        && let TypeKind::Address { ty, .. } = self.graph.types.get(argument).kind
                    {
                        return Ok(ty);
                    }
                    let super::constants::ConstantState::Value(value) =
                        self.graph.entity(binding.entity).constant
                    else {
                        return Err(Error::Invariant("enumerator without value"));
                    };
                    return Ok(value.ty);
                }
                BindingKind::Parameter | BindingKind::TemplateValue => binding.ty,
                BindingKind::Function => {
                    let ty = self
                        .graph
                        .entity(binding.entity)
                        .ty
                        .ok_or(Error::Invariant("function without type"))?;
                    Some(self.graph.types.canonical_function(ty)?)
                }
                _ => {
                    return Err(Error::at(
                        self.tree.expressions.get(expr).location,
                        "decltype operand is not an expression",
                    ));
                }
            }
            .ok_or(Error::Invariant("decltype binding without type"))?;
            if self.template_depth != 0 && self.exception_dependent_type(ty) {
                return self.dependent_operand(operand, scope, false);
            }
            return if parenthesized {
                let ty = if matches!(binding.kind, BindingKind::Variable | BindingKind::Parameter) {
                    self.captured_type(binding.entity, ty, true)?
                } else {
                    ty
                };
                self.graph.types.reference(ty, RefKind::Lvalue)
            } else {
                Ok(ty)
            };
        }
        match self.evaluate_constant(operand, scope, true) {
            Ok(value) => Ok(value.ty),
            Err(Error::NonConstant { .. }) => Err(Error::UnconditionalDiagnostic {
                location: self.tree.expressions.get(operand).location,
                message: "unsupported decltype operand",
            }),
            Err(error) => Err(error),
        }
    }
}
