// Translated from sema/analyzer.cpp, analyzer_declarations.cpp and analyzer_classes.cpp. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
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
            let existing = self
                .graph
                .scope(scope)
                .children
                .iter()
                .copied()
                .find(|&id| {
                    self.graph.scope(id).kind == ScopeKind::Namespace
                        && self.graph.scope(id).name.is_none()
                });
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
        if let Some(existing) = self.graph.own(scope, name, Filter::Ordinary) {
            let existing = self.graph.binding(existing);
            if existing.kind == BindingKind::NamespaceAlias && existing.entity == binding.entity {
                return Ok(());
            }
            return Err(Error::at(
                location,
                "namespace alias conflicts with an earlier declaration",
            ));
        }
        self.graph.add_binding(
            scope,
            None,
            Binding {
                kind: BindingKind::NamespaceAlias,
                name: Some(name),
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
        let target_id =
            self.lookup(target, scope, Filter::Ordinary)
                .map_err(|error| match error {
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
        if let Some(existing) = binding
            .name
            .and_then(|name| self.graph.own(scope, name, Filter::Ordinary))
            && self.graph.binding(existing).entity != binding.entity
        {
            return Err(Error::at(
                location,
                "using-declaration conflicts with an earlier declaration",
            ));
        }
        let declaration = self.graph.enum_declaration(target_id);
        let id = self.graph.add_binding(scope, Some(scope), binding)?;
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
        self.graph.add_binding(
            scope,
            Some(scope),
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
        let name = info
            .name
            .ok_or(Error::Invariant("init-declarator without a name"))?;
        let home = info.home.unwrap_or(scope);
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
        let function = matches!(
            self.graph.types.get(info.ty).kind,
            TypeKind::Function { .. }
        );
        if function {
            self.check_member_function_redeclaration(&info, home, location)?;
        }
        let mut ty = info.ty;
        if !function {
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
        }
        if !function {
            ty = self.complete_initializer_array(ty, init.initializer)?;
        }
        if info.home.is_some() && self.graph.own(home, name, Filter::Ordinary).is_none() {
            return Err(Error::at(
                location,
                "qualified definition does not match a declaration",
            ));
        }
        if !function
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
        let kind = if function {
            BindingKind::Function
        } else {
            BindingKind::Variable
        };
        let binding = self.declare_value(
            name,
            ty,
            home,
            if info.home.is_some() { home } else { scope },
            kind,
            location,
        )?;
        super::record(
            &mut self.graph.declared_bindings,
            init.declarator.index(),
            binding,
        );
        let entity = self.graph.binding(binding).entity;
        if function {
            self.graph.entity_mut(entity).constexpr |= specs.constexpr;
            self.record_function_exceptions(entity, &info, home, location)?;
        } else if self.graph.scope(home).kind == ScopeKind::Class && !specs.is_static {
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
        if !function
            && self.graph.scope(home).kind != ScopeKind::Class
            && (!specs.is_extern || init.initializer.is_some())
        {
            let entity = self.graph.binding(binding).entity;
            if self.graph.entity(entity).defined {
                return Err(Error::at(location, "redefinition of a variable"));
            }
            self.graph.entity_mut(entity).defined = true;
        }
        let member = self.graph.scope(home).kind == ScopeKind::Class && !specs.is_static;
        let definition = self.graph.scope(home).kind != ScopeKind::Class
            && (!specs.is_extern || init.initializer.is_some());
        if !function && (member || definition) {
            let entity = self.graph.binding(binding).entity;
            let object = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("object without type"))?;
            self.check_object_type(object, member, location)?;
        }
        if !function && let Some(initializer) = init.initializer {
            self.initializer_names(initializer, home, ty)?;
            if !member {
                self.record_constant_initializer(
                    self.graph.binding(binding).entity,
                    initializer,
                    home,
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
            && let Some(name) = info.name
            && let Some(binding) = self.graph.own(home, name, Filter::Ordinary)
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
        name: NameId,
        ty: TypeId,
        home: ScopeId,
        print: ScopeId,
        kind: BindingKind,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        let mut entity = None;
        if let Some(existing) = self.graph.own(home, name, Filter::Ordinary) {
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
                    let a = self.graph.types.canonical_function(old)?;
                    let b = self.graph.types.canonical_function(ty)?;
                    if a != b {
                        return Err(Error::Missing(
                            "function overload sets and incompatible redeclaration diagnostics",
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
            self.graph.add_entity(Some(ty), Some(name), Keyword::Int)?
        };
        self.graph.add_binding(
            home,
            Some(print),
            Binding {
                kind,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Int,
            },
        )
    }
    fn new_class(&mut self, key: Keyword, name: NameId) -> Result<EntityId, Error> {
        let entity = self.graph.add_entity(None, Some(name), key)?;
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
        let existing = if let Some(prefix) = prefix {
            self.qualified(prefix, key_name, Filter::Elaborated)?
        } else if lone {
            self.graph.own(scope, key_name, Filter::Elaborated)
        } else {
            self.unqualified(scope, key_name, Filter::Elaborated)?
        };
        let mut declaration_home = scope;
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
            self.new_class(key, key_name)?
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class without type"))?;
        if lone || existing.is_none() {
            self.graph.add_binding(
                declaration_home,
                Some(declaration_home),
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
        let name = node
            .name
            .ok_or(Error::Missing("anonymous class and union naming"))?;
        let qualified_name = *self.tree.names.get(name);
        let prefix = self.prefix(name, scope)?;
        let home = prefix.unwrap_or(scope);
        if prefix.is_some() && !self.graph.encloses(scope, home) {
            return Err(Error::at(
                qualified_name.location,
                "class definition in a scope that does not enclose it",
            ));
        }
        let key_name = self.declarator_class_name(name)?;
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
            self.new_class(node.key, key_name)?
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
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class without type"))?;
        self.graph.add_binding(
            home,
            Some(home),
            Binding {
                kind: BindingKind::Type,
                name: Some(key_name),
                entity,
                ty: Some(ty),
                key: node.key,
            },
        )?;
        let class = self
            .graph
            .add_scope(ScopeKind::Class, home, Some(home), Some(key_name))?;
        self.graph.entity_mut(entity).scope = Some(class);
        self.graph.scope_mut(class).owner = Some(entity);
        if node.packing != 0 || !node.attributes.is_empty() {
            self.graph.entity_mut(entity).layout = super::layout::LayoutState::Unsupported;
        }
        self.graph.entity_mut(entity).defined = true;
        self.graph.add_binding(
            class,
            None,
            Binding {
                kind: BindingKind::Type,
                name: Some(key_name),
                entity,
                ty: Some(ty),
                key: node.key,
            },
        )?;
        self.class_depth += 1;
        self.declarations(node.members, class)?;
        self.class_depth -= 1;
        self.graph.entity_mut(entity).complete = true;
        if self.class_depth == 0 {
            let bodies = std::mem::take(&mut self.deferred);
            for body in bodies {
                self.function_body(body)?;
            }
            self.finish_class_exceptions()?;
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
