// Translated from sema/analyzer.cpp, analyzer_declarations.cpp and analyzer_classes.cpp. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::{Definition, SpecialName, ValueKey},
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
    pub home: ScopeId,
    pub print: ScopeId,
    pub head: Option<super::templates::HeadId>,
    pub definition: Definition,
}
impl Analyzer<'_> {
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
        let target_key = self.value_name(target)?;
        let lookup = if target_key.special().is_some() {
            self.lookup_location = name.location;
            let prefix = self
                .prefix(target, scope)?
                .ok_or(Error::Invariant("qualified using without prefix"))?;
            self.qualified_key(prefix, target_key, Filter::Ordinary)?
                .ok_or_else(|| Error::at(location, "undeclared name in a using-declaration"))
        } else {
            self.lookup(target, scope, Filter::Ordinary)
        };
        let target_id = lookup.map_err(|error| match error {
            Error::Diagnostic {
                message: "undeclared name",
                ..
            } => Error::at(location, "undeclared name in a using-declaration"),
            other => other,
        })?;
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
            self.graph.record_special_value(scope, key, id);
            if let Some(&label) = self.graph.special_labels.get(&target_id) {
                self.graph.special_labels.insert(id, label);
            }
        }
        if let Some(declaration) = declaration {
            self.graph.record_enum_declaration(id, declaration);
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
    ) -> Result<(), Error> {
        let info = self.declarator(Some(init.declarator), specs.ty, scope)?;
        let member = self.declared_member(&info, scope)?;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        let class_name = self.graph.scope(home).kind == ScopeKind::Class
            && self.graph.scope(home).owner.is_some_and(|class| {
                info.name.is_some() && info.name == self.graph.entity(class).name
            });
        if specs.role == DeclarationRole::Typedef
            && (member.is_some() || class_name || self.conversion_declarator(&info))
        {
            let message = if info.home.is_some() || init.initializer.is_some() {
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
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        let location = self.declarator_location(init.declarator)?;
        if specs.role == DeclarationRole::Typedef {
            if self.declarator_specifies_exceptions(Some(init.declarator)) {
                return Err(Error::at(
                    location,
                    "an exception-specification in a typedef declaration",
                ));
            }
            let binding = self.declare_alias(name, info.ty, home, location)?;
            super::record(
                &mut self.graph.declared_bindings,
                init.declarator.index(),
                binding,
            );
            return Ok(());
        }
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
        if info.home.is_some() && self.graph.own(home, name, Filter::Ordinary).is_none() {
            return Err(Error::at(
                location,
                "qualified definition does not match a declaration",
            ));
        }
        self.check_data_member_declaration(specs, &info, init.initializer, home, location)?;
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
                home,
                print: info.home.unwrap_or(scope),
                head: self.active_template_head(scope),
                definition: Definition::Declaration,
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
        let entity = self.graph.binding(binding).entity;
        if self.graph.scope(home).kind == ScopeKind::Class && info.home.is_none() {
            self.graph.entity_mut(entity).data_member = Some(if specs.is_static {
                super::graph::DataMemberKind::Static
            } else {
                super::graph::DataMemberKind::Instance
            });
        }
        if self.graph.scope(home).kind == ScopeKind::Class
            && !specs.is_static
            && info.home.is_none()
        {
            let owner = self
                .graph
                .scope(home)
                .owner
                .ok_or(Error::Invariant("class scope without owner"))?;
            self.graph.entity_mut(owner).members.push(entity);
            let mut declarator = Some(init.declarator);
            while let Some(id) = declarator {
                let node = self.tree.declarators.get(id);
                if !node.attributes.is_empty() {
                    self.graph.entity_mut(owner).layout = super::layout::LayoutState::Unsupported;
                }
                declarator = node.nested;
            }
        }
        if (self.graph.scope(home).kind != ScopeKind::Class || info.home.is_some())
            && (!specs.is_extern || init.initializer.is_some())
        {
            let entity = self.graph.binding(binding).entity;
            if self.graph.entity(entity).defined {
                return Err(Error::at(location, "redefinition of a variable"));
            }
            self.graph.entity_mut(entity).defined = true;
        }
        let member =
            self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance);
        let definition = (self.graph.scope(home).kind != ScopeKind::Class || info.home.is_some())
            && (!specs.is_extern || init.initializer.is_some());
        if member || definition {
            let entity = self.graph.binding(binding).entity;
            let object = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("object without type"))?;
            self.check_object_type(object, member, location)?;
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
                    self.note_local_initializer();
                }

                self.scalar_initializer(initializer, info.lookup, ty)?;
            } else {
                self.initializer_names(initializer, info.lookup, ty)?;
            }
            if !member {
                self.record_constant_initializer(
                    self.graph.binding(binding).entity,
                    initializer,
                    info.lookup,
                )?;
            }
        }
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
                bound = u64::try_from(elements.len())
                    .map_err(|_| Error::Limit("array initializer size limit"))?;
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
        if self.graph.scope(home).kind == ScopeKind::Class
            && info.home.is_none()
            && let Some(binding) = self.graph.value_binding(home, self.declaration_key(info)?)
            && self.graph.binding(binding).kind == BindingKind::Function
        {
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
            home,
            print,
            head,
            definition,
        } = context;
        if let Some(name) = name.identifier() {
            self.check_template_parameter_name(print, name, location)?;
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
                location,
                "redeclaration of a name of the enclosing construct",
            ));
        }
        let mut entity = None;
        if let Some(existing) = self.graph.value_binding(home, name) {
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
                    if self.graph.entity(binding.entity).constexpr != specs.constexpr {
                        return Err(Error::at(
                            location,
                            "declarations of a function that disagree on constexpr",
                        ));
                    }
                } else if let Some(completed) = self.graph.types.complete_array(old, ty) {
                    self.graph.entity_mut(binding.entity).ty = Some(completed);
                } else {
                    return Err(Error::at(location, "conflicting declaration of a variable"));
                }
                entity = Some(binding.entity);
            }
        }
        let entity = if let Some(entity) = entity {
            entity
        } else {
            self.graph
                .add_entity(Some(ty), name.identifier(), Keyword::Int)?
        };
        if kind == BindingKind::Function && self.graph.entity(entity).function.is_none() {
            let member = self.graph.scope(home).kind == ScopeKind::Class;
            let special = name.special();
            let allocation = special.is_some_and(SpecialName::allocation);
            if member
                && specs.is_static
                && matches!(special, Some(SpecialName::Operator { .. }))
                && !allocation
            {
                return Err(Error::at(location, "a static member operator function"));
            }
            self.graph.entity_mut(entity).function = Some(super::graph::FunctionProperties {
                internal_linkage: specs.is_static
                    && self.graph.scope(home).kind == ScopeKind::Namespace
                    || !self.c_language && self.internal_namespace(home),
                is_static: member && (specs.is_static || allocation),
                special,
                allocation: None,
            });
        }
        if kind == BindingKind::Function
            && let Some(head) = head
        {
            self.merge_template_head(entity, head, location)?;
        }
        let binding = self.graph.add_binding(
            home,
            Some(print),
            Binding {
                kind,
                name: name.identifier(),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )?;
        if let Some(name) = name.special() {
            self.graph.record_special_value(home, name, binding);
        }
        Ok(binding)
    }
    fn internal_namespace(&self, mut scope: ScopeId) -> bool {
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
        if !specs.constexpr || specs.is_static || self.graph.scope(home).kind != ScopeKind::Class {
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
    fn new_class(&mut self, key: Keyword, name: Option<NameId>) -> Result<EntityId, Error> {
        let entity = self.graph.add_entity(None, name, key)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Class(entity), Cv::default())?;
        self.graph.entity_mut(entity).ty = Some(ty);
        Ok(entity)
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
                .own(self.declaration_scope(scope), key_name, Filter::Elaborated)
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
            let binding = self.graph.binding(existing);
            if binding.kind != BindingKind::Type {
                return Err(Error::at(
                    node.location,
                    if binding.kind == BindingKind::Alias {
                        "elaborated type specifier names a typedef"
                    } else {
                        "elaborated type specifier names a different kind of type"
                    },
                ));
            }
            if self.graph.entity(binding.entity).enumeration.is_some() {
                return Err(Error::at(
                    node.location,
                    if lone {
                        "class name redeclared as a different kind of type"
                    } else {
                        "elaborated type specifier names a different kind of type"
                    },
                ));
            }
            if (key == Keyword::Union) != (self.graph.entity(binding.entity).key == Keyword::Union)
            {
                return Err(Error::at(
                    node.location,
                    "class-key does not agree with an earlier declaration",
                ));
            }
            binding.entity
        } else {
            if prefix.is_some() {
                return Err(Error::at(node.location, "undeclared name"));
            }
            self.new_class(key, Some(key_name))?
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
        if !node.bases.is_empty() {
            return Err(Error::Missing("class inheritance"));
        }
        let Some(name) = node.name else {
            let entity = self.new_class(node.key, None)?;
            self.graph.entity_mut(entity).anonymous_class =
                Some(super::anonymous_types::AnonymousClass::Pending(node.tokens));
            return self.define_class(node, entity, scope, scope);
        };
        let qualified_name = *self.tree.names.get(name);
        let prefix = self.prefix(name, scope)?;
        let home = prefix.unwrap_or(self.declaration_scope(scope));
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
        let existing = self.graph.own(home, key_name, Filter::Elaborated);
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
                    "type name conflicts with an earlier declaration",
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
            self.new_class(node.key, Some(key_name))?
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
        if node.packing != 0 || !node.attributes.is_empty() {
            self.graph.entity_mut(entity).layout = super::layout::LayoutState::Unsupported;
        }
        self.graph.entity_mut(entity).defined = true;
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
        self.class_depth += 1;
        self.declarations(node.members, class)?;
        self.class_depth -= 1;
        self.graph.entity_mut(entity).complete = true;
        if self.class_depth == 0 {
            self.finish_class_exceptions()?;
            let bodies = std::mem::take(&mut self.deferred);
            for body in bodies {
                self.function_body(body)?;
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
