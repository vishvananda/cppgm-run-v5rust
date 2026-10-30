// Translated from class_templates.cpp::link_member/instantiate_variable and
// analyzer_declarations.cpp::CompatibleVariableTypes/declare_variable. See NOTICE.
//! Static member patterns, initializer facts and evaluated storage demands have
//! independent unit-owned identities. Missing definitions can arrive later.
use super::{
    Error,
    analyzer::Analyzer,
    constants::{Constant, ConstantState},
    declarators::DeclaratorInfo,
    graph::{BindingId, EntityId, ScopeId},
    templates::HeadId,
    types::{TypeId, TypeKind},
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::{support::source::SourceOffset, syntax::ast};

#[derive(Clone, Copy)]
pub(super) struct Pattern {
    pub declarator: ast::DeclaratorId,
    pub initializer: Option<ast::Initializer>,
    pub ty: TypeId,
    pub head: Option<HeadId>,
    pub scope: ScopeId,
    pub location: SourceOffset,
    pub limit: usize,
    pub auto_cv: super::types::Cv,
    pub auto_type: Option<TypeId>,
    pub qualified: bool,
}
#[derive(Clone, Copy)]
pub struct Definition {
    pub initializer: Option<ast::Initializer>,
}
#[derive(Clone, Copy)]
enum State {
    InProgress,
    Complete(Definition),
}
#[derive(Default)]
pub(super) struct Variables {
    pub(super) templates: super::variable_specializations::Specializations,
    pub(super) partial_lists: super::partial_specializations::PartialLists,
    patterns: HashMap<EntityId, Pattern>,
    // Declaration order is independent of any specialization's later demand.
    partials: HashMap<EntityId, Vec<(Vec<TypeId>, EntityId)>>,
    partial_declarations: Vec<Option<EntityId>>,
    deferred_declarations: HashSet<ast::DeclaratorId>,
    links: HashMap<EntityId, EntityId>,
    states: HashMap<EntityId, State>,
    demands: Vec<EntityId>,
    seen: HashSet<EntityId>,
    explicit_definitions: HashSet<EntityId>,
    initializers: Vec<EntityId>,
    queued_initializers: HashSet<EntityId>,
    layout_uses: Vec<(EntityId, Option<EntityId>)>,
    layout_seen: HashSet<(EntityId, Option<EntityId>)>,
    next_layout: usize,
    initializer_uses: HashMap<EntityId, Vec<EntityId>>,
    initializer_seen: HashSet<(EntityId, EntityId)>,
    next: usize,
}
impl Variables {
    pub(super) fn explicit_definition(&self, entity: EntityId) -> bool {
        self.explicit_definitions.contains(&entity)
    }
    fn outside_definition(&self, entity: EntityId) -> Option<Pattern> {
        let mut source = *self.links.get(&entity)?;
        loop {
            if let Some(&pattern) = self.patterns.get(&source) {
                return Some(pattern);
            }
            // Copies of nested member class templates retain one source edge
            // per instantiation, independently of initializer demand.
            source = *self.links.get(&source)?;
        }
    }
    pub(super) fn pattern(&self, entity: EntityId) -> Option<Pattern> {
        self.patterns.get(&entity).copied()
    }
    pub(super) fn member_source(&self, entity: EntityId) -> Option<EntityId> {
        self.links.get(&entity).copied()
    }
    pub(super) fn partials(&self, primary: EntityId) -> &[(Vec<TypeId>, EntityId)] {
        self.partials.get(&primary).map_or(&[], Vec::as_slice)
    }
    pub(super) fn begin(&mut self, entity: EntityId) {
        self.states.insert(entity, State::InProgress);
    }
    pub(super) fn started(&self, entity: EntityId) -> bool {
        self.states.contains_key(&entity)
    }
    pub(super) fn finish(
        &mut self,
        entity: EntityId,
        result: Result<Definition, Error>,
    ) -> Result<(), Error> {
        match result {
            Ok(definition) => {
                self.states.insert(entity, State::Complete(definition));
            }
            Err(error) => {
                self.states.remove(&entity);
                return Err(error);
            }
        }
        Ok(())
    }
    pub(super) fn defer_declaration(&mut self, declarator: ast::DeclaratorId) {
        self.deferred_declarations.insert(declarator);
    }
    pub(super) fn nonobject_declaration(&self, declarator: ast::DeclaratorId) -> bool {
        self.partial_declaration(declarator).is_some()
            || self.deferred_declarations.contains(&declarator)
    }
    pub(super) fn record_partial(
        &mut self,
        primary: EntityId,
        arguments: Vec<TypeId>,
        partial: EntityId,
        declarator: ast::DeclaratorId,
        key: Option<super::partial_specializations::PartialIndex>,
    ) {
        let partials = self.partials.entry(primary).or_default();
        self.partial_lists.add(primary, key, partials.len());
        partials.push((arguments, partial));
        super::record(&mut self.partial_declarations, declarator.index(), partial);
    }
    pub(super) fn partial_declaration(&self, declarator: ast::DeclaratorId) -> Option<EntityId> {
        self.partial_declarations
            .get(declarator.index())
            .copied()
            .flatten()
    }
    pub(super) fn link(&mut self, source: EntityId, target: EntityId) {
        self.links.insert(target, source);
    }
    pub(super) fn instance(&self, entity: EntityId) -> bool {
        self.links.contains_key(&entity) || self.templates.contains(entity)
    }
    pub(super) fn demands(&self) -> &[EntityId] {
        &self.demands
    }
    pub(super) fn initializer_requests(&self) -> &[EntityId] {
        &self.initializers
    }
    fn request_initializer(&mut self, entity: EntityId) {
        if self.queued_initializers.insert(entity) {
            self.initializers.push(entity);
        }
    }
    fn demand(&mut self, entity: EntityId) {
        self.request_initializer(entity);
        if self.seen.insert(entity) {
            self.demands.push(entity);
        }
    }
    pub(super) fn initializer(&self, entity: EntityId) -> Option<Definition> {
        match self.states.get(&entity) {
            Some(State::Complete(initializer)) => Some(*initializer),
            _ => None,
        }
    }
    pub(super) fn omitted_initialization(&self, entity: EntityId) -> bool {
        self.templates.explicit(entity)
            && self
                .initializer(entity)
                .is_some_and(|d| d.initializer.is_none())
    }
    fn request_layout_use(&mut self, entity: EntityId, owner: Option<EntityId>) {
        if self.layout_seen.insert((entity, owner)) {
            self.layout_uses.push((entity, owner));
        }
    }
    fn record_initializer_use(&mut self, source: EntityId, target: EntityId) {
        if self.initializer_seen.insert((source, target)) {
            self.initializer_uses
                .entry(source)
                .or_default()
                .push(target);
        }
    }
}
impl super::Graph {
    #[inline]
    #[must_use]
    pub fn lower_variable_instantiated(&self, entity: EntityId) -> bool {
        self.variable_instances.member_source(entity).is_some()
            || self.variable_instances.templates.primary(entity).is_some()
    }
}
impl Analyzer<'_> {
    // template_specializations.cpp::explicit_variable_specialization. A
    // declaration without an initializer blocks primary-definition demand,
    // but leaves the same canonical entity available for its own definition.
    pub(super) fn explicit_member_variable(
        &mut self,
        info: &DeclaratorInfo,
        initialized: bool,
        location: SourceOffset,
    ) -> Result<Option<bool>, Error> {
        if !self.specializing() {
            return Ok(None);
        }
        let Some(home) = info.home else {
            return Err(Error::at(
                location,
                "an explicit specialization of a variable is not supported",
            ));
        };
        if !self.graph.within_class_instance(home) {
            return Ok(None);
        }
        let name = info
            .name
            .ok_or(Error::Invariant("static specialization without name"))?;
        let binding = self
            .qualified(home, name, super::lookup::Filter::Ordinary)?
            .ok_or(Error::at(
                location,
                "qualified definition does not match a declaration",
            ))?;
        let entity = self.graph.binding(binding).entity;
        self.specialization_mode = super::analyzer::SpecializationMode::Ordinary;
        if self.graph.variable_instances.states.contains_key(&entity) {
            return Err(Error::at(
                location,
                "an explicit specialization after the member's instantiation",
            ));
        }
        self.graph.variable_instances.links.remove(&entity);
        Ok(Some(initialized))
    }

    pub(super) fn retain_variable_template_pattern(
        &mut self,
        entity: EntityId,
        init: ast::InitDeclarator,
        info: &DeclaratorInfo,
        scope: ScopeId,
        location: SourceOffset,
        automatic: Option<(TypeId, super::types::Cv)>,
    ) {
        // A later declaration without an initializer keeps the definition's
        // parameter names, lookup scope and definition point.
        if init.initializer.is_none()
            && self.graph.variable_instances.patterns.contains_key(&entity)
        {
            return;
        }
        self.graph.variable_instances.patterns.insert(
            entity,
            Pattern {
                declarator: init.declarator,
                initializer: init.initializer,
                ty: info.ty,
                head: self.active_template_head(scope),
                scope: info.lookup,
                location,
                limit: self.graph.binding_limit(),
                auto_cv: automatic.map_or(super::types::Cv::default(), |(_, cv)| cv),
                auto_type: automatic.map(|(ty, _)| ty),
                qualified: info.home.is_some(),
            },
        );
    }
    pub(super) fn explicit_class_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        let variables = &self.graph.variable_instances;
        if !variables
            .links
            .get(&entity)
            .is_some_and(|source| variables.patterns.contains_key(source))
        {
            return Ok(());
        }
        self.instantiate_variable(entity)?;
        self.graph
            .variable_instances
            .explicit_definitions
            .insert(entity);
        self.graph.variable_instances.demand(entity);
        Ok(())
    }
    pub(super) fn compatible_variable_type(
        &mut self,
        old: TypeId,
        new: TypeId,
    ) -> Result<Option<TypeId>, Error> {
        let canonical_old = self.graph.types.canonical_function(old)?;
        let canonical_new = self.graph.types.canonical_function(new)?;
        let compatible = canonical_old == canonical_new
            || matches!((&self.graph.types.get(canonical_old).kind, &self.graph.types.get(canonical_new).kind),
                (TypeKind::Array { element: old_element, bound: old_bound, .. },
                 TypeKind::Array { element: new_element, bound: new_bound, .. })
                    if old_element == new_element && (*old_bound == 0 || *new_bound == 0));
        Ok(compatible.then_some(
            if matches!(
                self.graph.types.get(old).kind,
                TypeKind::Array { bound: 0, .. }
            ) {
                new
            } else {
                old
            },
        ))
    }
    pub(super) fn record_variable_pattern(
        &mut self,
        binding: BindingId,
        init: ast::InitDeclarator,
        info: &DeclaratorInfo,
        definition: bool,
        ty: TypeId,
        location: SourceOffset,
    ) {
        let entity = self.graph.binding(binding).entity;
        if self.template_depth == 0
            || !definition
            || info.home.is_none()
                && (self.graph.entity(entity).inline != super::specifiers::InlinePreference::Inline
                    || self.graph.entity(entity).context().template_head.is_some())
            || self.graph.entity(entity).object().data_member
                != Some(super::graph::DataMemberKind::Static)
        {
            return;
        }
        self.graph.variable_instances.patterns.insert(
            entity,
            Pattern {
                declarator: init.declarator,
                // An inline definition reuses the instance's already typed
                // in-class initializer instead of copying the pattern again.
                initializer: info.home.and(init.initializer),
                ty,
                head: info.home.and(self.template_head),
                scope: info.lookup,
                location,
                limit: self.graph.binding_limit(),
                auto_cv: super::types::Cv::default(),
                auto_type: None,
                qualified: info.home.is_some(),
            },
        );
    }
    pub(super) fn use_instance_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.deferring_type_query() {
            self.type_queries
                .uses
                .push(super::type_queries::DeferredUse::Variable(entity));
        }
        if self.template_depth != 0 && !self.graph.variable_instances.templates.concrete(entity) {
            return Ok(());
        }
        if !self.graph.variable_instances.instance(entity) {
            if self.graph.ordinary_inline_member(entity)
                && self.lowering()
                && self.unevaluated == 0
                && self.template_arguments == 0
            {
                self.instantiate_variable(entity)?;
                self.record_variable_storage_use(entity);
            }
            return Ok(());
        }
        if self.graph.variable_instances.templates.concrete(entity)
            && self.graph.entity(entity).ty.is_some_and(|ty| {
                self.integral_kind(ty).is_some() && self.graph.lower_cv(ty) & 1 != 0
                    || matches!(
                        self.graph.types.get(ty).kind,
                        TypeKind::Array {
                            bound: 0,
                            operand: None,
                            ..
                        }
                    )
            })
        {
            self.instantiate_variable(entity)?;
        }
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("static member without type"))?;
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array {
                bound: 0,
                operand: None,
                ..
            }
        ) || matches!(
            self.graph.entity(entity).object().constant,
            ConstantState::None
        ) && self.integral_kind(ty).is_some()
            && self.graph.lower_cv(ty) & 1 != 0
        {
            self.instantiate_variable(entity)?;
        }
        self.record_variable_storage_use(entity);
        // Fixed-type namespace and member specializations share the same
        // first evaluated-use queue. Only a type/constant demand above needs
        // the initializer now; eager storage initialization changes closure
        // identities when the two kinds of variables are interleaved.
        if self.unevaluated == 0 && self.template_arguments == 0 {
            self.graph.variable_instances.request_initializer(entity);
            // Class statics have the course's separate emission demand even
            // for constant reads. Namespace definitions are emitted only when
            // the lowered code/data actually refers to their storage.
            if self
                .graph
                .entity(entity)
                .home
                .is_some_and(|home| self.graph.scope(home).kind == super::graph::ScopeKind::Class)
            {
                self.graph.variable_instances.demand(entity);
            }
        }
        Ok(())
    }
    fn record_variable_storage_use(&mut self, entity: EntityId) {
        if !self.lowering()
            || self.template_depth != 0
            || self.unevaluated != 0
            || self.template_arguments != 0
        {
            return;
        }
        let owner = self.body_context.as_ref().and_then(|body| body.function);
        if let Some(initializing) = self.initializing {
            self.graph
                .variable_instances
                .record_initializer_use(initializing, entity);
            // A template initializer is not a storage root. Its final static
            // image (or dynamic dependencies) is followed only if it is used.
            if !self.graph.variable_instances.instance(initializing)
                && !self.graph.ordinary_inline_member(initializing)
                && self
                    .graph
                    .entity(initializing)
                    .context()
                    .template_head
                    .is_none()
                && self.graph.entity(initializing).object().data_member
                    != Some(super::graph::DataMemberKind::Instance)
            {
                self.graph
                    .variable_instances
                    .request_layout_use(initializing, owner);
            }
        } else if owner.is_some()
            && !(matches!(
                self.graph.entity(entity).object().constant,
                ConstantState::Value(_)
            ) && self
                .graph
                .entity(entity)
                .ty
                .is_some_and(|ty| self.integral_kind(ty).is_some() && !self.scoped_enum(ty)))
        {
            self.graph
                .variable_instances
                .request_layout_use(entity, owner);
        }
    }
    // Process only newly demanded storage after static images are ready. This
    // graph is keyed by object and demanding function: an unused inline body
    // cannot consume another function's layout demand. Constant results prune
    // incidental initializer uses that never become emitted addresses.
    pub(super) fn finish_variable_storage_layouts(&mut self) -> Result<(), Error> {
        use super::{static_arrays::Item, static_initialization::Value};
        while let Some(&(entity, owner)) = self
            .graph
            .variable_instances
            .layout_uses
            .get(self.graph.variable_instances.next_layout)
        {
            self.graph.variable_instances.next_layout += 1;
            if self.graph.lower_omitted_variable_initialization(entity)
                || self.graph.ordinary_inline_member(entity)
            {
                let location = if self.graph.ordinary_inline_member(entity) {
                    let declarator = self
                        .graph
                        .entity(entity)
                        .object()
                        .variable_definition
                        .ok_or(Error::Invariant("inline storage without declaration"))?;
                    self.tree.declarators.get(declarator).location
                } else {
                    self.graph
                        .variable_instances
                        .templates
                        .location(entity)
                        .ok_or(Error::Invariant("specialization storage without location"))?
                };
                let ty = self.graph.lower_entity_type(entity)?;
                self.request_object_layout_for(ty, location, owner);
            }
            if let Some(value) = self
                .graph
                .entity(entity)
                .initialization()
                .static_initializer
            {
                if let Value::Address(target, _) = value {
                    self.graph
                        .variable_instances
                        .request_layout_use(target, owner);
                }
            } else if let Some(image) = self.graph.array_images.get(entity) {
                let variables = &mut self.graph.variable_instances;
                for item in image {
                    if let Item::Scalar(_, Value::Address(target, _)) = item {
                        variables.request_layout_use(*target, owner);
                    }
                }
            } else {
                // Dynamic initialization still evaluates its recorded operands.
                let count = self
                    .graph
                    .variable_instances
                    .initializer_uses
                    .get(&entity)
                    .map_or(0, Vec::len);
                for index in 0..count {
                    let target = self.graph.variable_instances.initializer_uses[&entity][index];
                    self.graph
                        .variable_instances
                        .request_layout_use(target, owner);
                }
            }
        }
        Ok(())
    }
    pub(super) fn finish_instance_variables(&mut self) -> Result<(), Error> {
        while let Some(&entity) = self
            .graph
            .variable_instances
            .initializer_requests()
            .get(self.graph.variable_instances.next)
        {
            self.graph.variable_instances.next += 1;
            // These facts are prepared for lowering, whereas the C++ owner
            // instantiates them in variable_symbol. Keep their otherwise
            // namespace-context argument uses off the unit's early root list.
            let previous = self.initializer_demand_owner.replace(entity);
            let result = self.instantiate_variable(entity);
            self.initializer_demand_owner = previous;
            result?;
        }
        Ok(())
    }
    /// A scalar constant query instantiates only an unknown const/constexpr
    /// template variable. It records no evaluated storage or emission use.
    pub(super) fn variable_constant(
        &mut self,
        entity: EntityId,
    ) -> Result<Option<Constant>, Error> {
        let node = self.graph.entity(entity);
        if matches!(node.object().constant, ConstantState::None)
            && self.graph.variable_instances.instance(entity)
            && (node.constexpr
                || node.ty.is_some_and(|ty| {
                    self.graph.types.get(ty).cv.0 & super::types::Cv::CONST.0 != 0
                }))
        {
            self.instantiate_variable(entity)?;
        }
        match self.graph.entity(entity).object().constant {
            ConstantState::Value(value) => Ok(Some(value)),
            _ => Ok(None),
        }
    }

    // constant_evaluator.cpp::variable_value: naming a fixed-type variable in
    // semantics mode needs no initializer; reading its constant value does.
    pub(super) fn instantiate_variable_value(&mut self, entity: EntityId) -> Result<(), Error> {
        let node = self.graph.entity(entity);
        if self.graph.variable_instances.instance(entity)
            && (node.constexpr
                || node.ty.is_some_and(|ty| {
                    self.graph.lower_cv(ty) & 1 != 0
                        || matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                }))
        {
            self.instantiate_variable(entity)?;
        }
        Ok(())
    }
    fn instance_array_type(
        &mut self,
        ty: TypeId,
        declared: TypeId,
        scope: ScopeId,
        copy: &mut super::pack_frames::CopySession,
    ) -> Result<TypeId, Error> {
        let TypeKind::Array {
            element,
            bound: 0,
            operand: None,
        } = self.graph.types.get(ty).kind
        else {
            return Ok(ty);
        };
        let TypeKind::Array { bound, operand, .. } = self.graph.types.get(declared).kind else {
            return Ok(ty);
        };
        let bound = if let Some(operand) = operand {
            let TypeKind::TemplateParameter { entity, .. } = self.graph.types.get(operand).kind
            else {
                return Err(Error::Invariant("bound operand without entity"));
            };
            let pattern = self
                .graph
                .template_operands
                .pattern(entity)
                .ok_or(Error::Invariant("array bound without syntax"))?;
            let expression = self.copy_expression_with_session(pattern.expression, copy)?;
            self.array_bound(expression, scope)?
        } else {
            bound
        };
        if bound == 0 {
            return Ok(ty);
        }
        self.graph.types.intern(
            TypeKind::Array {
                element,
                bound,
                operand: None,
            },
            super::types::Cv::default(),
        )
    }
    pub(super) fn explicit_variable_instantiation(
        &mut self,
        info: &DeclaratorInfo,
        specs: super::specifiers::Specifiers,
        is_extern: bool,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let name = info
            .syntax_name
            .ok_or(Error::Invariant("explicit variable without name"))?;
        let binding = self.lookup(name, scope, super::lookup::Filter::Ordinary)?;
        let binding = self.graph.binding(binding);
        let entity = binding.entity;
        if binding.kind != super::graph::BindingKind::Variable
            || !self.graph.variable_instances.instance(entity)
        {
            return Err(Error::at(
                location,
                "an explicit instantiation names no static data member or variable template specialization",
            ));
        }
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("instance variable without type"))?;
        if self.graph.types.canonical_function(ty)?
            != self.graph.types.canonical_function(info.ty)?
        {
            return Err(Error::at(
                location,
                "an explicit instantiation's type differs from the variable's",
            ));
        }
        if specs.inline == super::specifiers::InlinePreference::Inline
            || specs.constexpr
            || specs.is_static
            || specs.is_extern
            || specs.thread_storage != super::specifiers::ThreadStorage::Ordinary
        {
            return Err(Error::at(
                location,
                "a specifier an explicit instantiation may not have",
            ));
        }
        let home = self
            .graph
            .entity(entity)
            .home
            .ok_or(Error::Invariant("instance variable without owner"))?;
        if !self.graph.encloses(
            self.declaration_scope(scope),
            self.enclosing_namespace(home),
        ) {
            return Err(Error::at(
                location,
                "an explicit instantiation outside its template's namespace",
            ));
        }
        if is_extern {
            return Ok(());
        }
        self.instantiate_variable(entity)?;
        if self.graph.variable_instances.initializer(entity).is_none() {
            return Err(Error::at(
                location,
                "an explicit instantiation of a variable whose template's definition is not available",
            ));
        }
        self.graph
            .variable_instances
            .explicit_definitions
            .insert(entity);
        self.graph.variable_instances.demand(entity);
        Ok(())
    }
    fn instantiate_variable(&mut self, entity: EntityId) -> Result<(), Error> {
        if self.graph.variable_instances.states.contains_key(&entity) {
            return Ok(());
        }
        if self.graph.variable_instances.templates.contains(entity) {
            return self.instantiate_variable_specialization(entity);
        }
        if self.graph.ordinary_inline_member(entity) {
            let node = self.graph.entity(entity);
            let initializer = node.initialization().member_initializer;
            let home = node
                .home
                .ok_or(Error::Invariant("inline member without owner"))?;
            self.graph.variable_instances.begin(entity);
            // The declaration already typed its initializer. In particular,
            // no initializer here must not select a default constructor.
            let result = self
                .request_static_initializer(entity, initializer, home, home)
                .map(|()| Definition { initializer });
            return self.graph.variable_instances.finish(entity, result);
        }
        let Some(pattern) = self.graph.variable_instances.outside_definition(entity) else {
            return Ok(());
        };
        self.graph.variable_instances.begin(entity);
        let result = (|| {
            let owner = self
                .graph
                .entity(entity)
                .home
                .ok_or(Error::Invariant("instance variable without owner"))?;
            let frame = if pattern.qualified {
                self.outside_class_frame(owner, pattern.head, pattern.scope)?
            } else {
                owner
            };
            let (initializer, mut copy) =
                self.prepare_initializer_copy(pattern.initializer, frame)?;
            self.required_instantiation(
                self.graph.instantiation_levels.get(entity)?,
                pattern.location,
                pattern.limit,
                |analyzer| {
                    let binding = analyzer
                        .graph
                        .own(
                            owner,
                            analyzer
                                .graph
                                .entity(entity)
                                .name
                                .ok_or(Error::Invariant("unnamed static member"))?,
                            super::lookup::Filter::Ordinary,
                        )
                        .ok_or(Error::Invariant("static member without binding"))?;
                    analyzer.graph.entity_mut(entity).defined = true;
                    analyzer
                        .graph
                        .entity_mut(entity)
                        .object_mut()
                        .variable_definition = Some(pattern.declarator);
                    let mut ty = analyzer
                        .graph
                        .entity(entity)
                        .ty
                        .ok_or(Error::Invariant("static member without type"))?;
                    ty = analyzer.instance_array_type(ty, pattern.ty, frame, &mut copy)?;
                    ty = analyzer.complete_initializer_array(ty, initializer)?;
                    analyzer.graph.complete_variable_type(binding, ty)?;
                    let effective = initializer.or(analyzer
                        .graph
                        .entity(entity)
                        .initialization()
                        .member_initializer);
                    if let Some(initializer) = initializer {
                        analyzer.initialize_variable(
                            binding,
                            initializer,
                            frame,
                            pattern.location,
                        )?;
                        analyzer.record_constant_initializer(entity, initializer, frame)?;
                    } else if effective.is_none()
                        && !matches!(
                            analyzer.graph.entity(entity).object().constant,
                            ConstantState::Value(_)
                        )
                        && !analyzer.graph.entity(entity).constexpr
                    {
                        analyzer.default_initialize_declaration(binding, pattern.location)?;
                    }
                    analyzer.variable_destruction(
                        binding,
                        frame,
                        pattern.location,
                        effective.is_some(),
                    )?;
                    if analyzer.lowering() {
                        analyzer.request_static_initializer(entity, effective, frame, owner)?;
                        analyzer.request_storage_layout(entity, pattern.location)?;
                    }
                    Ok(effective)
                },
            )
        })();
        match result {
            Ok(initializer) => {
                self.graph.work.totals.instantiated_variables += 1;
                self.graph
                    .variable_instances
                    .states
                    .insert(entity, State::Complete(Definition { initializer }));
            }
            Err(error) => {
                self.graph.variable_instances.states.remove(&entity);
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::lookup::Filter;
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn inline_member_storage_reuses_the_instances_typed_initializer_once() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{static inline constexpr int n=sizeof(T);\
              static inline constexpr int unused=sizeof(T)*2;};using X=A<int>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"inline-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names =
            [b"X".as_slice(), b"n", b"unused"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_hosted(true);
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let alias = analyzer
            .graph
            .own(global, names[0], Filter::Ordinary)
            .unwrap();
        let class = analyzer
            .graph
            .class_of(analyzer.graph.binding(alias).ty.unwrap())
            .unwrap();
        assert!(analyzer.complete_class(class).unwrap());
        let home = analyzer.graph.entity(class).context().scope.unwrap();
        let members = [names[1], names[2]].map(|name| {
            let binding = analyzer.graph.own(home, name, Filter::Ordinary).unwrap();
            analyzer.graph.binding(binding).entity
        });
        assert!(analyzer.graph.variable_instances.states.is_empty());
        assert!(analyzer.graph.variable_instances.demands.is_empty());
        let [entity, unused] = members;
        let source = analyzer.graph.variable_instances.links[&entity];
        let pattern = analyzer.graph.variable_instances.patterns[&source];
        assert!(pattern.initializer.is_none());
        assert!(pattern.head.is_none());
        assert!(!pattern.qualified);
        let initializer = analyzer
            .graph
            .entity(entity)
            .initialization()
            .member_initializer
            .unwrap();
        let ast::InitializerKind::Assign(expression) = initializer.kind else {
            panic!("assigned initializer");
        };
        assert!(analyzer.graph.expression_facts.get(expression).is_ok());
        assert!(analyzer.graph.entity(entity).defined);
        assert!(
            analyzer
                .graph
                .entity(entity)
                .object()
                .variable_definition
                .is_some()
        );
        assert!(
            analyzer
                .graph
                .variable_instances
                .initializer(entity)
                .is_none()
        );
        let expressions = analyzer.tree.expressions.mark();
        for _ in 0..2 {
            analyzer.instantiate_variable(entity).unwrap();
            let definition = analyzer
                .graph
                .variable_instances
                .initializer(entity)
                .unwrap();
            assert!(matches!(definition.initializer.unwrap().kind,
                ast::InitializerKind::Assign(id) if id == expression));
            assert_eq!(analyzer.graph.variable_instances.states.len(), 1);
            assert!(
                analyzer
                    .graph
                    .variable_instances
                    .initializer(unused)
                    .is_none()
            );
            assert_eq!(analyzer.tree.expressions.mark(), expressions);
        }
    }

    #[test]
    fn hosted_inline_storage_and_linkage_belong_to_the_first_declaration() {
        use super::super::specifiers::InlinePreference;
        for lowering in [false, true] {
            let source = SourceBuffer::new(
                b"inline const int a=1;extern const int b;inline const int b=2;\
                  struct A{static inline constexpr int n=3;};\
                  struct Deleted{Deleted()=delete;};struct Unused{static inline Deleted value;};"
                    .to_vec(),
            )
            .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"inline-linkage").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let names = [b"a".as_slice(), b"b", b"A", b"n", b"Unused", b"value"]
                .map(|name| cursor.intern_name(name).unwrap());
            let graph = super::super::analyze_unit(
                tree,
                cursor,
                declarations,
                StackBudget::default(),
                true,
                lowering,
            )
            .unwrap();
            let entities = [names[0], names[1], names[2]].map(|name| {
                let binding = graph.own(graph.global, name, Filter::Ordinary).unwrap();
                graph.binding(binding).entity
            });
            let [a, b, class] = entities;
            assert_eq!(graph.entity(a).inline == InlinePreference::Inline, lowering);
            assert_eq!(
                graph.entity(a).object().variable.unwrap().internal_linkage,
                !lowering
            );
            assert!(graph.entity(b).inline == InlinePreference::Unspecified);
            assert!(!graph.entity(b).object().variable.unwrap().internal_linkage);
            let binding = graph
                .own(
                    graph.entity(class).context().scope.unwrap(),
                    names[3],
                    Filter::Ordinary,
                )
                .unwrap();
            let member = graph.binding(binding).entity;
            assert_eq!(graph.entity(member).defined, lowering);
            assert!(graph.lower_demanded_initializer(member).is_none());
            let class = graph.own(graph.global, names[4], Filter::Ordinary).unwrap();
            let home = graph
                .entity(graph.binding(class).entity)
                .context()
                .scope
                .unwrap();
            let binding = graph.own(home, names[5], Filter::Ordinary).unwrap();
            let unused = graph.binding(binding).entity;
            assert_eq!(graph.entity(unused).defined, lowering);
            assert!(graph.entity(unused).initialization().constructor.is_none());
            assert_eq!(
                graph.lower_omitted_variable_initialization(unused),
                lowering
            );
            assert!(graph.variable_instances.states.is_empty());
        }
    }

    #[test]
    fn outside_pack_initializer_owns_element_facts_in_its_bound_frame() {
        let source = SourceBuffer::new(
            b"template<int...N>struct S{static int a[sizeof...(N)];};\
              template<int...N>int S<N...>::a[]={N...};using X=S<1,2,3>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"pack-initializer").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names =
            [b"X".as_slice(), b"a", b"N__pack0"].map(|name| cursor.intern_name(name).unwrap());
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let alias = analyzer
            .graph
            .own(global, names[0], Filter::Ordinary)
            .unwrap();
        let class = analyzer
            .graph
            .class_of(analyzer.graph.binding(alias).ty.unwrap())
            .unwrap();
        assert!(analyzer.complete_class(class).unwrap());
        let home = analyzer.graph.entity(class).context().scope.unwrap();
        let binding = analyzer
            .graph
            .own(home, names[1], Filter::Ordinary)
            .unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        let source = analyzer.graph.variable_instances.links[&entity];
        let pattern = analyzer.graph.variable_instances.patterns[&source];
        let frame = analyzer
            .outside_class_frame(home, pattern.head, pattern.scope)
            .unwrap();
        let element = analyzer
            .unqualified(frame, names[2], Filter::Ordinary)
            .unwrap()
            .unwrap();
        assert!(
            matches!(analyzer.graph.entity(analyzer.graph.binding(element).entity).object().constant,
            ConstantState::Value(value) if value.bits == 1)
        );
        let original = pattern.initializer.unwrap();
        let copy = analyzer.copy_initializer_in_frame(original, frame).unwrap();
        let ast::InitializerKind::Assign(original) = original.kind else {
            panic!("assigned initializer");
        };
        let ast::InitializerKind::Assign(copy) = copy.kind else {
            panic!("copied initializer");
        };
        let ast::ExprKind::Braced {
            elements: original, ..
        } = analyzer.tree.expressions.get(original).kind
        else {
            panic!("original list");
        };
        let ast::ExprKind::Braced {
            elements: copied, ..
        } = analyzer.tree.expressions.get(copy).kind
        else {
            panic!("copied list");
        };
        assert_eq!(analyzer.tree.expr_lists.get(original).len(), 1);
        assert_eq!(analyzer.tree.expr_lists.get(copied).len(), 3);
        for index in 0..analyzer.tree.expr_lists.get(copied).len() {
            let id = analyzer.tree.expr_lists.get(copied)[index];
            let ast::ExprKind::Id(name) = analyzer.tree.expressions.get(id).kind else {
                panic!("element name");
            };
            assert!(analyzer.lookup(name, frame, Filter::Ordinary).is_ok());
        }
        analyzer.instantiate_variable(entity).unwrap();
        analyzer.finish_static_initializers().unwrap();
        assert!(analyzer.graph.array_images.get(entity).is_some());
    }

    #[test]
    fn explicit_member_declaration_blocks_only_its_own_primary_initializer() {
        let source = SourceBuffer::new(
            b"template<class T>struct A{static int n;};\
              template<class T>int A<T>::n=sizeof(typename T::missing);\
              int size=sizeof(A<int>)+sizeof(A<char>);\
              template<>int A<int>::n;template<>int A<int>::n=7;"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"member-initializer").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let ids = analyzer.tree.decl_lists.get(declarations).to_vec();
        let global = analyzer.graph.global;
        for &id in &ids[..3] {
            analyzer.declaration(id, global).unwrap();
        }
        assert_eq!(analyzer.graph.variable_instances.links.len(), 2);
        let expressions = analyzer.tree.expressions.mark();
        analyzer.declaration(ids[3], global).unwrap();
        assert_eq!(analyzer.graph.variable_instances.links.len(), 1);
        assert!(analyzer.graph.variable_instances.states.is_empty());
        let ast::DeclKind::Template { declaration, .. } =
            analyzer.tree.declarations.get(ids[3]).kind
        else {
            panic!("specialization syntax");
        };
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("static member syntax");
        };
        let declarator = analyzer.tree.init_declarators.get(declarators)[0].declarator;
        let binding = analyzer.graph.declared_bindings[declarator.index()].unwrap();
        let entity = analyzer.graph.binding(binding).entity;
        assert!(!analyzer.graph.entity(entity).defined);
        analyzer.instantiate_variable(entity).unwrap();
        assert!(!analyzer.graph.entity(entity).defined);
        analyzer.declaration(ids[4], global).unwrap();
        assert!(analyzer.graph.entity(entity).defined);
        assert!(
            analyzer
                .graph
                .entity(entity)
                .object()
                .variable_definition
                .is_some()
        );
        assert!(analyzer.graph.variable_instances.states.is_empty());
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
    }

    #[test]
    fn variable_template_partials_preserve_primary_and_definition_ownership() {
        let source = SourceBuffer::new(
            b"template<class T,class U,class=void>extern const bool v;\
            template<class A,class B,class C>const bool v=false;\
            template<class X,class Y,class Z>extern const bool v;\
            template<class P>const bool v<P,P> =true;\
            template<class Q>int v<Q*,Q*> =sizeof(typename Q::missing);\
            template<class T>void unused(){int ignored<T>;}\
            template<class T>struct A{int ignored<T>;};"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"variable-patterns").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"v").unwrap();
        let initializer_count = tree.expressions.mark();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let primary = graph.binding(binding).entity;
        let variables = &graph.variable_instances;
        let partials = &variables.partials[&primary];
        assert_eq!(partials.len(), 2);
        assert_ne!(partials[0].1, partials[1].1);
        assert_ne!(partials[0].1, primary);
        assert_eq!(variables.patterns.len(), 3);
        assert!(variables.demands.is_empty());
        assert!(variables.links.is_empty());
        assert!(variables.states.is_empty());
        assert_eq!(variables.deferred_declarations.len(), 2);
        assert!(graph.entity(primary).defined);
        let pattern = variables.patterns[&primary];
        assert_ne!(pattern.head, graph.entity(primary).context().template_head);
        assert_ne!(
            pattern.head,
            graph.entity(partials[0].1).context().template_head
        );
        // Heads of identical shape have canonical parameter positions, but
        // each definition retains its actual scope and initializer syntax.
        assert_ne!(pattern.scope, variables.patterns[&partials[0].1].scope);
        for &entity in &[primary, partials[0].1, partials[1].1] {
            let pattern = variables.patterns[&entity];
            let ast::InitializerKind::Assign(expression) = pattern.initializer.unwrap().kind else {
                panic!("pattern lost initializer");
            };
            assert!(graph.expression_facts.get(expression).is_err());
        }
        assert_eq!(tree.expressions.mark(), initializer_count);
        assert!(partials.iter().all(|(arguments, _)| arguments.len() == 3));
    }

    #[test]
    fn invalid_partial_does_not_replace_or_register_the_primary() {
        for (tail, message) in [
            (
                "template<class U>int v<U>;",
                "a partial specialization whose arguments are its template's own parameters",
            ),
            (
                "template<class U=int>int v<U*>;",
                "a default template argument in a partial specialization's template parameter list",
            ),
            (
                "template<class U>int v<typename U::missing>;",
                "a partial specialization's template parameter that its arguments do not deduce",
            ),
        ] {
            let source =
                SourceBuffer::new(format!("template<class T>extern int v;{tail}").into_bytes())
                    .unwrap();
            let mut unit =
                SyntaxUnit::new(Preprocessor::with_name(&source, b"partial-publication").unwrap());
            unit.parse().unwrap();
            let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
            let name = cursor.intern_name(b"v").unwrap();
            let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
            let marker = 0u8;
            analyzer.stack_base = std::ptr::from_ref(&marker).addr();
            let result = analyzer.declarations(declarations, analyzer.graph.global);
            assert!(
                matches!(result, Err(Error::Diagnostic{message: actual,..}) if actual==message)
            );
            assert_eq!(analyzer.template_depth, 0);
            assert!(analyzer.template_head.is_none());
            let binding = analyzer
                .graph
                .own(
                    analyzer.graph.global,
                    name,
                    super::super::lookup::Filter::Ordinary,
                )
                .unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            assert!(
                analyzer
                    .graph
                    .entity(entity)
                    .context()
                    .template_head
                    .is_some()
            );
            assert!(!analyzer.graph.entity(entity).defined);
            let variables = &analyzer.graph.variable_instances;
            assert!(variables.partials.is_empty());
            assert_eq!(variables.patterns.len(), 1);
            assert!(variables.demands.is_empty());
        }
    }

    #[test]
    fn declarations_initializers_and_storage_are_independent_memoized_facts() {
        let source = SourceBuffer::new(b"template<class Original>struct Box{static Original value;static int bad;};\
            template<class Renamed>Renamed Box<Renamed>::value=sizeof(Renamed);\
            template<class T>int Box<T>::bad=T::missing;\
            int main(){return sizeof(Box<long>::bad)+Box<int>::value+Box<char>::value+Box<int>::value;}".to_vec()).unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"static-facts").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = super::super::analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let variables = &graph.variable_instances;
        assert_eq!(variables.patterns.len(), 2);
        assert_eq!(variables.links.len(), 6);
        assert_eq!(variables.demands.len(), 2);
        assert_eq!(variables.states.len(), 2);
        let mut copies = Vec::new();
        for &entity in &variables.demands {
            let definition = variables.initializer(entity).unwrap();
            let initializer = definition.initializer.unwrap();
            let original = variables
                .patterns
                .get(&variables.links[&entity])
                .unwrap()
                .initializer
                .unwrap();
            let ast::InitializerKind::Assign(copy) = initializer.kind else {
                panic!("copy lost initializer");
            };
            let ast::InitializerKind::Assign(pattern) = original.kind else {
                panic!("pattern lost initializer");
            };
            assert_ne!(copy, pattern);
            assert!(graph.expression_facts.get(copy).is_ok());
            assert!(graph.expression_facts.get(pattern).is_err());
            copies.push(copy);
        }
        assert_ne!(copies[0], copies[1]);
    }
}
