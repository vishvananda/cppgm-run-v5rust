// Translated from sema/analyzer_constexpr.cpp. See NOTICE.
//! Literal-type queries belong to canonical classes; constructor queries are query-local.
use super::{
    Error,
    analyzer::Analyzer,
    construction::ConstructorId,
    graph::{EntityId, FunctionAvailability},
    special_members::SpecialDefinition,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, ExprKind},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Default)]
pub(super) enum LiteralState {
    #[default]
    NotStarted,
    InProgress,
    Complete(bool),
}
impl Analyzer<'_> {
    pub(super) fn check_constexpr_variable(
        &mut self,
        is_constexpr: bool,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.semantic()
            && self.template_depth == 0
            && is_constexpr
            && !self.constant_literal_type(ty)?
        {
            return Err(Error::at(
                location,
                "a constexpr variable of a type that is not a literal type",
            ));
        }
        Ok(())
    }
    pub(super) fn constant_literal_type(&mut self, mut ty: TypeId) -> Result<bool, Error> {
        self.check_stack()?;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
            return Ok(true);
        }
        while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
            ty = element;
        }
        let TypeKind::Class(class) = self.graph.types.get(ty).kind else {
            return Ok(!matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            ));
        };
        match self.graph.entity(class).literal {
            LiteralState::Complete(value) => return Ok(value),
            LiteralState::InProgress => return Ok(true),
            LiteralState::NotStarted => (),
        }
        if !self.complete_class(class)? || self.graph.entity(class).scope.is_none() {
            return Ok(false);
        }
        self.graph.entity_mut(class).literal = LiteralState::InProgress;
        let result = self.constant_literal_class(class);
        self.graph.entity_mut(class).literal = match result {
            Ok(value) => LiteralState::Complete(value),
            Err(_) => LiteralState::NotStarted,
        };
        result
    }
    fn constant_literal_class(&mut self, class: EntityId) -> Result<bool, Error> {
        if self.graph.lower_nontrivial_destructor(class)? {
            return Ok(false);
        }
        for index in 0..self.graph.entity(class).bases.len() {
            if self.graph.entity(class).bases[index].is_virtual {
                return Ok(false);
            }
            let base = self.graph.entity(class).bases[index].class;
            if !self.constant_literal_type(self.graph.lower_entity_type(base)?)? {
                return Ok(false);
            }
        }
        for index in 0..self.graph.entity(class).members.len() {
            let member = self.graph.entity(class).members[index];
            let mut ty = self.graph.lower_entity_type(member)?;
            if !self.constant_literal_type(ty)? {
                return Ok(false);
            }
            if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    ty = element;
                }
                if self.type_cv(ty).0 & 2 != 0 {
                    return Ok(false);
                }
            }
        }
        if self.graph.aggregate_class(class) {
            return Ok(true);
        }
        for index in 0..self.graph.entity(class).declared_constructors.len() {
            let binding = self.graph.entity(class).declared_constructors[index];
            let function = self.graph.binding(binding).entity;
            if self.graph.entity(function).copy_kind.is_some() {
                continue;
            }
            if self.graph.entity(function).constexpr {
                return Ok(true);
            }
            if self.graph.inherited_constructors.source(function).is_some() {
                let plan = self.declared_constructor_plan(binding)?;
                if self.constant_constructor_eligible(plan, &mut HashMap::new())? {
                    return Ok(true);
                }
            }
        }
        if self.graph.entity(class).user_constructor.is_some() {
            return Ok(false);
        }
        // An unavailable/deleted implicit default is a negative suitability
        // query, not a use of that constructor. Actual initialization retains
        // its diagnostic path.
        let id = match self.default_constructor(class, SourceOffset(0)) {
            Ok(id) => id,
            Err(Error::Diagnostic { .. }) => return Ok(false),
            Err(error) => return Err(error),
        };
        self.constant_constructor_eligible(id, &mut HashMap::new())
    }
    pub(super) fn constant_constructor_eligible(
        &mut self,
        id: ConstructorId,
        known: &mut HashMap<ConstructorId, bool>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        if let Some(&value) = known.get(&id) {
            return Ok(value);
        }
        known.insert(id, false);
        let result = self.constant_constructor_eligible_plan(id, known)?;
        known.insert(id, result);
        Ok(result)
    }
    fn constant_constructor_eligible_plan(
        &mut self,
        id: ConstructorId,
        known: &mut HashMap<ConstructorId, bool>,
    ) -> Result<bool, Error> {
        let plan = self.graph.constructors.get(id);
        if plan.deleted {
            return Ok(false);
        }
        if let Some(binding) = plan.binding {
            let function = self.graph.entity(self.graph.binding(binding).entity);
            if self
                .graph
                .inherited_constructors
                .source(self.graph.binding(binding).entity)
                .is_none()
                && function.copy_origin != super::copy_construction::Origin::Implicit
                && !matches!(
                    function.special_definition,
                    Some(SpecialDefinition::Defaulted { first: true, .. })
                )
            {
                return Ok(function.constexpr);
            }
        }
        let class = plan.class;
        if self
            .graph
            .entity(class)
            .bases
            .iter()
            .any(|edge| edge.is_virtual)
        {
            return Ok(false);
        }
        if let Some(copy) = plan.copy.as_ref() {
            let count = copy.len();
            for index in 0..count {
                if let Some(child) = self
                    .graph
                    .constructors
                    .get(id)
                    .copy
                    .as_ref()
                    .ok_or(Error::Invariant("lost constant copy plan"))?[index]
                    .constructor
                    && !self.constant_constructor_eligible(child, known)?
                {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        for index in 0..self.graph.constructors.get(id).bases.len() {
            if let Some(base) = self.graph.constructors.get(id).bases[index].constructor
                && !self.constant_constructor_eligible(base, known)?
            {
                return Ok(false);
            }
        }
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut selected_count = 0;
        let mut next_subobject = 0;
        for index in 0..self.graph.constructors.get(id).members.len() {
            let owner = self.graph.constructors.get(id);
            let (member, initializer) = owner.members[index];
            let child = owner
                .subobjects
                .get(next_subobject)
                .filter(|plan| plan.member == member)
                .map(|plan| plan.constructor);
            if child.is_some() {
                next_subobject += 1;
            }
            if self.graph.unnamed_bit_field(member) {
                continue;
            }
            let anonymous = self
                .graph
                .entity(member)
                .ty
                .and_then(|ty| self.graph.class_of(ty))
                .is_some_and(|class| self.graph.anonymous_member_storage(class).is_some());
            let selected = initializer.is_some()
                || anonymous && child.is_some_and(|child| self.constant_plan_initializes(child));
            selected_count += usize::from(selected);
            if union && !selected {
                continue;
            }
            if anonymous {
                if let Some(child) = child
                    && !self.constant_constructor_eligible(child, known)?
                {
                    return Ok(false);
                }
                continue;
            }
            if initializer.is_some() {
                continue;
            }
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            let Some(class) = self.graph.class_of(ty) else {
                return Ok(false);
            };
            let child = match child {
                Some(child) => child,
                None => self.default_constructor(class, SourceOffset(0))?,
            };
            if !self.constant_constructor_eligible(child, known)? {
                return Ok(false);
            }
        }
        if union && !self.graph.entity(class).members.is_empty() && selected_count != 1 {
            return Ok(false);
        }
        if self
            .graph
            .constructors
            .get(id)
            .binding
            .is_some_and(|binding| {
                self.graph
                    .inherited_constructors
                    .source(self.graph.binding(binding).entity)
                    .is_some()
            })
        {
            return self.constant_inherited_member_initializers(id, known);
        }
        Ok(true)
    }
    pub(super) fn constant_plan_initializes(&self, id: ConstructorId) -> bool {
        let mut work = vec![id];
        while let Some(id) = work.pop() {
            let plan = self.graph.constructors.get(id);
            if plan
                .members
                .iter()
                .any(|(_, initializer)| initializer.is_some())
            {
                return true;
            }
            work.extend(
                plan.subobjects
                    .iter()
                    .filter(|child| {
                        self.graph
                            .entity(child.member)
                            .ty
                            .and_then(|ty| self.graph.class_of(ty))
                            .is_some_and(|class| {
                                self.graph.anonymous_member_storage(class).is_some()
                            })
                    })
                    .map(|child| child.constructor),
            );
        }
        false
    }
    fn constant_inherited_member_initializers(
        &mut self,
        id: ConstructorId,
        known: &mut HashMap<ConstructorId, bool>,
    ) -> Result<bool, Error> {
        for index in 0..self.graph.constructors.get(id).subobjects.len() {
            let child = self.graph.constructors.get(id).subobjects[index].constructor;
            if self.graph.constructors.get(child).nontrivial()
                && !self.constant_constructor_eligible(child, known)?
            {
                return Ok(false);
            }
        }
        for index in 0..self.graph.constructors.get(id).members.len() {
            let (member, initializer) = self.graph.constructors.get(id).members[index];
            let Some(initializer) = initializer else {
                continue;
            };
            let (ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr)) =
                initializer.kind
            else {
                continue;
            };
            if !self.graph.member_initializers.typed(member)
                || self.constant_initializer_uses_object(expr, 0)
            {
                continue;
            }
            let scope = self
                .graph
                .entity(member)
                .home
                .ok_or(Error::Invariant("member without home"))?;
            let result = self.in_constant_evaluation(|analyzer| {
                let region = analyzer.constexpr_frames.region();
                let result = analyzer.probe_constant_member_initializer(member, initializer, scope);
                analyzer.constexpr_frames.release(region);
                result
            });
            match result {
                Ok(()) => (),
                Err(Error::NonConstant { .. }) => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }
    fn probe_constant_member_initializer(
        &mut self,
        member: EntityId,
        initializer: ast::Initializer,
        scope: super::graph::ScopeId,
    ) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(member)?;
        let frame = super::constexpr_frames::Frame::new(ty, scope);
        self.with_constant_frame(frame, |analyzer| {
            let place = analyzer.allocate_constant_object(ty, None, initializer.location)?;
            let constructor = analyzer.graph.entity(member).constructor;
            analyzer.construct_constant_object(
                place,
                Some(initializer),
                constructor,
                scope,
                initializer.location,
            )?;
            for cell in analyzer
                .constexpr_frames
                .active_cells(place, initializer.location)?
            {
                let child = analyzer.constexpr_frames.cell_place(place, cell);
                if !matches!(
                    analyzer.graph.types.get(child.designated).kind,
                    TypeKind::Class(_) | TypeKind::Array { .. }
                ) {
                    analyzer
                        .constexpr_frames
                        .read(child, initializer.location)?;
                }
            }
            Ok(())
        })
    }
    // analyzer_constexpr.cpp::UsesObject: conservatively defer forms outside
    // this bounded syntax walk to evaluation with a constructing object.
    fn constant_initializer_uses_object(&self, expr: ast::ExprId, depth: u32) -> bool {
        if depth > 64
            || self.graph.expression_facts.get(expr).is_ok_and(|facts| {
                facts.object.is_some_and(|object| {
                    matches!(
                        object.source,
                        super::expression_facts::ObjectSource::Implicit
                    )
                })
            })
        {
            return true;
        }
        let uses = |expr| self.constant_initializer_uses_object(expr, depth + 1);
        match self.tree.expressions.get(expr).kind {
            ExprKind::Id(_)
            | ExprKind::Literal(_)
            | ExprKind::Sizeof { .. }
            | ExprKind::SizeofPack { .. }
            | ExprKind::TypeTrait { .. }
            | ExprKind::Typeof { .. }
            | ExprKind::TypeTransform { .. }
            | ExprKind::TypePredicate { .. }
            | ExprKind::OffsetOf { .. }
            | ExprKind::OffsetOfBase(_)
            | ExprKind::BuiltinValue { .. }
            | ExprKind::VaArg { .. } => false,
            ExprKind::Keyword(keyword) => keyword == Keyword::This,
            ExprKind::Paren(inner) => uses(inner),
            ExprKind::Binary { left, right, .. } => uses(left) || uses(right),
            ExprKind::ComplexPart { operand, .. }
            | ExprKind::Unary { operand, .. }
            | ExprKind::Postfix { operand, .. }
            | ExprKind::Cast { operand, .. } => uses(operand),
            ExprKind::Conditional {
                condition,
                then_expr,
                else_expr,
            } => uses(condition) || uses(then_expr) || uses(else_expr),
            ExprKind::Subscript { base, index } => uses(base) || uses(index),
            ExprKind::Member { base, .. } => uses(base),
            ExprKind::Call { callee, arguments } => {
                uses(callee)
                    || self
                        .tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .copied()
                        .any(uses)
            }
            ExprKind::FunctionalCast {
                arguments, braced, ..
            } => braced.map_or_else(
                || {
                    self.tree
                        .expr_lists
                        .get(arguments)
                        .iter()
                        .copied()
                        .any(uses)
                },
                uses,
            ),
            ExprKind::Braced { elements, .. } => {
                self.tree.expr_lists.get(elements).iter().copied().any(uses)
            }
            _ => true,
        }
    }
    pub(super) fn check_constant_constructor_plan(
        &mut self,
        function: EntityId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let entity = self.graph.entity(function);
        if !entity.constexpr
            || entity.availability == FunctionAvailability::Deleted
            || entity.template_head.is_some()
            || entity
                .special_definition
                .is_some_and(|special| matches!(special, SpecialDefinition::Defaulted { .. }))
            || entity
                .home
                .is_some_and(|home| self.graph.within_class_instance(home))
        {
            return Ok(());
        }
        let id = entity
            .constructor
            .ok_or(Error::Invariant("constexpr constructor without plan"))?;
        if let Some(delegation) = self.graph.constructors.get(id).delegation {
            if !self.constant_constructor_eligible(delegation.constructor, &mut HashMap::new())? {
                return Err(Error::at(
                    location,
                    "a constexpr constructor delegates to a constructor that is not constexpr",
                ));
            }
            return Ok(());
        }
        if self
            .graph
            .entity(self.graph.constructors.get(id).class)
            .bases
            .iter()
            .any(|edge| edge.is_virtual)
        {
            return Err(Error::at(
                location,
                "a constexpr constructor of a class with a virtual base",
            ));
        }
        for index in 0..self.graph.constructors.get(id).bases.len() {
            let base = self.graph.constructors.get(id).bases[index];
            if let Some(child) = base.constructor
                && !self.graph.constructors.get(child).deleted
                && !self.constant_trivial_value_initialization(child, base.initializer)
                && !self.constant_constructor_eligible(child, &mut HashMap::new())?
            {
                return Err(Error::at(
                    location,
                    "a constexpr constructor initializes a subobject by a constructor that is not constexpr",
                ));
            }
        }
        self.check_constant_member_plan(id, false, location)
    }
    fn check_constant_member_plan(
        &mut self,
        id: ConstructorId,
        variant: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let class = self.graph.constructors.get(id).class;
        let union = self.graph.entity(class).key == Keyword::Union;
        let mut selected_count = 0;
        let mut next_subobject = 0;
        for index in 0..self.graph.constructors.get(id).members.len() {
            let owner = self.graph.constructors.get(id);
            let (member, initializer) = owner.members[index];
            let child = owner
                .subobjects
                .get(next_subobject)
                .filter(|plan| plan.member == member)
                .map(|plan| plan.constructor);
            if child.is_some() {
                next_subobject += 1;
            }
            let anonymous = self
                .graph
                .entity(member)
                .ty
                .and_then(|ty| self.graph.class_of(ty))
                .is_some_and(|class| self.graph.anonymous_member_storage(class).is_some());
            let selected = initializer.is_some()
                || anonymous && child.is_some_and(|child| self.constant_plan_initializes(child));
            selected_count += usize::from(selected);
            if union && !selected || self.graph.unnamed_bit_field(member) {
                continue;
            }
            let written = owner.written_members[index]
                .then_some(initializer)
                .flatten();
            if !anonymous
                && let Some(child) = child
                && !self.graph.constructors.get(child).deleted
                && !self.constant_trivial_value_initialization(child, written)
                && !self.constant_constructor_eligible(child, &mut HashMap::new())?
            {
                return Err(Error::at(
                    location,
                    "a constexpr constructor initializes a subobject by a constructor that is not constexpr",
                ));
            }
            if anonymous {
                if (!variant || selected)
                    && let Some(child) = child
                {
                    self.check_constant_member_plan(child, union || variant, location)?;
                }
                continue;
            }
            if initializer.is_some() || union || variant {
                continue;
            }
            let mut ty = self.graph.lower_entity_type(member)?;
            while let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                ty = element;
            }
            if self.graph.class_of(ty).is_none() {
                return Err(Error::at(
                    location,
                    "a constexpr constructor leaves a non-static data member uninitialized",
                ));
            }
        }
        if union && selected_count == 0 && !self.graph.entity(class).members.is_empty() {
            return Err(Error::at(
                location,
                if self.graph.anonymous_member_storage(class).is_some() {
                    "a constexpr constructor initializes no member of an anonymous union"
                } else {
                    "a constexpr constructor of a union initializes no member"
                },
            ));
        }
        Ok(())
    }
    fn constant_trivial_value_initialization(
        &self,
        constructor: ConstructorId,
        initializer: Option<ast::Initializer>,
    ) -> bool {
        !self.graph.constructors.get(constructor).nontrivial()
            && initializer.is_some_and(|initializer| match initializer.kind {
                ast::InitializerKind::Parenthesized(values) => self.tree.expr_lists.get(values).is_empty(),
                ast::InitializerKind::Braced(expr) => matches!(self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { elements, .. } if self.tree.expr_lists.get(elements).is_empty()),
                _ => false,
            })
    }
}
