// Translated from sema/analyzer.cpp, analyzer_declarations.cpp and analyzer_classes.cpp. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::{Definition, ValueKey},
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    specifiers::{DeclarationRole, Specifiers},
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::{
        ast::{self, Name},
        storage::List,
    },
};
#[derive(Clone, Copy)]
pub(super) struct ValueScope {
    pub qualified: bool,
    pub home: ScopeId,
    pub print: ScopeId,
    pub head: Option<super::templates::HeadId>,
    pub definition: Definition,
    pub declarator_location: SourceOffset,
}
impl Analyzer<'_> {
    pub(super) fn simple_declaration(
        &mut self,
        id: ast::DeclId,
        specifiers: List<ast::Specifier>,
        declarators: List<ast::InitDeclarator>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let declaration = *self.tree.declarations.get(id);
        let location = declaration.location;
        let first = self
            .tree
            .init_declarators
            .get(declarators)
            .first()
            .map(|init| init.declarator);
        let specs =
            self.declaration_specifiers(specifiers, scope, first, declarators.is_empty())?;
        self.keep_specifier_access();
        if specs.friendship == super::specifiers::Friendship::Friend && declarators.is_empty() {
            self.friend_type(specs.ty, scope, location)?;
            return Ok(());
        }
        if declarators.is_empty()
            && let TypeKind::Class(class) = self.graph.types.get(specs.ty).kind
        {
            self.class_layout_attributes(class, declaration.attributes, scope, true)?;
        }
        let first_name = self
            .tree
            .init_declarators
            .get(declarators)
            .first()
            .map(|init| self.simple_declarator_name(init.declarator))
            .transpose()?
            .flatten();
        let naming = if specs.role == super::specifiers::DeclarationRole::Typedef
            || !self.semantic() && self.graph.scope(scope).kind != ScopeKind::Class
        {
            first_name
        } else {
            None
        };
        if let Some(declared) = specs.declared {
            self.name_unnamed_enum(declared, naming)?;
            self.name_unnamed_class(declared, naming);
            if !declarators.is_empty() {
                self.name_local_class(declared)?;
            }
        }
        if declarators.is_empty()
            && let Some(storage) = self.anonymous_union(specs, scope, location, false)?
        {
            super::record(&mut self.graph.declaration_storage, id.index(), storage);
        }
        if declarators.is_empty() {
            self.settle_declaration_access(None)?;
        }
        let mut deduced = None;
        let attributes = if declarators.is_empty() {
            super::layout_attributes::LayoutAttributes::default()
        } else {
            self.layout_attributes(declaration.attributes, scope, false)?
        };
        for init in self.tree.init_declarators.get(declarators) {
            let attributes =
                self.declarator_layout_attributes(attributes, init.declarator, scope)?;
            self.resume_declaration_access(scope);
            self.declare_init(specs, *init, scope, location, &mut deduced)?;
            self.apply_storage_attributes(init.declarator, attributes)?;
            if init
                .initializer
                .is_some_and(|i| matches!(i.kind, ast::InitializerKind::Default))
                && let Some(binding) = self
                    .graph
                    .declared_bindings
                    .get(init.declarator.index())
                    .copied()
                    .flatten()
                && self.graph.binding(binding).kind == BindingKind::Function
            {
                self.graph
                    .entity_mut(self.graph.binding(binding).entity)
                    .function_definition = Some(id);
            }
        }
        Ok(())
    }
    pub(super) fn namespace_definition(
        &mut self,
        name: Option<NameId>,
        inline: bool,
        declarations: List<ast::DeclId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(scope).kind != ScopeKind::Namespace {
            return Err(Error::at(
                location,
                "a namespace can only be defined in a namespace",
            ));
        }
        let prior = name.and_then(|name| self.graph.own(scope, name, Filter::Ordinary));
        let target = if let Some(prior) = prior {
            let binding = self.graph.binding(prior);
            if binding.kind == BindingKind::NamespaceAlias {
                return Err(Error::at(location, "a namespace alias cannot be extended"));
            }
            if binding.kind != BindingKind::Namespace {
                return Err(Error::at(
                    location,
                    "namespace name conflicts with an earlier declaration",
                ));
            }
            let target = self
                .graph
                .entity(binding.entity)
                .scope
                .ok_or(Error::Invariant("namespace without scope"))?;
            if inline && !self.graph.scope(target).is_inline {
                return Err(Error::at(
                    location,
                    "an extension cannot make a namespace inline",
                ));
            }
            target
        } else if name.is_none() {
            // The anonymous namespace of this owner reopens its first anonymous child.
            let existing = self.graph.scope(scope).anonymous_namespace;
            if let Some(existing) = existing {
                existing
            } else {
                self.new_namespace(name, inline, scope)?
            }
        } else {
            self.new_namespace(name, inline, scope)?
        };
        self.declarations(declarations, target)
    }
    fn new_namespace(
        &mut self,
        name: Option<NameId>,
        inline: bool,
        scope: ScopeId,
    ) -> Result<ScopeId, Error> {
        let entity = self.graph.add_entity(None, name, Keyword::Namespace)?;
        let target = self
            .graph
            .add_scope(ScopeKind::Namespace, scope, Some(scope), name)?;
        self.graph.scope_mut(target).is_inline = inline;
        if name.is_none() {
            self.graph.scope_mut(scope).anonymous_namespace = Some(target);
        }
        self.graph.entity_mut(entity).scope = Some(target);
        self.graph.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::Namespace,
                name,
                entity,
                ty: None,
                key: Keyword::Namespace,
            },
        )?;
        if inline || name.is_none() {
            self.graph.add_edge(scope, target, false);
        }
        if inline {
            self.graph.add_edge(scope, target, true);
        }
        Ok(target)
    }
    pub(super) fn namespace_alias(
        &mut self,
        name: NameId,
        target: Name,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let target = self.lookup(target, scope, Filter::Namespace)?;
        let binding = self.graph.binding(target);
        let target_scope = self.graph.entity(binding.entity).scope;
        if let Some(existing) = self.graph.own(scope, name, Filter::Ordinary) {
            let existing = self.graph.binding(existing);
            if existing.kind == BindingKind::NamespaceAlias
                && self.graph.entity(existing.entity).scope == target_scope
            {
                return Ok(());
            }
            return Err(Error::at(
                location,
                "namespace alias conflicts with an earlier declaration",
            ));
        }
        let entity = self
            .graph
            .add_entity(None, Some(name), Keyword::Namespace)?;
        self.graph.entity_mut(entity).scope = target_scope;
        self.graph.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::NamespaceAlias,
                name: Some(name),
                entity,
                ..binding
            },
        )?;
        Ok(())
    }
    pub(super) fn using_declaration(
        &mut self,
        directive: bool,
        target: Name,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if directive {
            if self.graph.scope(scope).kind == ScopeKind::Class {
                return Err(Error::at(
                    location,
                    "a using-directive cannot appear in a class",
                ));
            }
            let binding = self.lookup(target, scope, Filter::Namespace)?;
            let target = self
                .graph
                .entity(self.graph.binding(binding).entity)
                .scope
                .ok_or(Error::Invariant("namespace directive without scope"))?;
            self.graph.add_edge(scope, target, false);
            return Ok(());
        }
        let name = self.tree.names.get(target);
        if !name.global && self.tree.name_lists.get(name.components).len() == 1 {
            return Err(Error::at(
                location,
                "a using-declaration needs a nested-name-specifier",
            ));
        }
        if self
            .tree
            .name_lists
            .get(name.components)
            .last()
            .is_some_and(|last| last.arguments.is_some())
        {
            return Err(Error::at(
                location,
                "a using-declaration cannot name a template-id",
            ));
        }
        if let Some(last) = self.tree.name_lists.get(name.components).last()
            && let ast::NameKind::Conversion(ty) = last.kind
        {
            return self.using_conversion(target, ty, scope, location);
        }
        let target_key = self.value_name(target)?;
        self.lookup_location = name.location;
        let prefix = self
            .prefix(target, scope)?
            .ok_or(Error::Invariant("qualified using without prefix"))?;
        if self.note_inherited_constructors(scope, prefix, target_key, target)? {
            return Ok(());
        }
        self.prepare_assignment_lookup(prefix, target_key, false)?;
        let found = self.qualified_result(prefix, target_key, Filter::Ordinary)?;
        let target_id = found
            .binding
            .ok_or_else(|| Error::at(location, "undeclared name in a using-declaration"))?;
        // Access is checked for the entire named set before hiding any import.
        // In particular, an inaccessible base overload cannot be excused by a
        // same-signature declaration already present in the derived class.
        if self.graph.scope(prefix).kind == ScopeKind::Class {
            let naming = self
                .graph
                .scope(prefix)
                .owner
                .ok_or(Error::Invariant("using member without naming class"))?;
            if self.graph.binding(target_id).kind == BindingKind::Function {
                for &binding in found.functions.as_slice() {
                    self.validate_member_access(binding, naming, None, scope, location)?;
                }
            } else {
                self.validate_member_access(target_id, naming, None, scope, location)?;
            }
        }
        if self.graph.binding(target_id).kind == BindingKind::Function {
            for &target in found.functions.as_slice() {
                self.introduce_function_using(target, target_key, scope, location)?;
            }
            return Ok(());
        }
        let binding = self.graph.binding(target_id);
        if matches!(
            binding.kind,
            BindingKind::Namespace | BindingKind::NamespaceAlias
        ) {
            return Err(Error::at(
                location,
                "a using-declaration cannot name a namespace",
            ));
        }
        if let Some(existing) = self.graph.value_binding(scope, target_key)
            && self.graph.binding(existing).entity != binding.entity
        {
            return Err(Error::at(
                location,
                if target_key.special().is_some() {
                    "using-declaration conflicts with a function of the same signature"
                } else {
                    "using-declaration conflicts with an earlier declaration"
                },
            ));
        }
        let declaration = self.graph.enum_declaration(target_id);
        let id = self.graph.add_binding(scope, Some(scope), binding)?;
        if let Some(key) = target_key.special() {
            self.graph.record_special_value(scope, key, id)?;
            if let Some(&label) = self.graph.special_labels.get(&target_id) {
                self.graph.special_labels.insert(id, label);
            }
        }
        if let Some(declaration) = declaration {
            self.graph.record_enum_declaration(id, declaration);
        }
        Ok(())
    }
    fn introduce_function_using(
        &mut self,
        target: BindingId,
        key: ValueKey,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let binding = self.graph.binding(target);
        if self
            .graph
            .value_binding(scope, key)
            .is_some_and(|id| self.graph.binding(id).kind != BindingKind::Function)
        {
            return Err(Error::at(
                location,
                "using-declaration conflicts with an earlier declaration",
            ));
        }
        let mut next = 0;
        while let Some(existing) = self.graph.next_function(scope, key, &mut next) {
            let earlier = self.graph.binding(existing);
            if earlier.entity != binding.entity
                && self.same_function_parameters(
                    earlier
                        .ty
                        .ok_or(Error::Invariant("function using without type"))?,
                    binding
                        .ty
                        .ok_or(Error::Invariant("function using without type"))?,
                )?
            {
                if self.graph.scope(scope).kind == ScopeKind::Class
                    && self.graph.entity(earlier.entity).home == Some(scope)
                {
                    return Ok(());
                }
                return Err(Error::at(
                    location,
                    "using-declaration conflicts with a function of the same signature",
                ));
            }
        }
        let id = self.graph.add_binding(scope, Some(scope), binding)?;
        if let Some(key) = key.special() {
            self.graph.record_special_value(scope, key, id)?;
            if let Some(&label) = self.graph.special_labels.get(&target) {
                self.graph.special_labels.insert(id, label);
            }
        }
        Ok(())
    }
    pub(super) fn declare_alias(
        &mut self,
        name: NameId,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        self.check_template_parameter_name(scope, name, location)?;
        let print = scope;
        let scope = self.declaration_scope(scope);
        let mut entity = None;
        if let Some(existing) = self.graph.own(scope, name, Filter::Ordinary) {
            let binding = self.graph.binding(existing);
            if !matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                return Err(Error::at(
                    location,
                    "typedef-name conflicts with an earlier declaration",
                ));
            }
            if binding.ty != Some(ty) {
                return Err(Error::at(
                    location,
                    "typedef redefinition with a different type",
                ));
            }
            if self.graph.scope(scope).kind == ScopeKind::Class
                && binding.kind == BindingKind::Alias
            {
                return Err(Error::at(location, "member typedef redeclared"));
            }
            if binding.kind == BindingKind::Alias {
                entity = Some(binding.entity);
            }
        }
        let entity = if let Some(entity) = entity {
            entity
        } else {
            self.graph
                .add_entity(Some(ty), Some(name), Keyword::Typedef)?
        };
        if let Some(head) = self.active_template_head(print) {
            self.merge_template_head(entity, head, location)?;
        }
        self.graph.add_binding(
            scope,
            Some(print),
            Binding {
                kind: BindingKind::Alias,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Typedef,
            },
        )
    }
    pub(super) fn declare_init(
        &mut self,
        specs: Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        _location: SourceOffset,
        deduced: &mut Option<TypeId>,
    ) -> Result<(), Error> {
        let info = self.placeholder_declarator(Some(init.declarator), specs, scope)?;
        self.declare_analyzed_init(specs, init, scope, deduced, info)
    }
    fn check_typedef_declarator(
        &self,
        specs: Specifiers,
        info: &super::declarators::DeclaratorInfo,
        init: ast::InitDeclarator,
        home: ScopeId,
        special_member: bool,
    ) -> Result<(), Error> {
        let class_name = self.graph.scope(home).kind == ScopeKind::Class
            && self.graph.scope(home).owner.is_some_and(|class| {
                info.name.is_some() && info.name == self.graph.entity(class).name
            });
        if specs.role == DeclarationRole::Typedef
            && (info.home.is_some()
                || init.initializer.is_some()
                || self.has_virt_specifier(info)
                || specs.constexpr
                || special_member
                || class_name
                || self.conversion_declarator(info))
        {
            let message = if info.home.is_some()
                || init.initializer.is_some()
                || self.has_virt_specifier(info)
            {
                "invalid typedef declarator"
            } else if specs.constexpr {
                "constexpr in a typedef declaration"
            } else {
                "invalid typedef name"
            };
            return Err(Error::at(
                self.tree.declarators.get(init.declarator).location,
                message,
            ));
        }
        Ok(())
    }
    pub(super) fn declare_analyzed_init(
        &mut self,
        specs: Specifiers,
        init: ast::InitDeclarator,
        scope: ScopeId,
        deduced: &mut Option<TypeId>,
        mut info: super::declarators::DeclaratorInfo,
    ) -> Result<(), Error> {
        let member = self.declared_member(&info, scope)?;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        self.check_virtual_object_declaration(specs, &info, init)?;
        self.check_mutable_declarator(
            specs,
            &info,
            self.tree.declarators.get(init.declarator).location,
        )?;
        self.check_typedef_declarator(specs, &info, init, home, member.is_some())?;
        if self.conversion_declarator(&info) {
            return Err(Error::at(
                self.declarator_location(init.declarator)?,
                "a conversion function with a return type",
            ));
        }
        if let Some((class, kind)) = member {
            return self.complete_special_member(
                class,
                kind,
                &info,
                specs,
                scope,
                super::special_members::MemberDeclaration {
                    location: self.declarator_location(init.declarator)?,
                    name: info
                        .syntax_name
                        .ok_or(Error::Invariant("special member without name"))?,
                    declarator: init.declarator,
                    initializer: init.initializer,
                    body: None,
                    implicit_type: false,
                },
            );
        }
        if specs.role != DeclarationRole::Typedef
            && matches!(
                self.graph.types.get(info.ty).kind,
                TypeKind::Function { .. }
            )
        {
            return self.declare_function_init(
                info,
                specs,
                init,
                scope,
                self.tree.declarators.get(init.declarator).location,
            );
        }
        if info.name.is_none() {
            return Err(Error::at(
                self.tree.declarators.get(init.declarator).location,
                if specs.role == DeclarationRole::Typedef {
                    "invalid typedef name"
                } else {
                    "invalid variable name"
                },
            ));
        }
        let name = info
            .name
            .ok_or(Error::Invariant("init-declarator without a name"))?;
        let location = self.declarator_location(init.declarator)?;
        if specs.role == DeclarationRole::Typedef {
            self.settle_declaration_access(None)?;
            return self.declare_typedef_binding(init.declarator, name, info.ty, home, location);
        }
        self.deduce_declared_object(&mut info, specs, init, home, location, deduced)?;
        let mut ty = info.ty;
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Void)
        ) {
            return Err(Error::at(
                self.tree.declarators.get(init.declarator).location,
                "variable of type void",
            ));
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            && init.initializer.is_none()
            && !specs.is_extern
            && self.graph.scope(home).kind != ScopeKind::Class
        {
            return Err(Error::at(
                self.tree.declarators.get(init.declarator).location,
                "reference declared without an initializer",
            ));
        }
        if specs.constexpr {
            ty = self.graph.types.qualify(ty, Cv::CONST)?;
        }
        ty = self.complete_initializer_array(ty, init.initializer)?;
        if info.home.is_some() && self.qualified(home, name, Filter::Ordinary)?.is_none() {
            return Err(Error::at(
                location,
                "qualified definition does not match a declaration",
            ));
        }
        self.check_mutable_variable(
            specs,
            &info,
            home,
            self.tree.declarators.get(init.declarator).location,
        )?;
        self.check_data_member_declaration(specs, &info, init.initializer, home, location)?;
        let mut specs = specs;
        specs.is_extern |= self.linkage_specification
            == Some(super::linkage::Specification::Single)
            && self.graph.scope(home).kind == ScopeKind::Namespace;
        self.check_variable_storage(specs, init.initializer.is_some(), home, location)?;
        if info.home.is_none()
            && self.graph.scope(home).kind == ScopeKind::Class
            && self
                .graph
                .scope(home)
                .index
                .get(&name)
                .is_some_and(|e| e.value.is_some())
        {
            return Err(Error::at(
                location,
                "redeclaration of a name in the same scope",
            ));
        }
        let binding = self.declare_value(
            name.into(),
            ty,
            ValueScope {
                qualified: info.home.is_some(),
                home,
                print: info.home.unwrap_or(scope),
                head: self.active_template_head(scope),
                definition: Definition::Declaration,
                declarator_location: self.tree.declarators.get(init.declarator).location,
            },
            BindingKind::Variable,
            location,
            specs,
        )?;
        super::record(
            &mut self.graph.declared_bindings,
            init.declarator.index(),
            binding,
        );
        self.settle_declaration_access(Some(binding))?;
        let entity = self.graph.binding(binding).entity;
        if self.semantic() {
            ty = self.graph.publish_variable_type(binding)?;
        }
        self.note_nothrow_target(entity, ty, Some(init.declarator), info.lookup)?;
        self.record_data_member(binding, specs, &info, init, home)?;
        let definition = (self.graph.scope(home).kind != ScopeKind::Class || info.home.is_some())
            && (!specs.is_extern || init.initializer.is_some());
        if definition {
            if self.graph.entity(entity).defined {
                return Err(Error::at(location, "redefinition of a variable"));
            }
            self.graph.entity_mut(entity).defined = true;
            self.graph.entity_mut(entity).variable_definition = Some(init.declarator);
        }
        let member =
            self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance);
        if member || definition {
            let object = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("object without type"))?;
            let object = self.initial_object_type(object, init.initializer);
            self.check_object_type(object, member, location)?;
            self.check_abstract_object(object, location)?;
        }
        self.graph.entity_mut(entity).constexpr |= specs.constexpr;
        self.check_static_member_initializer(specs, &info, init.initializer, home, location)?;
        if let Some(initializer) = init.initializer {
            if self.semantic() {
                if self.graph.scope(home).kind == ScopeKind::Block
                    && !specs.is_static
                    && !specs.is_extern
                    && specs.thread_storage == super::specifiers::ThreadStorage::Ordinary
                {
                    self.note_local_initializer(home)?;
                }

                ty = self.initialize_variable(binding, initializer, info.lookup, location)?;
                if member || definition {
                    self.check_object_type(ty, member, location)?;
                }
            } else {
                self.initializer_names(initializer, info.lookup, ty)?;
            }
            if !member {
                self.record_declared_constant(entity, initializer, &info, location)?;
                if self.lowering() && definition {
                    self.request_static_initializer(entity, initializer, info.lookup, home)?;
                }
            }
        } else if self.semantic() && definition {
            self.default_initialize_declaration(binding, location)?;
        }
        if self.semantic()
            && self.template_depth == 0
            && !member
            && (definition || init.initializer.is_some())
        {
            self.variable_destruction(binding, info.lookup, location, init.initializer.is_some())?;
        }
        if self.lowering() && self.template_depth == 0 && definition && !member {
            self.request_storage_layout(entity, location)?;
        }
        Ok(())
    }
    fn declare_typedef_binding(
        &mut self,
        declarator: ast::DeclaratorId,
        name: NameId,
        ty: TypeId,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.declarator_specifies_exceptions(Some(declarator)) {
            return Err(Error::at(
                self.tree.declarators.get(declarator).location,
                "an exception-specification in a typedef declaration",
            ));
        }
        if matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
            return Err(Error::at(
                self.tree.declarators.get(declarator).location,
                "typedef of auto",
            ));
        }
        let binding = self.declare_alias(name, ty, home, location)?;
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        Ok(())
    }
    fn complete_initializer_array(
        &mut self,
        ty: TypeId,
        initializer: Option<ast::Initializer>,
    ) -> Result<TypeId, Error> {
        let TypeKind::Array { element, bound: 0 } = self.graph.types.get(ty).kind else {
            return Ok(ty);
        };
        let Some(initializer) = initializer else {
            return Ok(ty);
        };
        let (ast::InitializerKind::Assign(expr) | ast::InitializerKind::Braced(expr)) =
            initializer.kind
        else {
            return Ok(ty);
        };
        let mut bound = 0;
        match self.tree.expressions.get(expr).kind {
            ast::ExprKind::Braced { elements, .. } => {
                let elements = self.tree.expr_lists.get(elements);
                if !self.semantic() {
                    bound = u64::try_from(elements.len())
                        .map_err(|_| Error::Limit("array initializer size limit"))?;
                }
                if elements.len() == 1
                    && self.integral_element(element)
                    && let ast::ExprKind::Literal(literal) =
                        self.tree.expressions.get(elements[0]).kind
                    && let crate::lex::PostTokenKind::String { elements, .. } = literal.kind
                {
                    bound = u64::from(elements);
                }
            }
            ast::ExprKind::Literal(literal) => {
                if let crate::lex::PostTokenKind::String { elements, .. } = literal.kind {
                    bound = u64::from(elements);
                }
            }
            _ => (),
        }
        if bound == 0 {
            return Ok(ty);
        }
        self.graph
            .types
            .intern(TypeKind::Array { element, bound }, Cv::default())
    }
    fn integral_element(&self, ty: TypeId) -> bool {
        match self.graph.types.get(ty).kind {
            TypeKind::Scalar(Scalar::Basic(ty)) => !matches!(
                ty,
                crate::types::FundamentalType::Float
                    | crate::types::FundamentalType::Double
                    | crate::types::FundamentalType::LongDouble
            ),
            TypeKind::Scalar(
                Scalar::SignedChar | Scalar::UnsignedChar | Scalar::Short | Scalar::UnsignedShort,
            ) => true,
            _ => false,
        }
    }
    fn check_object_type(
        &self,
        mut ty: TypeId,
        member: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        while let TypeKind::Array { element, bound } = self.graph.types.get(ty).kind {
            if bound == 0 && !member {
                return Err(Error::at(location, "variable of incomplete type"));
            }
            ty = element;
        }
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind
            && !self.graph.entity(entity).complete
        {
            return Err(Error::at(location, "variable of incomplete type"));
        }
        Ok(())
    }
    pub(super) fn check_member_function_redeclaration(
        &mut self,
        info: &super::declarators::DeclaratorInfo,
        home: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.graph.scope(home).kind == ScopeKind::Class && info.home.is_none() {
            let key = self.declaration_key(info)?;
            let mut next = 0;
            while let Some(binding) = self.graph.next_function(home, key, &mut next) {
                if self.graph.entity(self.graph.binding(binding).entity).home != Some(home) {
                    continue;
                }
                let old = self
                    .graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("function without type"))?;
                if self.graph.types.canonical_function(old)?
                    == self.graph.types.canonical_function(info.ty)?
                {
                    return Err(Error::at(
                        location,
                        "member function redeclared in its class",
                    ));
                }
            }
        }
        Ok(())
    }
    pub(super) fn declare_value(
        &mut self,
        name: ValueKey,
        ty: TypeId,
        context: ValueScope,
        kind: BindingKind,
        location: SourceOffset,
        specs: Specifiers,
    ) -> Result<BindingId, Error> {
        let ValueScope {
            qualified,
            home,
            print,
            head,
            definition,
            declarator_location,
        } = context;
        if let Some(name) = name.identifier() {
            self.check_template_parameter_name(print, name, location)?;
        }
        if kind == BindingKind::Function
            && home == self.graph.global
            && head.is_none()
            && let Some(key) = name.identifier()
        {
            self.ensure_builtin(key)?;
        }
        if kind == BindingKind::Variable
            && self.graph.scope(home).guarded
            && let Some(parent) = self.graph.scope(home).parent
            && let Some(name) = name.identifier()
            && self
                .graph
                .scope(parent)
                .index
                .get(&name)
                .is_some_and(|entry| entry.value.is_some())
        {
            return Err(Error::at(
                declarator_location,
                "redeclaration of a name of the enclosing construct",
            ));
        }
        let mut entity = None;
        let mut existing = if qualified && kind == BindingKind::Variable {
            self.qualified_key(home, name, Filter::Ordinary)?
        } else {
            self.graph.value_binding(home, name)
        };
        if kind == BindingKind::Function
            && existing.is_some_and(|id| self.graph.binding(id).kind == BindingKind::Function)
        {
            existing = self.match_declared_function(home, name, ty, location)?;
        }
        if kind == BindingKind::Variable
            && existing.is_none()
            && self.graph.scope(home).kind == ScopeKind::Block
            && specs.is_extern
            && let Some(name) = name.identifier()
            && let Some(outer) = self.outer_linkage_entity(home, name, kind, ty)?
        {
            entity = Some(outer);
        }
        if let Some(existing) = existing {
            let binding = self.graph.binding(existing);
            if matches!(
                binding.kind,
                BindingKind::Namespace | BindingKind::NamespaceAlias | BindingKind::Alias
            ) || (matches!(
                binding.kind,
                BindingKind::Variable
                    | BindingKind::Function
                    | BindingKind::Parameter
                    | BindingKind::Enumerator
            ) && binding.kind != kind)
            {
                return Err(Error::at(
                    location,
                    if kind == BindingKind::Function {
                        "function conflicts with an earlier declaration"
                    } else if matches!(
                        binding.kind,
                        BindingKind::Function | BindingKind::Parameter | BindingKind::Enumerator
                    ) {
                        "redeclaration of a name in the same scope"
                    } else {
                        "variable name conflicts with an earlier declaration"
                    },
                ));
            }
            if binding.kind == kind {
                let old = self
                    .graph
                    .entity(binding.entity)
                    .ty
                    .ok_or(Error::Invariant("value entity without type"))?;
                if kind == BindingKind::Function {
                    let previous = self.graph.entity(binding.entity).template_head;
                    if match (head, previous) {
                        (Some(a), Some(b)) => !self.same_template_shape(a, b),
                        (None, None) => false,
                        _ => true,
                    } {
                        return Err(Error::Missing("ordinary/template function overload sets"));
                    }
                    self.check_function_type_agreement(old, ty, location)?;
                    if definition == Definition::Deleted {
                        return Err(Error::at(
                            location,
                            "deleted definition of a function declared before",
                        ));
                    }
                    if definition.defines() && self.graph.entity(binding.entity).defined {
                        return Err(Error::at(location, "redefinition of a function"));
                    }
                    if specs.is_static
                        && self.graph.scope(home).kind == ScopeKind::Namespace
                        && self
                            .graph
                            .entity(binding.entity)
                            .function
                            .is_some_and(|props| !props.internal_linkage)
                    {
                        return Err(Error::at(
                            location,
                            "a static declaration of a function declared with external linkage",
                        ));
                    }
                    if self.graph.scope(home).kind == ScopeKind::Namespace
                        && !qualified
                        && self.graph.entity(binding.entity).builtin.is_none()
                        && let Some(properties) = self.graph.entity(binding.entity).function
                    {
                        self.agree_language(properties.language, location)?;
                    }
                    if self.graph.entity(binding.entity).constexpr != specs.constexpr {
                        return Err(Error::at(
                            location,
                            "declarations of a function that disagree on constexpr",
                        ));
                    }
                } else if let Some(completed) = self.graph.types.complete_array(old, ty) {
                    self.agree_variable_linkage(binding.entity, specs, home, qualified, location)?;
                    self.graph.entity_mut(binding.entity).ty = Some(completed);
                } else {
                    return Err(Error::at(location, "conflicting declaration of a variable"));
                }
                entity = Some(binding.entity);
            }
        }
        if let Some(entity) = entity
            && kind == BindingKind::Variable
            && existing.is_none()
        {
            self.complete_linked_variable(entity, ty, location)?;
        }
        let entity = if let Some(entity) = entity {
            entity
        } else {
            let entity = self
                .graph
                .add_entity(Some(ty), name.identifier(), Keyword::Int)?;
            if kind == BindingKind::Variable {
                self.initialize_linkage(
                    entity,
                    specs,
                    super::linkage::Declaration {
                        home,
                        name,
                        ty,
                        kind,
                        templated: head.is_some(),
                        location,
                    },
                )?;
            }
            entity
        };
        if kind == BindingKind::Function
            && specs.inline == super::specifiers::InlinePreference::Inline
        {
            self.graph.entity_mut(entity).inline = specs.inline;
        }
        if kind == BindingKind::Function && self.graph.entity(entity).function.is_none() {
            self.initialize_linkage(
                entity,
                specs,
                super::linkage::Declaration {
                    home,
                    name,
                    ty,
                    kind,
                    templated: head.is_some(),
                    location,
                },
            )?;
        }
        if kind == BindingKind::Function
            && let Some(head) = head
        {
            self.merge_template_head(entity, head, location)?;
        }
        let ty = if kind == BindingKind::Variable
            && matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Array { bound: 0, .. }
            ) {
            self.graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("completed variable without type"))?
        } else {
            ty
        };
        let invisible = kind == BindingKind::Function
            && specs.friendship == super::specifiers::Friendship::Friend
            && existing
                .filter(|&id| self.graph.binding(id).kind == BindingKind::Function)
                .is_none_or(|id| self.graph.friendships.hidden(id));
        let binding = self.graph.add_value_binding(
            home,
            Some(print),
            Binding {
                kind,
                name: name.identifier(),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
            invisible,
        )?;
        if let Some(name) = name.special() {
            self.graph.record_special_value(home, name, binding)?;
        }
        Ok(binding)
    }
    fn match_declared_function(
        &mut self,
        home: ScopeId,
        name: ValueKey,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        if self.graph.scope(home).kind == ScopeKind::Namespace {
            let candidate = self.graph.namespace_function(home, name, ty)?;
            if let Some(candidate) = candidate
                && self.graph.entity(self.graph.binding(candidate).entity).home != Some(home)
            {
                return Err(Error::at(
                    location,
                    "function conflicts with a using-declaration",
                ));
            }
            return Ok(candidate);
        }
        let mut next = 0;
        while let Some(candidate) = self.graph.next_function(home, name, &mut next) {
            let binding = self.graph.binding(candidate);
            let old = binding
                .ty
                .ok_or(Error::Invariant("function without type"))?;
            if self.same_function_parameters(old, ty)? {
                if self.graph.entity(binding.entity).home != Some(home) {
                    if self.graph.scope(home).kind != ScopeKind::Class {
                        return Err(Error::at(
                            location,
                            "function conflicts with a using-declaration",
                        ));
                    }
                    self.graph
                        .remove_class_function(home, name, binding.entity)?;
                    return Ok(None);
                }
                return Ok(Some(candidate));
            }
        }
        Ok(None)
    }
    pub(super) fn internal_namespace(&self, mut scope: ScopeId) -> bool {
        loop {
            let current = self.graph.scope(scope);
            if scope != self.graph.global
                && current.kind == ScopeKind::Namespace
                && current.name.is_none()
            {
                return true;
            }
            let Some(parent) = current.parent else {
                return false;
            };
            scope = parent;
        }
    }
    pub(super) fn same_function_parameters(&mut self, a: TypeId, b: TypeId) -> Result<bool, Error> {
        let a = self.graph.types.canonical_function(a)?;
        let b = self.graph.types.canonical_function(b)?;
        let (
            TypeKind::Function {
                parameters: ap,
                variadic: av,
                cv: ac,
                reference: ar,
                ..
            },
            TypeKind::Function {
                parameters: bp,
                variadic: bv,
                cv: bc,
                reference: br,
                ..
            },
        ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
        else {
            return Err(Error::Invariant("function signature without function type"));
        };
        Ok(ap == bp && av == bv && ac == bc && ar == br)
    }
    pub(super) fn check_function_type_agreement(
        &mut self,
        old: TypeId,
        new: TypeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let a = self.graph.types.canonical_function(old)?;
        let b = self.graph.types.canonical_function(new)?;
        if a == b {
            return Ok(());
        }
        if let (
            TypeKind::Function {
                parameters: ap,
                variadic: av,
                cv: ac,
                reference: ar,
                ..
            },
            TypeKind::Function {
                parameters: bp,
                variadic: bv,
                cv: bc,
                reference: br,
                ..
            },
        ) = (&self.graph.types.get(a).kind, &self.graph.types.get(b).kind)
            && ap == bp
            && av == bv
            && ac == bc
            && ar == br
        {
            return Err(Error::at(
                location,
                "functions that differ only in their return type",
            ));
        }
        Err(Error::Missing(
            "function overload sets and incompatible redeclaration diagnostics",
        ))
    }
    pub(super) fn function_specifier_type(
        &mut self,
        specs: Specifiers,
        info: &mut super::declarators::DeclaratorInfo,
        scope: ScopeId,
    ) -> Result<(), Error> {
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        if !specs.constexpr
            || specs.is_static
            || specs.friendship == super::specifiers::Friendship::Friend
            || self.graph.scope(home).kind != ScopeKind::Class
        {
            return Ok(());
        }
        if info.home.is_some()
            && let Some(binding) = info
                .name
                .and_then(|name| self.graph.own(home, name, Filter::Ordinary))
            && self
                .graph
                .entity(self.graph.binding(binding).entity)
                .function
                .is_some_and(|props| props.is_static)
        {
            return Ok(());
        }
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            cv,
            reference,
        } = &self.graph.types.get(info.ty).kind
        else {
            return Ok(());
        };
        if cv.0 & Cv::CONST.0 == 0 {
            info.ty = self.graph.types.intern(
                TypeKind::Function {
                    result: *result,
                    parameters: parameters.clone(),
                    variadic: *variadic,
                    cv: Cv(cv.0 | Cv::CONST.0),
                    reference: *reference,
                },
                Cv::default(),
            )?;
        }
        Ok(())
    }
    fn new_class(
        &mut self,
        key: Keyword,
        name: Option<NameId>,
        home: ScopeId,
    ) -> Result<EntityId, Error> {
        let entity = self.graph.add_entity(None, name, key)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        self.graph.entity_mut(entity).ty = Some(ty);
        self.record_local_class(entity, home)?;
        Ok(entity)
    }
    pub(super) fn elaborated_class_entity(
        &self,
        existing: BindingId,
        key: Keyword,
        location: SourceOffset,
        lone: bool,
    ) -> Result<EntityId, Error> {
        let binding = self.graph.binding(existing);
        if binding.kind != BindingKind::Type {
            return Err(Error::at(
                location,
                if binding.kind == BindingKind::Alias {
                    "elaborated type specifier names a typedef"
                } else {
                    "elaborated type specifier names a different kind of type"
                },
            ));
        }
        if self.graph.entity(binding.entity).enumeration.is_some() {
            return Err(Error::at(
                location,
                if lone {
                    "class name redeclared as a different kind of type"
                } else {
                    "elaborated type specifier names a different kind of type"
                },
            ));
        }
        if (key == Keyword::Union) != (self.graph.entity(binding.entity).key == Keyword::Union) {
            return Err(Error::at(
                location,
                "class-key does not agree with an earlier declaration",
            ));
        }
        Ok(binding.entity)
    }
    pub(super) fn elaborated_type(
        &mut self,
        key: Keyword,
        name: Name,
        scope: ScopeId,
        lone: bool,
    ) -> Result<TypeId, Error> {
        if key == Keyword::Enum {
            return self.elaborated_enum(name, scope);
        }
        let node = *self.tree.names.get(name);
        let prefix = self.prefix(name, scope)?;
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty class name"))?;
        let key_name = Self::component_name(last.kind)?;
        if lone {
            self.check_template_parameter_name(scope, key_name, node.location)?;
        }
        let existing = if let Some(prefix) = prefix {
            self.qualified(prefix, key_name, Filter::Elaborated)?
        } else if lone {
            self.graph
                .declared_name(self.declaration_scope(scope), key_name, Filter::Elaborated)
        } else {
            self.unqualified(scope, key_name, Filter::Elaborated)?
        };
        let mut declaration_home = self.declaration_scope(scope);
        if !lone && existing.is_none() {
            while !matches!(
                self.graph.scope(declaration_home).kind,
                ScopeKind::Namespace | ScopeKind::Block
            ) {
                declaration_home = self
                    .graph
                    .scope(declaration_home)
                    .parent
                    .ok_or(Error::Invariant("elaborated declaration without home"))?;
            }
        }
        let entity = if let Some(existing) = existing {
            self.elaborated_class_entity(existing, key, node.location, lone)?
        } else {
            if prefix.is_some() {
                return Err(Error::at(node.location, "undeclared name"));
            }
            self.new_class(key, Some(key_name), declaration_home)?
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class without type"))?;
        if lone {
            self.class_template_head(entity, scope, existing.is_some(), node.location)?;
        }
        if lone || existing.is_none() {
            self.graph.add_binding(
                declaration_home,
                Some(if declaration_home == self.declaration_scope(scope) {
                    scope
                } else {
                    declaration_home
                }),
                Binding {
                    kind: BindingKind::Type,
                    name: Some(key_name),
                    entity,
                    ty: Some(ty),
                    key,
                },
            )?;
        }
        Ok(ty)
    }
    pub(super) fn class_type(&mut self, id: ast::ClassId, scope: ScopeId) -> Result<TypeId, Error> {
        let node = *self.tree.classes.get(id);
        let Some(name) = node.name else {
            let entity = self.new_class(node.key, None, scope)?;
            self.graph.entity_mut(entity).anonymous_class =
                Some(super::anonymous_types::AnonymousClass::Pending(node.tokens));
            return self.define_class(node, entity, scope, scope);
        };
        let qualified_name = *self.tree.names.get(name);
        let prefix = self.prefix(name, scope)?;
        let home = prefix.unwrap_or(self.declaration_scope(scope));
        if prefix.is_some() && self.graph.scope(home).kind == ScopeKind::Enum {
            return Err(Error::at(
                qualified_name.location,
                "qualified class name must name a class or namespace member",
            ));
        }
        if prefix.is_some() && !self.graph.encloses(self.declaration_scope(scope), home) {
            return Err(Error::at(
                qualified_name.location,
                "class definition in a scope that does not enclose it",
            ));
        }
        let key_name = self.declarator_class_name(name)?;
        if prefix.is_none() {
            self.check_template_parameter_name(scope, key_name, qualified_name.location)?;
        }
        let existing = if prefix.is_some() {
            self.qualified_declaration_result(home, key_name.into(), Filter::Elaborated)?
                .binding
        } else {
            self.graph.declared_name(home, key_name, Filter::Elaborated)
        };
        if prefix.is_some() && existing.is_none() {
            return Err(Error::at(
                qualified_name.location,
                "qualified class definition does not match a declaration",
            ));
        }
        let entity = if let Some(existing) = existing {
            let binding = self.graph.binding(existing);
            if binding.kind != BindingKind::Type {
                return Err(Error::at(
                    qualified_name.location,
                    if prefix.is_some() {
                        "qualified class definition does not match a declaration"
                    } else {
                        "type name conflicts with an earlier declaration"
                    },
                ));
            }
            if self.graph.entity(binding.entity).enumeration.is_some() {
                return Err(Error::at(
                    qualified_name.location,
                    if prefix.is_some() {
                        "qualified class definition does not match a declaration"
                    } else {
                        "class name redeclared as a different kind of type"
                    },
                ));
            }
            binding.entity
        } else {
            self.new_class(node.key, Some(key_name), home)?
        };
        if self.graph.entity(entity).defined {
            return Err(Error::at(qualified_name.location, "class redefinition"));
        }
        if (node.key == Keyword::Union) != (self.graph.entity(entity).key == Keyword::Union) {
            return Err(Error::at(
                qualified_name.location,
                "class-key does not agree with an earlier declaration",
            ));
        }
        self.class_template_head(entity, scope, existing.is_some(), qualified_name.location)?;
        let print = prefix.unwrap_or(scope);
        let parent = if prefix.is_some() && self.active_template_head(scope).is_some() {
            self.qualified_template_scope(scope, home)?
        } else {
            print
        };
        self.define_class(node, entity, parent, print)
    }
    fn define_class(
        &mut self,
        node: ast::ClassSyntax,
        entity: EntityId,
        parent: ScopeId,
        home: ScopeId,
    ) -> Result<TypeId, Error> {
        let name = self.graph.entity(entity).name;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class without type"))?;
        self.graph.add_binding(
            self.declaration_scope(home),
            Some(home),
            Binding {
                kind: BindingKind::Type,
                name,
                entity,
                ty: Some(ty),
                key: node.key,
            },
        )?;
        let class = self
            .graph
            .add_scope(ScopeKind::Class, parent, Some(home), name)?;
        self.graph.entity_mut(entity).scope = Some(class);
        self.graph.scope_mut(class).owner = Some(entity);
        self.class_layout_attributes(entity, node.attributes, parent, false)?;
        self.graph.entity_mut(entity).class_packing = node.packing;
        self.graph.entity_mut(entity).defined = true;
        self.graph.entity_mut(entity).derivation = if node.final_specifier {
            super::inheritance::Derivation::Final
        } else {
            super::inheritance::Derivation::Allowed
        };
        if name.is_some() {
            self.graph.add_binding(
                class,
                None,
                Binding {
                    kind: BindingKind::Type,
                    name,
                    entity,
                    ty: Some(ty),
                    key: node.key,
                },
            )?;
        }
        // The injected name is public; the class-key governs written members.
        self.graph.scope_mut(class).member_access = if node.key == Keyword::Class {
            super::graph::Access::Private
        } else {
            super::graph::Access::Public
        };
        self.class_bases(node, entity, parent)?;
        self.class_depth += 1;
        self.declarations(node.members, class)?;
        self.class_depth -= 1;
        self.graph.entity_mut(entity).complete = true;
        if self.semantic() && self.template_depth == 0 {
            self.declare_inherited_constructors(entity)?;
        }
        self.finish_class_initializers(entity, node.location)?;
        if self.semantic() && self.template_depth == 0 {
            self.classify_class_copies(entity)?;
            self.complete_destructor(entity)?;
            self.complete_virtual_destructor(entity, node.location)?;
            self.complete_virtual_assignments(entity, node.location)?;
            self.complete_virtual_members(entity)?;
            self.complete_virtual_layout(entity)?;
        }
        if self.template_depth == 0 && self.graph.entity(entity).storage_attributes.alignment != 0 {
            self.class_layout(entity, node.location)?;
        }
        if self.class_depth == 0 {
            self.finish_class_exceptions()?;
            let bodies = std::mem::take(&mut self.deferred);
            for body in bodies {
                match body {
                    super::analyzer::DeferredMember::Body(body) => self.function_body(body)?,
                    super::analyzer::DeferredMember::Defaults(defaults) => {
                        self.type_function_defaults(defaults)?;
                    }
                }
            }
        }
        Ok(ty)
    }
    fn declarator_class_name(&self, name: Name) -> Result<NameId, Error> {
        let node = self.tree.names.get(name);
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty class name"))?;
        if last.arguments.is_some() {
            return Err(Error::Missing("class template specializations"));
        }
        Self::component_name(last.kind)
    }
}
