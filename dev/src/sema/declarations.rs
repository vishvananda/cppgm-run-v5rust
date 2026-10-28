// Translated from sema/analyzer.cpp, analyzer_declarations.cpp and analyzer_classes.cpp. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::{Definition, ValueKey},
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::{Filter, Prefix},
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
pub(super) struct ValueScope<'info> {
    pub qualified: bool,
    pub pattern_friend: bool,
    // A qualified variable's earlier declaration is selected once, before
    // storage/linkage checks and publication of its new declaration.
    pub variable_previous: Option<BindingId>,
    pub variable_object: Option<(Option<ast::Initializer>, bool)>,
    pub home: ScopeId,
    pub print: ScopeId,
    pub head: Option<super::templates::HeadId>,
    // Borrowed only while declaring a value; the limit predates publication.
    pub function: Option<(&'info super::declarators::DeclaratorInfo, usize)>,
    pub definition: Definition,
    pub declarator_location: SourceOffset,
}
impl Analyzer<'_> {
    pub(super) fn check_construct_redeclaration(
        &self,
        scope: ScopeId,
        name: NameId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let block = self.graph.scope(scope);
        if !block.guarded {
            return Ok(());
        }
        let Some(parent) = block.parent else {
            return Ok(());
        };
        let construct = self.graph.scope(parent);
        let mut entry = construct.index.get(&name);
        if entry.is_none_or(|entry| entry.value.is_none())
            && construct.guarded
            && let Some(outer) = construct.parent
            && self.graph.scope(outer).entry == super::graph::ScopeEntry::SelectionInit
        {
            entry = self.graph.scope(outer).index.get(&name);
        }
        if entry.is_some_and(|entry| entry.value.is_some()) {
            return Err(Error::at(
                location,
                "redeclaration of a name of the enclosing construct",
            ));
        }
        Ok(())
    }

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
        if matches!(
            declaration.kind,
            ast::DeclKind::Simple {
                decomposition: true,
                ..
            }
        ) {
            return self.structured_binding_declaration(specs, declarators, scope, location);
        }
        if specs.friendship == super::specifiers::Friendship::Friend && declarators.is_empty() {
            self.friend_type(specs.ty, scope, location)?;
            return Ok(());
        }
        if declarators.is_empty()
            && let TypeKind::Class(class) = self.graph.types.get(specs.ty).kind
        {
            let tag_location = self
                .tree
                .specs
                .get(specifiers)
                .iter()
                .find_map(|specifier| {
                    if let ast::SpecifierKind::Elaborated { name, .. } = specifier.kind {
                        Some(self.tree.names.get(name).location)
                    } else {
                        None
                    }
                })
                .unwrap_or(location);
            let redeclared = self
                .graph
                .entity(class)
                .home
                .zip(self.graph.entity(class).name)
                .and_then(|(home, name)| self.graph.declared_name(home, name, Filter::Elaborated))
                .is_some_and(|binding| self.graph.binding_redeclared(binding));
            self.class_abi_tags(
                class,
                declaration.attributes,
                true,
                redeclared,
                tag_location,
            )?;
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
        for syntax_index in 0..self.tree.init_declarators.get(declarators).len() {
            let syntax_item = self.tree.init_declarators.get(declarators)[syntax_index];
            let init = &syntax_item;
            let attributes =
                self.declarator_layout_attributes(attributes, init.declarator, scope)?;
            self.resume_declaration_access(scope);
            self.declare_init(specs, *init, scope, location, &mut deduced)?;
            self.apply_storage_attributes(init.declarator, specs, attributes)?;
            // A partial variable template has no declared entity here. The
            // frozen owner checks standard noreturn only on a typedef or an
            // actual variable, and applies effects only to a function.
            if let Some(binding) = self
                .graph
                .declared_bindings
                .get(init.declarator.index())
                .copied()
                .flatten()
                .map(|binding| self.graph.binding(binding))
            {
                let function = (binding.kind == BindingKind::Function).then_some(binding.entity);
                self.declaration_return_attributes(
                    function,
                    declaration.attributes,
                    init.declarator,
                    location,
                )?;
            }
            self.declarator_symbol_attributes(init.declarator, declaration.attributes)?;
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
        attributes: List<ast::Attribute>,
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
        self.namespace_abi_tags(target, attributes, name, location)?;
        self.declarations(declarations, target)
    }
    fn new_namespace(
        &mut self,
        name: Option<NameId>,
        inline: bool,
        scope: ScopeId,
    ) -> Result<ScopeId, Error> {
        let key = if let Some(name) = name {
            name
        } else {
            self.identifiers
                .intern(b"<unnamed namespace>")
                .map_err(Error::Limit)?
        };
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
                name: Some(key),
                entity,
                ty: None,
                key: Keyword::Namespace,
            },
        )?;
        if inline || name.is_none() {
            self.graph.add_edge(scope, target, false)?;
        }
        if inline {
            self.graph.add_edge(scope, target, true)?;
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
        attributes: List<ast::Attribute>,
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
            self.graph.add_edge(scope, target, false)?;
            return Ok(());
        }
        let name = *self.tree.names.get(target);
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
        let components = self.tree.name_lists.get(name.components);
        let last = *components
            .last()
            .ok_or(Error::Invariant("empty using name"))?;
        if matches!(
            last.kind,
            ast::NameKind::Destructor(_)
                | ast::NameKind::DestructorDecltype(_)
                | ast::NameKind::Decltype(_)
        ) {
            return Err(Error::at(
                location,
                "a using-declaration cannot name a destructor",
            ));
        }
        let count = components.len().saturating_sub(1);
        let names_owner = count != 0
            && matches!((components[count - 1].kind, last.kind),
                (ast::NameKind::Identifier(a), ast::NameKind::Identifier(b)) if a == b)
            && self.graph.scope(self.declaration_scope(scope)).kind == ScopeKind::Class;
        let prefix = match self.name_prefix(target, scope, count, true)? {
            Some(Prefix::Scope(prefix)) => prefix,
            Some(Prefix::Dependent(qualifier)) => {
                if let ast::NameKind::Identifier(name) = last.kind
                    && !names_owner
                {
                    let home = self.declaration_scope(scope);
                    if self.graph.own(home, name, Filter::Ordinary).is_none() {
                        let binding = self.dependent_member_binding(qualifier, name)?;
                        self.graph
                            .add_binding(home, None, self.graph.binding(binding))?;
                    }
                }
                return Ok(());
            }
            None => return Err(Error::Invariant("qualified using without prefix")),
        };
        if let ast::NameKind::Conversion(ty) = last.kind {
            return self.using_conversion(prefix, ty, scope, location);
        }
        let target_key = self.value_name(target)?;
        self.lookup_location = name.location;
        if self.note_inherited_constructors(scope, prefix, target_key, target)? {
            return Ok(());
        }
        let identifier = matches!(last.kind, ast::NameKind::Identifier(_));
        let mut found = super::lookup::LookupResult::default();
        if identifier {
            self.prepare_assignment_lookup(prefix, target_key, false)?;
            found = self.qualified_declaration_result(prefix, target_key, Filter::Ordinary)?;
        }
        // The second lookup follows declared-name formation and implicit-member
        // demand. Global builtins and unscoped enumerations take their own path.
        if found.binding.is_none() && name.global && count == 0 && identifier {
            if let Some(key) = target_key.identifier()
                && let Some(builtin) = self.builtin_lookup(key)?
            {
                found = builtin;
            }
        } else if self.graph.scope(prefix).kind != ScopeKind::EnumMembers {
            self.prepare_assignment_lookup(prefix, target_key, false)?;
            found = self.using_lookup(prefix, target_key, location)?;
        }
        let Some(target_id) = found.binding else {
            // Only an absent final name is optional. Prefix resolution,
            // ambiguity, access and introduction retain their normal checks.
            if self
                .tree
                .attributes
                .get(attributes)
                .iter()
                .any(|attribute| attribute.kind == ast::AttributeKind::UsingIfExists)
            {
                return Ok(());
            }
            return Err(Error::at(
                location,
                "undeclared name in a using-declaration",
            ));
        };
        // Access is checked for the entire named set before hiding any import.
        // In particular, an inaccessible base overload cannot be excused by a
        // same-signature declaration already present in the derived class.
        if self.graph.scope(prefix).kind == ScopeKind::Class && !self.using_access_dependent(scope)
        {
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
            // Lookup lists newest first; introduce in declaration order, as
            // Analyzer::using_declaration does, so the representative entity
            // and inherited declaration-set identity remain the source's.
            for &target in found.functions.iter().rev() {
                self.introduce_function_using(target, target_key, scope, location)?;
            }
            return Ok(());
        }
        let binding = self.graph.binding(target_id);
        if let Some(name) = binding.name {
            self.check_template_parameter_name(scope, name, location)?;
        }
        if matches!(
            binding.kind,
            BindingKind::Namespace | BindingKind::NamespaceAlias
        ) {
            return Err(Error::at(
                location,
                "a using-declaration cannot name a namespace",
            ));
        }
        self.check_using_template_conflict(target_id, scope, location)?;
        let existing = if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
            binding
                .name
                .and_then(|name| self.graph.own(scope, name, Filter::Prefix))
        } else {
            self.using_value_binding(scope, target_key)
        };
        if let Some(existing) = existing
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
    pub(super) fn using_lookup(
        &self,
        prefix: ScopeId,
        key: ValueKey,
        location: SourceOffset,
    ) -> Result<super::lookup::LookupResult, Error> {
        let found = self.qualified_declaration_result(prefix, key, Filter::Ordinary)?;
        if found.ambiguous {
            return Err(Error::at(location, "ambiguous name in a using-declaration"));
        }
        Ok(found)
    }
    pub(super) fn using_access_dependent(&self, scope: ScopeId) -> bool {
        std::iter::successors(Some(scope), |&id| self.graph.scope(id).parent).any(|id| {
            let scope = self.graph.scope(id);
            scope.kind == ScopeKind::Class
                && scope.owner.is_some_and(|class| {
                    self.graph.entity(class).base_dependency == ast::BaseDependency::Dependent
                })
        })
    }
    fn using_value_binding(&self, scope: ScopeId, key: ValueKey) -> Option<BindingId> {
        self.graph.value_binding(scope, key).filter(|&id| {
            matches!(
                self.graph.binding(id).kind,
                BindingKind::Variable
                    | BindingKind::Function
                    | BindingKind::Parameter
                    | BindingKind::Enumerator
            )
        })
    }
    fn check_using_template_conflict(
        &self,
        target: BindingId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let binding = self.graph.binding(target);
        let Some(name) = binding.name else {
            return Ok(());
        };
        let is_template = |id| {
            let binding = self.graph.binding(id);
            binding.kind == BindingKind::Type
                && self.graph.entity(binding.entity).template_head.is_some()
        };
        let conflict = if matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
            is_template(target) && self.using_value_binding(scope, name.into()).is_some()
        } else {
            self.graph
                .own(scope, name, Filter::Prefix)
                .is_some_and(is_template)
        };
        if conflict {
            return Err(Error::at(
                location,
                "using-declaration conflicts with a class template of the same name",
            ));
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
        if let Some(name) = binding.name {
            self.check_template_parameter_name(scope, name, location)?;
        }
        self.check_using_template_conflict(target, scope, location)?;
        if self
            .using_value_binding(scope, key)
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
                && self.same_declared_function(
                    existing,
                    binding
                        .ty
                        .ok_or(Error::Invariant("function using without type"))?,
                    self.graph.entity(binding.entity).template_head,
                )?
                && (self.graph.entity(earlier.entity).template_head.is_none()
                    || self.graph.entity(earlier.entity).home == Some(scope))
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
        if self.variable_template_declarator(specs, init, scope, &info)? {
            self.settle_declaration_access(None)?;
            return Ok(());
        }
        let member = self.declared_member(&info, scope)?;
        let home = info.home.unwrap_or(self.declaration_scope(scope));
        self.check_function_specifiers(specs, &info, init)?;
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
            self.declare_typedef_binding(init.declarator, name, info.ty, home, location)?;
            if self.graph.scope(home).kind == ScopeKind::Block {
                let binding = self.graph.declared_bindings[init.declarator.index()]
                    .ok_or(Error::Invariant("typedef without binding"))?;
                self.graph.set_binding_dependence(
                    binding,
                    self.dependent_uses.get() != specs.dependent_uses,
                );
            }
            return Ok(());
        }
        let automatic = self.holds_auto(info.ty).then_some((info.ty, specs.auto_cv));
        self.deduce_declared_object(&mut info, specs, init, home, location, deduced)?;
        let automatic = automatic.filter(|_| self.graph.dependent_type(info.ty));
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
        self.check_mutable_variable(
            specs,
            &info,
            home,
            self.tree.declarators.get(init.declarator).location,
        )?;
        let specialization =
            self.explicit_member_variable(&info, init.initializer.is_some(), location)?;
        let previous =
            self.check_data_member_declaration(specs, &info, init.initializer, home, location)?;
        let specs = self.variable_storage(specs, home, init.initializer.is_some(), location)?;
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
        let definition = specialization.unwrap_or(
            (self.graph.scope(home).kind != ScopeKind::Class || info.home.is_some())
                && (!specs.is_extern || init.initializer.is_some())
                || self.inline_static_member(specs, &info, home),
        );
        let member = self.graph.scope(home).kind == ScopeKind::Class && !specs.is_static;
        let binding = self.declare_value(
            name.into(),
            ty,
            ValueScope {
                qualified: info.home.is_some(),
                pattern_friend: false,
                variable_previous: previous,
                variable_object: (member || definition).then_some((init.initializer, member)),
                home,
                print: info.home.unwrap_or(scope),
                head: self.active_template_head(scope),
                function: None,
                definition: Definition::Declaration,
                declarator_location: self.tree.declarators.get(init.declarator).location,
            },
            BindingKind::Variable,
            location,
            specs,
        )?;
        if self.graph.scope(home).kind == ScopeKind::Block {
            self.graph
                .set_binding_dependence(binding, self.dependent_uses.get() != specs.dependent_uses);
        }
        let entity = self.publish_object_declaration(init.declarator, binding)?;
        self.note_asm_label(entity, info.asm_label)?;
        if self.semantic() {
            ty = self.graph.publish_variable_type(binding)?;
            self.note_nothrow_target(entity, ty, Some(init.declarator), info.lookup)?;
        }
        self.record_data_member(binding, specs, &info, init, home)?;
        if definition {
            if self.graph.entity(entity).defined {
                let location = self.tree.declarators.get(init.declarator).location;
                return Err(Error::at(location, "redefinition of a variable"));
            }
            self.graph.entity_mut(entity).defined = true;
            self.graph.entity_mut(entity).variable_definition = Some(init.declarator);
        }
        let member =
            self.graph.entity(entity).data_member == Some(super::graph::DataMemberKind::Instance);
        self.graph.entity_mut(entity).constexpr |= specs.constexpr;
        if specs.mutability == super::graph::Mutability::Mutable {
            self.graph.entity_mut(entity).mutability = super::graph::Mutability::Mutable;
        }
        self.declare_variable_template(binding, init, &info, scope, location, automatic)?;
        self.check_constexpr_variable(specs.constexpr, ty, location)?;
        self.check_static_member_initializer(specs, &info, init.initializer, home, location)?;
        let typed_initialization =
            self.typed_variable_initialization(specs, &info, home, init.initializer.is_some());
        if let Some(initializer) = init.initializer {
            if self.semantic() {
                if self.graph.scope(home).kind == ScopeKind::Block
                    && !specs.is_static
                    && !specs.is_extern
                    && specs.thread_storage == super::specifiers::ThreadStorage::Ordinary
                {
                    self.note_local_initializer(home)?;
                }

                if member {
                    self.defer_member_initializer(binding, info.lookup);
                } else if self.template_depth == 0 && !self.reserved_context.pattern {
                    ty = self.initialize_variable(binding, initializer, info.lookup, location)?;
                    if member || definition {
                        self.check_object_type(ty, member, location)?;
                    }
                } else {
                    self.initializer_names(initializer, info.lookup, ty)?;
                }
            } else {
                self.initializer_names(initializer, info.lookup, ty)?;
            }
            if !member && self.template_depth == 0 {
                self.record_declared_constant(entity, init, &info)?;
            }
        } else if typed_initialization && definition {
            self.default_initialize_declaration(binding, location)?;
        }
        self.record_variable_pattern(binding, init, &info, definition, ty, location);
        let storage_root = self.lowering()
            && self.template_depth == 0
            && definition
            && !member
            && !self.inline_static_member(specs, &info, home);
        if storage_root {
            self.request_static_initializer(entity, init.initializer, info.lookup, home)?;
        }
        if typed_initialization && !member && (definition || init.initializer.is_some()) {
            self.variable_destruction(binding, info.lookup, location, init.initializer.is_some())?;
        }
        if storage_root {
            self.request_storage_layout(entity, location)?;
        }
        Ok(())
    }
    fn publish_object_declaration(
        &mut self,
        declarator: ast::DeclaratorId,
        binding: super::graph::BindingId,
    ) -> Result<super::graph::EntityId, Error> {
        super::record(
            &mut self.graph.declared_bindings,
            declarator.index(),
            binding,
        );
        self.settle_declaration_access(Some(binding))?;
        let entity = self.graph.binding(binding).entity;
        self.graph
            .initializer_constructors
            .publish_deduction(declarator, entity);
        Ok(entity)
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
        if matches!(self.graph.types.get(ty).kind, TypeKind::Auto(_)) {
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
    pub(super) fn complete_initializer_array(
        &mut self,
        ty: TypeId,
        initializer: Option<ast::Initializer>,
    ) -> Result<TypeId, Error> {
        let TypeKind::Array {
            element,
            bound: 0,
            operand: None,
        } = self.graph.types.get(ty).kind
        else {
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
                    && let crate::lex::PostTokenKind::String {
                        initializer_bound, ..
                    } = literal.kind
                {
                    bound = initializer_bound;
                }
            }
            ast::ExprKind::Literal(literal) => match literal.kind {
                crate::lex::PostTokenKind::String {
                    initializer_bound, ..
                } => {
                    bound = initializer_bound;
                }
                crate::lex::PostTokenKind::Character {
                    initializer_bound, ..
                } => {
                    bound = u64::from(initializer_bound);
                }
                _ => (),
            },
            _ => (),
        }
        if bound == 0 {
            return Ok(ty);
        }
        self.graph.types.intern(
            TypeKind::Array {
                element,
                bound,
                operand: None,
            },
            Cv::default(),
        )
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
    pub(super) fn check_object_type(
        &mut self,
        mut ty: TypeId,
        member: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.graph.types.get(ty).kind
        {
            if bound == 0 && operand.is_none() && !member {
                return Err(Error::at(location, "variable of incomplete type"));
            }
            ty = element;
        }
        if let TypeKind::Class(entity) = self.graph.types.get(ty).kind
            && self.template_depth == 0
            && !self.complete_class(entity)?
        {
            return Err(Error::at(location, "variable of incomplete type"));
        }
        Ok(())
    }
    pub(super) fn check_member_function_redeclaration(
        &mut self,
        info: &super::declarators::DeclaratorInfo,
        home: ScopeId,
        head: Option<super::templates::HeadId>,
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
                if self.same_declared_function(binding, info.ty, head)?
                    && self.graph.types.canonical_function(old)?
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
        context: ValueScope<'_>,
        kind: BindingKind,
        location: SourceOffset,
        specs: Specifiers,
    ) -> Result<BindingId, Error> {
        let ValueScope {
            qualified,
            pattern_friend,
            variable_previous,
            variable_object,
            home,
            print,
            head,
            function,
            definition: _,
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
            && let Some(name) = name.identifier()
        {
            self.check_construct_redeclaration(home, name, declarator_location)?;
        }
        let mut entity = None;
        let mut existing = if pattern_friend {
            None
        } else if qualified && kind == BindingKind::Variable {
            Some(variable_previous.ok_or(Error::Invariant(
                "qualified variable without its selected declaration",
            ))?)
        } else {
            self.graph.redeclared_value(home, name)
        };
        // A dependent using's type placeholder can denote a function set in
        // an instance. An own function joins or hides that set there.
        if kind == BindingKind::Function
            && existing.is_some_and(|id| {
                self.graph
                    .dependent_members
                    .get(self.graph.binding(id).entity)
                    .is_some()
            })
        {
            existing = None;
        }
        if kind == BindingKind::Function
            && existing.is_some_and(|id| self.graph.binding(id).kind == BindingKind::Function)
        {
            existing = self.match_declared_function(home, name, ty, head, location)?;
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
                    self.agree_function_redeclaration(
                        binding.entity,
                        ty,
                        context,
                        specs,
                        location,
                    )?;
                } else if let Some(completed) = self.compatible_variable_type(old, ty)? {
                    self.agree_variable_linkage(binding.entity, specs, home, qualified, location)?;
                    self.graph.entity_mut(binding.entity).ty = Some(completed);
                } else {
                    return Err(Error::at(
                        context.declarator_location,
                        "conflicting declaration of a variable",
                    ));
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
        if kind == BindingKind::Function {
            if let Some((info, _)) = function {
                self.note_asm_label(entity, info.asm_label)?;
                self.check_template_function_defaults(info, head, existing.is_none(), location)?;
            }
            if let Some(head) = head {
                self.merge_template_head(entity, head, location)?;
            }
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
        // Complete the declared object before publishing this binding. A
        // demanded class can introduce friends into the namespace; those
        // declarations precede this variable and cannot see it yet.
        if let Some((initializer, member)) = variable_object {
            let object = self.initial_object_type(ty, initializer);
            self.check_object_type(object, member, location)?;
            self.check_abstract_object(object, location)?;
        }
        let invisible = kind == BindingKind::Function
            && specs.friendship == super::specifiers::Friendship::Friend
            && existing
                .filter(|&id| self.graph.binding(id).kind == BindingKind::Function)
                .is_none_or(|id| self.graph.friendships.hidden(id));
        let value = Binding {
            kind,
            name: name.identifier(),
            entity,
            ty: Some(ty),
            key: Keyword::Int,
        };
        let binding = if pattern_friend {
            self.graph.add_pattern_function(home, print, value)?
        } else {
            self.graph
                .add_value_binding(home, Some(print), value, invisible)?
        };
        if !pattern_friend && let Some(name) = name.special() {
            self.graph.record_special_value(home, name, binding)?;
        }
        Ok(binding)
    }
    fn match_declared_function(
        &mut self,
        home: ScopeId,
        name: ValueKey,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
        location: SourceOffset,
    ) -> Result<Option<BindingId>, Error> {
        if self.graph.scope(home).kind == ScopeKind::Namespace {
            let candidate = self.graph.indexed_function(home, name, ty, head)?;
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
            let matches = self.same_declared_function(candidate, ty, head)?
                || head.is_some_and(|head| {
                    self.graph.entity(binding.entity).home != Some(home)
                        && self
                            .graph
                            .entity(binding.entity)
                            .template_head
                            .is_some_and(|old| self.same_template_shape(head, old))
                }) && self.same_function_parameters(old, ty)?;
            if matches {
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
    pub(super) fn same_declared_function(
        &mut self,
        previous: BindingId,
        ty: TypeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<bool, Error> {
        let binding = self.graph.binding(previous);
        let old = binding
            .ty
            .ok_or(Error::Invariant("function without type"))?;
        match (head, self.graph.entity(binding.entity).template_head) {
            (Some(a), Some(b)) => Ok(self.same_template_shape(a, b)
                && self.graph.types.declared_type(old)? == self.graph.types.declared_type(ty)?),
            (None, None) => self.same_function_parameters(old, ty),
            _ => Ok(false),
        }
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
        let a = self.graph.types.declared_type(old)?;
        let b = self.graph.types.declared_type(new)?;
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
        if self.graph.cxx14
            || !specs.constexpr
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
    // analyzer_declarations.cpp::check_type_name_conflict. A class can
    // coexist with a value, but cannot replace another visible type name.
    pub(super) fn check_new_type_name(
        &self,
        home: ScopeId,
        name: NameId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self
            .graph
            .scope(home)
            .index
            .get(&name)
            .and_then(|entry| entry.ty)
            .is_some_and(|binding| !self.graph.friendships.hidden(binding))
        {
            return Err(Error::at(
                location,
                "type name conflicts with an earlier declaration",
            ));
        }
        Ok(())
    }

    pub(super) fn new_class(
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
        self.record_member_class_source(entity, name, home);
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
        let last = self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .ok_or(Error::Invariant("empty class name"))?;
        if !matches!(last.kind, ast::NameKind::Identifier(_)) {
            return Err(Error::at(node.location, "invalid class name"));
        }
        if self
            .tree
            .name_lists
            .get(node.components)
            .last()
            .is_some_and(|last| last.arguments.is_some())
        {
            if lone {
                if self.specializing() {
                    let entity = self.specialized_class(name, key, scope)?;
                    return self.graph.entity(entity).ty.ok_or(Error::Invariant(
                        "explicit class specialization without type",
                    ));
                }
                if self.active_template_head(scope).is_some() {
                    let entity = self.partial_class(name, key, scope)?;
                    return self.graph.entity(entity).ty.ok_or(Error::Invariant(
                        "class partial specialization without type",
                    ));
                }
                return Err(Error::at(
                    node.location,
                    "a class template-id declared outside a class template specialization's declaration",
                ));
            }
            return self.elaborated_specialization(key, name, scope);
        }
        let count = self
            .tree
            .name_lists
            .get(node.components)
            .len()
            .saturating_sub(1);
        let prefix = match self.name_prefix(name, scope, count, false)? {
            Some(Prefix::Dependent(qualifier)) => {
                // The elaborated specifier is checked when its retained syntax
                // sees a concrete qualifier. No dependent qualifier has a scope.
                let last = self
                    .tree
                    .name_lists
                    .get(node.components)
                    .last()
                    .ok_or(Error::Invariant("empty class name"))?;
                let key_name = Self::component_name(last.kind)?;
                let binding = self.dependent_member_binding(qualifier, key_name)?;
                return self
                    .graph
                    .binding(binding)
                    .ty
                    .ok_or(Error::Invariant("dependent class name without type"));
            }
            Some(Prefix::Scope(prefix)) => Some(prefix),
            None => None,
        };
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
            self.class_abi_tags(entity, node.attributes, false, false, node.location)?;
            self.graph.entity_mut(entity).anonymous_class =
                Some(super::anonymous_types::AnonymousClass::Pending(node.tokens));
            return self.define_class(node, entity, scope, scope);
        };
        let qualified_name = *self.tree.names.get(name);
        if self.specializing()
            && self.active_template_head(scope).is_none()
            && self
                .tree
                .name_lists
                .get(qualified_name.components)
                .last()
                .is_some_and(|last| last.arguments.is_some())
        {
            return self.class_explicit_specialization(node, name, scope);
        }
        if self.active_template_head(scope).is_some()
            && self
                .tree
                .name_lists
                .get(qualified_name.components)
                .last()
                .is_some_and(|last| last.arguments.is_some())
        {
            return self.class_partial_definition(id, name, scope);
        }
        let last = self
            .tree
            .name_lists
            .get(qualified_name.components)
            .last()
            .ok_or(Error::Invariant("empty class name"))?;
        if last.arguments.is_some() {
            return Err(Error::at(
                qualified_name.location,
                "a class template-id named outside a class template specialization's declaration",
            ));
        }
        let ast::NameKind::Identifier(key_name) = last.kind else {
            return Err(Error::at(qualified_name.location, "invalid class name"));
        };
        let prefix = self.prefix(name, scope)?;
        let home = prefix.unwrap_or(self.declaration_scope(scope));
        if prefix.is_some()
            && matches!(
                self.graph.scope(home).kind,
                ScopeKind::Enum | ScopeKind::EnumMembers
            )
        {
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
        if prefix.is_some() && self.specializing() {
            self.explicit_member_class(entity, home, scope, qualified_name.location)?;
        }
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
        self.class_abi_tags(
            entity,
            node.attributes,
            false,
            existing.is_some() || prefix.is_some(),
            qualified_name.location,
        )?;
        let print = prefix.unwrap_or(scope);
        let parent = if prefix.is_some() && self.active_template_head(scope).is_some() {
            self.member_lookup_bridge(scope, home)?
        } else {
            print
        };
        let ty = self.define_class(node, entity, parent, print)?;
        self.record_class_pattern(entity, id, parent);
        Ok(ty)
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
        if self.defer_class_member(entity, node, parent)? {
            return Ok(ty);
        }
        self.class_body(node, entity, parent, Some(home))
    }
    pub(super) fn class_body(
        &mut self,
        node: ast::ClassSyntax,
        entity: EntityId,
        parent: ScopeId,
        print: Option<ScopeId>,
    ) -> Result<TypeId, Error> {
        let name = self.graph.entity(entity).name;
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("class without type"))?;
        let class = self
            .graph
            .add_scope(ScopeKind::Class, parent, print, name)?;
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
        let first_throw = self.pending_throws.len();
        let first_override = self.exception_overrides.len();
        self.class_depth += 1;
        self.declare_class_members(entity, node.members, class)?;
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
            self.complete_virtual_members(entity, node.location)?;
            self.complete_virtual_layout(entity, node.location)?;
        }
        if self.template_depth == 0 && self.graph.entity(entity).storage_attributes.alignment != 0 {
            self.class_layout(entity, node.location)?;
        }
        if self.class_depth == 0 {
            self.finish_class_exceptions(first_throw, first_override)?;
            self.finish_class_nesting()?;
        }
        Ok(ty)
    }
}
