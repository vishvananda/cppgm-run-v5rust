// Translated from sema/analyzer.cpp, class_templates.cpp and template_names.cpp. See NOTICE.
//! Parameter scopes own names; template heads retain syntax and positional identity.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    lookup::Filter,
    types::{Cv, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::ast,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct HeadId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct ShapeId(u32);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ParameterShape {
    non_type: bool,
    pack: bool,
    inner: Option<ShapeId>,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Position {
    depth: u32,
    index: u16,
    shape: ParameterShape,
}
#[derive(Clone, Copy)]
pub(super) struct ParameterInfo {
    position: Position,
    pub(super) inner: Option<HeadId>,
    pub(super) value_type: Option<TypeId>,
}
impl ParameterInfo {
    pub(super) fn same_argument_kind(self, other: Self) -> bool {
        self.value_type == other.value_type
            && self.inner.is_some() == other.inner.is_some()
            && self.is_pack() == other.is_pack()
    }
    pub(super) fn is_pack(self) -> bool {
        self.position.shape.pack
    }
    pub(super) fn depth(self) -> u32 {
        self.position.depth
    }
    pub(super) fn abi_index(self) -> u32 {
        // Ordinary signature mangling uses T_, T0_, ... at every lexical
        // depth. Only bound pack patterns encode a template parameter level.
        u32::from(self.position.index)
    }
    pub(super) fn index(self, depth: u32) -> Option<usize> {
        (self.position.depth == depth).then_some(self.argument_index())
    }
    pub(super) fn argument_index(self) -> usize {
        usize::from(self.position.index)
    }
}
#[derive(Clone, Copy)]
pub(super) struct DefaultArgument {
    pub syntax: ast::TemplateArgumentKind,
    pub scope: ScopeId,
    pub limit: usize,
    pub bound: Option<TypeId>,
}
struct Head {
    depth: u32,
    limit: usize,
    scope: ScopeId,
    syntax: ast::TemplateParameterClause,
    parameters: Vec<EntityId>,
    shape: ShapeId,
    defaults: Vec<Option<DefaultArgument>>,
}
#[derive(Default)]
pub(super) struct Templates {
    heads: Vec<Head>,
    shapes: HashMap<Vec<(ParameterShape, Option<TypeId>)>, ShapeId>,
    positions: HashMap<Position, EntityId>,
    synthesized_values: Vec<TypeId>,
    unique_names: Vec<TypeId>,
    partial_orderings: HashMap<(EntityId, EntityId), bool>,
}
impl Templates {
    pub(super) fn partial_ordering(&self, first: EntityId, second: EntityId) -> Option<bool> {
        self.partial_orderings.get(&(first, second)).copied()
    }
    pub(super) fn record_partial_ordering(
        &mut self,
        first: EntityId,
        second: EntityId,
        ordered: bool,
    ) {
        self.partial_orderings
            .entry((first, second))
            .or_insert(ordered);
    }
    pub(super) fn parameter_scope(&self, id: HeadId) -> ScopeId {
        self.head(id).scope
    }
    pub(super) fn parameters(&self, id: HeadId) -> &[EntityId] {
        &self.head(id).parameters
    }
    pub(super) fn default_argument(&self, id: HeadId, index: usize) -> Option<DefaultArgument> {
        self.head(id).defaults[index]
    }
    pub(super) fn definition_limit(&self, id: HeadId) -> usize {
        self.head(id).limit
    }
    pub(super) fn depth(&self, id: HeadId) -> u32 {
        // A head's first parameter has the lexical depth of its scope.
        self.head(id).depth
    }
    pub(super) fn shape(&self, id: HeadId) -> ShapeId {
        self.head(id).shape
    }
    fn head(&self, id: HeadId) -> &Head {
        &self.heads[id.0 as usize]
    }
    fn head_mut(&mut self, id: HeadId) -> &mut Head {
        &mut self.heads[id.0 as usize]
    }
    fn add_head(
        &mut self,
        mut head: Head,
        shape: Vec<(ParameterShape, Option<TypeId>)>,
    ) -> Result<HeadId, Error> {
        head.shape = if let Some(&id) = self.shapes.get(&shape) {
            id
        } else {
            let id = ShapeId(
                u32::try_from(self.shapes.len())
                    .map_err(|_| Error::Limit("template shape limit"))?,
            );
            self.shapes.insert(shape, id);
            id
        };
        let id = HeadId(
            u32::try_from(self.heads.len()).map_err(|_| Error::Limit("template head limit"))?,
        );
        self.heads.push(head);
        Ok(id)
    }
}
impl Analyzer<'_> {
    /// Ordering's unique value placeholders are positional and remain dependent.
    /// Their descriptor records the argument kind; a substitution frame retains
    /// the target parameter's own declared value type.
    pub(super) fn unique_partial_arguments(
        &mut self,
        head: HeadId,
    ) -> Result<Vec<Option<TypeId>>, Error> {
        let count = self.graph.templates.parameters(head).len();
        let mut arguments = self.unique_template_types(count)?;
        for (index, argument) in arguments.iter_mut().enumerate() {
            let parameter = self.graph.templates.parameters(head)[index];
            let info = self
                .graph
                .entity(parameter)
                .template_parameter
                .ok_or(Error::Invariant("ordering without parameter"))?;
            if info.inner.is_some() {
                while self.graph.templates.unique_names.len() <= index {
                    let position = u16::try_from(self.graph.templates.unique_names.len() & 0xffff)
                        .map_err(|_| Error::Invariant("truncated template index"))?;
                    let entity = self.graph.add_entity(None, None, Keyword::Class)?;
                    self.graph.entity_mut(entity).template_parameter = Some(ParameterInfo {
                        position: Position {
                            depth: 0x7fff,
                            index: position,
                            shape: ParameterShape {
                                non_type: false,
                                pack: false,
                                inner: None,
                            },
                        },
                        inner: None,
                        value_type: None,
                    });
                    let ty = self.graph.types.intern(
                        TypeKind::TemplateParameter {
                            entity,
                            template: true,
                        },
                        Cv::default(),
                    )?;
                    self.graph.entity_mut(entity).ty = Some(ty);
                    self.graph.templates.unique_names.push(ty);
                }
                *argument = Some(self.graph.templates.unique_names[index]);
                continue;
            }
            if info.value_type.is_none() {
                continue;
            }
            while self.graph.templates.synthesized_values.len() <= index {
                // The frozen parameter record stores the low 16 bits.
                let index =
                    u16::try_from(self.graph.templates.synthesized_values.len() & 0xffff)
                        .map_err(|_| Error::Invariant("truncated synthesized parameter index"))?;
                let entity = self.graph.add_entity(None, None, Keyword::Int)?;
                self.graph.entity_mut(entity).template_parameter = Some(ParameterInfo {
                    position: Position {
                        depth: 0x7fff,
                        index,
                        shape: ParameterShape {
                            non_type: true,
                            pack: false,
                            inner: None,
                        },
                    },
                    inner: None,
                    value_type: info.value_type,
                });
                let ty = self.graph.types.intern(
                    TypeKind::TemplateParameter {
                        entity,
                        template: false,
                    },
                    Cv::default(),
                )?;
                self.graph.entity_mut(entity).ty = Some(ty);
                self.graph.templates.synthesized_values.push(ty);
            }
            *argument = Some(self.graph.templates.synthesized_values[index]);
        }
        Ok(arguments)
    }
    pub(super) fn declaration_scope(&self, mut scope: ScopeId) -> ScopeId {
        while self.graph.scope(scope).kind == ScopeKind::TemplateParameters {
            let Some(parent) = self.graph.scope(scope).parent else {
                break;
            };
            scope = parent;
        }
        scope
    }
    pub(super) fn check_template_parameter_name(
        &self,
        mut scope: ScopeId,
        name: NameId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if self.template_depth == 0 {
            return Ok(());
        }
        loop {
            let current = self.graph.scope(scope);
            if current.kind == ScopeKind::Namespace {
                return Ok(());
            }
            if current.kind == ScopeKind::TemplateParameters
                && self
                    .graph
                    .own(scope, name, Filter::Ordinary)
                    .is_some_and(|id| {
                        self.graph.binding(id).kind == BindingKind::Type
                            && self
                                .graph
                                .entity(self.graph.binding(id).entity)
                                .template_parameter
                                .is_some()
                    })
            {
                return Err(Error::at(
                    location,
                    "a template parameter redeclared within its scope",
                ));
            }
            if current.template_bridge {
                return Ok(());
            }
            let Some(parent) = current.parent else {
                return Ok(());
            };
            scope = parent;
        }
    }
    pub(super) fn template_declaration(
        &mut self,
        parameters: ast::TemplateParameterClause,
        declaration: ast::DeclId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        if let ast::DeclKind::Simple { declarators, .. } =
            self.tree.declarations.get(declaration).kind
            && self.tree.init_declarators.get(declarators).len() > 1
        {
            return Err(Error::at(
                location,
                "a template declaration must declare a single entity",
            ));
        }
        if parameters.parameters.is_empty() {
            return self.explicit_specialization(declaration, scope, location);
        }
        let home = self.declaration_scope(scope);
        if !matches!(
            self.graph.scope(home).kind,
            ScopeKind::Namespace | ScopeKind::Class
        ) {
            return Err(Error::at(location, "a template declared in a block scope"));
        }
        let mut enclosing = Some(home);
        while let Some(id) = enclosing {
            let parent = self.graph.scope(id);
            if parent.kind == ScopeKind::Namespace {
                break;
            }
            if matches!(parent.kind, ScopeKind::Function | ScopeKind::Block) {
                return Err(Error::at(location, "a member template of a local class"));
            }
            enclosing = parent.parent;
        }
        if self.c_language && self.graph.scope(home).kind == ScopeKind::Namespace {
            return Err(Error::at(location, "a template with C language linkage"));
        }
        let parameter_scope =
            self.graph
                .add_scope(ScopeKind::TemplateParameters, scope, Some(scope), None)?;
        let specs = match self.tree.declarations.get(declaration).kind {
            ast::DeclKind::Simple { specifiers, .. }
            | ast::DeclKind::Function { specifiers, .. } => Some(specifiers),
            _ => None,
        };
        let saved_friend = self.namespace_friend_class;
        if self.graph.scope(home).kind == ScopeKind::Class
            && self
                .graph
                .scope(home)
                .owner
                .is_some_and(|entity| self.graph.entity(entity).template_head.is_none())
            && specs.is_some_and(|specs| {
                self.tree
                    .specs
                    .get(specs)
                    .iter()
                    .any(|spec| matches!(spec.kind, ast::SpecifierKind::Keyword(Keyword::Friend)))
            })
        {
            self.namespace_friend_class = Some(home);
        }
        let result = (|| {
            let head = self.template_parameters(parameters, parameter_scope, true)?;
            let saved = self.template_head.replace(head);
            self.template_depth += 1;
            let saved_owner = self.template_owner;
            let result = (|| {
                self.template_owner = self.qualifying_template(declaration, parameter_scope)?;
                self.declaration(declaration, parameter_scope)
            })();
            self.template_owner = saved_owner;
            self.template_depth -= 1;
            self.template_head = saved;
            result
        })();
        self.namespace_friend_class = saved_friend;
        result
    }

    fn template_parameters(
        &mut self,
        syntax: ast::TemplateParameterClause,
        scope: ScopeId,
        declare: bool,
    ) -> Result<HeadId, Error> {
        self.check_stack()?;
        // Bound argument frames share the lookup scope kind, but introduce
        // no template parameter list (Scope::lists in the frozen compiler).
        self.graph.scope_mut(scope).template_lists = 1;
        let mut depth = 0u32;
        let mut enclosing = self.graph.scope(scope).parent;
        while let Some(id) = enclosing {
            let current = self.graph.scope(id);
            depth = depth
                .checked_add(current.template_lists)
                .ok_or(Error::Limit("template parameter depth limit"))?;
            enclosing = current.parent;
        }
        let mut parameters = Vec::new();
        let mut shapes = Vec::new();
        let mut defaults = Vec::new();
        for index in 0..self.tree.template_parameters.get(syntax.parameters).len() {
            let syntax_item = self.tree.template_parameters.get(syntax.parameters)[index];
            let parameter = &syntax_item;
            let pack = parameter.pack
                || matches!(parameter.kind,
                    ast::TemplateParameterKind::NonType { declarator: Some(id), .. }
                    if self.tree.declarators.get(id).pack);
            let (key, name, inner_syntax, default, value_type) = match parameter.kind {
                ast::TemplateParameterKind::Type {
                    key,
                    name,
                    parameters,
                    default,
                } => (
                    key,
                    name,
                    parameters,
                    default.map(ast::TemplateArgumentKind::Type),
                    None,
                ),
                ast::TemplateParameterKind::NonType {
                    specifiers,
                    declarator,
                    default,
                } => {
                    self.template_depth += 1;
                    let result = self.non_type_parameter_type(
                        specifiers,
                        declarator,
                        scope,
                        parameter.location,
                    );
                    self.template_depth -= 1;
                    (
                        Keyword::Int,
                        self.declarator_name(declarator)?,
                        None,
                        default.map(ast::TemplateArgumentKind::Expression),
                        Some(result?),
                    )
                }
            };
            if inner_syntax.is_some() && key == Keyword::Typename {
                return Err(Error::at(
                    parameter.location,
                    "a template template-parameter introduced by typename",
                ));
            }
            let inner = if let Some(syntax) = inner_syntax {
                let own = self
                    .graph
                    .add_scope(ScopeKind::TemplateParameters, scope, None, None)?;
                Some(self.template_parameters(syntax, own, false)?)
            } else {
                None
            };
            if let Some(name) = name {
                if self.graph.own(scope, name, Filter::Ordinary).is_some() {
                    return Err(Error::at(
                        parameter.location,
                        "template parameter redeclared",
                    ));
                }
                if declare && let Some(parent) = self.graph.scope(scope).parent {
                    self.check_template_parameter_name(parent, name, parameter.location)?;
                }
            }
            let bound = if declare && let Some(default) = default {
                self.check_parameter_default(default, inner, scope, parameter.location)?
            } else {
                None
            };
            if pack && default.is_some() {
                return Err(Error::at(
                    parameter.location,
                    "a template parameter pack with a default argument",
                ));
            }
            let shape = ParameterShape {
                non_type: value_type.is_some(),
                pack,
                inner: inner.map(|id| self.graph.templates.head(id).shape),
            };
            let position = Position {
                depth,
                // The C++ parameter record deliberately stores the low 16 bits.
                // Preserve that target truncation independently of host size.
                index: u16::try_from(index & 0xffff)
                    .map_err(|_| Error::Invariant("truncated template parameter index"))?,
                shape,
            };
            let entity = self.graph.add_entity(None, name, key)?;
            let canonical = if let Some(&canonical) = self.graph.templates.positions.get(&position)
            {
                canonical
            } else {
                let canonical = self.graph.add_entity(None, None, key)?;
                self.graph.templates.positions.insert(position, canonical);
                canonical
            };
            self.graph.entity_mut(entity).template_parameter = Some(ParameterInfo {
                position,
                inner,
                value_type,
            });
            if self.graph.entity(canonical).template_parameter.is_none() {
                self.graph.entity_mut(canonical).template_parameter =
                    self.graph.entity(entity).template_parameter;
            }
            self.graph.types.record_parameter_entity(entity, canonical);
            let ty = self.graph.types.intern(
                TypeKind::TemplateParameter {
                    entity,
                    template: inner.is_some(),
                },
                Cv::default(),
            )?;
            self.graph.entity_mut(entity).ty = Some(ty);
            if self.graph.entity(canonical).ty.is_none() {
                let positional = self.graph.types.intern(
                    TypeKind::TemplateParameter {
                        entity: canonical,
                        template: inner.is_some(),
                    },
                    Cv::default(),
                )?;
                self.graph.entity_mut(canonical).ty = Some(positional);
            }
            if name.is_some() {
                self.graph.add_binding(
                    scope,
                    declare.then_some(scope),
                    Binding {
                        kind: if value_type.is_some() {
                            BindingKind::TemplateValue
                        } else {
                            BindingKind::Type
                        },
                        name,
                        entity,
                        ty: Some(value_type.unwrap_or(ty)),
                        key,
                    },
                )?;
            }
            parameters.push(entity);
            shapes.push((
                shape,
                value_type
                    .map(|ty| self.graph.types.canonical_function(ty))
                    .transpose()?,
            ));
            defaults.push(default.map(|syntax| DefaultArgument {
                syntax,
                scope,
                limit: self.graph.binding_limit(),
                bound,
            }));
        }
        self.graph.templates.add_head(
            Head {
                depth,
                limit: self.graph.binding_limit(),
                scope,
                syntax,
                parameters,
                shape: ShapeId(0),
                defaults,
            },
            shapes,
        )
    }
    fn check_parameter_default(
        &mut self,
        default: ast::TemplateArgumentKind,
        inner: Option<HeadId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Option<TypeId>, Error> {
        if let ast::TemplateArgumentKind::Type(ty) = default
            && inner.is_some()
        {
            let argument = ast::TemplateArgument {
                kind: ast::TemplateArgumentKind::Type(ty),
                pack: false,
            };
            let name = self.plain_template_name(argument).map(|(name, _)| name);
            if let Some(name) = name
                && self.template_name_argument(name, scope, true)?.is_some()
            {
                return Ok(None);
            }
            return Err(Error::at(
                location,
                "a template template-parameter's default names no template",
            ));
        }
        self.template_depth += 1;
        // Member-template heads share immutable syntax; formation facts are private.
        let result = (|| {
            let copy = self.copy_template_argument(default, scope)?;
            match copy.kind {
                ast::TemplateArgumentKind::Expression(expression) => {
                    self.expression(expression, scope).map(|()| None)
                }
                ast::TemplateArgumentKind::Type(ty) => self.type_syntax(ty, scope).map(Some),
            }
        })();
        self.template_depth -= 1;
        let ty = result?;
        Ok(ty.filter(|&ty| {
            self.graph
                .scope(scope)
                .parent
                .is_some_and(|parent| self.graph.scope(parent).kind == ScopeKind::Class)
                && !self.graph.dependent_type(ty)
        }))
    }
    pub(super) fn class_template_head(
        &mut self,
        entity: EntityId,
        scope: ScopeId,
        existed: bool,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let current = self.member_template_head(scope, self.graph.entity(entity).home);
        let earlier = self.graph.entity(entity).template_head;
        let Some(head) = current else {
            if earlier.is_some() {
                return Err(Error::at(
                    location,
                    "a class template redeclared as a class that is not a template",
                ));
            }
            return Ok(());
        };
        if existed && earlier.is_none() {
            return Err(Error::at(
                location,
                "a class template redeclares a class that is not a template",
            ));
        }
        if self.graph.entity(entity).name.is_some_and(|name| {
            self.graph
                .own(self.declaration_scope(scope), name, Filter::Ordinary)
                .is_some_and(|id| self.graph.binding(id).kind != BindingKind::Type)
        }) {
            return Err(Error::at(
                location,
                "a class template shares its name with a declaration in its scope",
            ));
        }
        self.merge_template_head(entity, head, location)?;
        self.check_template_defaults(earlier.unwrap_or(head), location)
    }
    pub(super) fn check_friend_template_defaults(&self, scope: ScopeId) -> Result<(), Error> {
        let Some(head) = self.active_template_head(scope) else {
            return Ok(());
        };
        for parameter in self
            .tree
            .template_parameters
            .get(self.graph.templates.head(head).syntax.parameters)
        {
            let defaulted = match parameter.kind {
                ast::TemplateParameterKind::Type { default, .. } => default.is_some(),
                ast::TemplateParameterKind::NonType { default, .. } => default.is_some(),
            };
            if defaulted {
                return Err(Error::at(
                    parameter.location,
                    "a default template argument in a friend class template declaration",
                ));
            }
        }
        Ok(())
    }
    fn check_template_defaults(&self, head: HeadId, location: SourceOffset) -> Result<(), Error> {
        let head = self.graph.templates.head(head);
        let mut defaulted = false;
        for (index, &entity) in head.parameters.iter().enumerate() {
            let info = self
                .graph
                .entity(entity)
                .template_parameter
                .ok_or(Error::Invariant("head without parameter"))?;
            if info.position.shape.pack && index + 1 != head.parameters.len() {
                return Err(Error::at(
                    location,
                    "a class template parameter pack is not last",
                ));
            }
            if head.defaults[index].is_some() {
                defaulted = true;
            } else if defaulted && !info.position.shape.pack {
                return Err(Error::at(
                    location,
                    "a template parameter without a default follows one with a default",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn declare_alias_template(
        &mut self,
        name: NameId,
        ty: TypeId,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<BindingId, Error> {
        self.check_template_parameter_name(scope, name, location)?;
        let home = self.declaration_scope(scope);
        let head = self
            .active_template_head(scope)
            .ok_or(Error::Invariant("alias template without head"))?;
        if let Some(earlier) = self.graph.own(home, name, Filter::Ordinary) {
            let binding = self.graph.binding(earlier);
            let previous = self.graph.entity(binding.entity).template_head;
            if binding.kind == BindingKind::Alias
                && previous.is_some()
                && self.graph.scope(home).kind != ScopeKind::Class
            {
                let previous = previous.ok_or(Error::Invariant("alias template lost head"))?;
                let previous_ty = binding
                    .ty
                    .ok_or(Error::Invariant("alias template without type"))?;
                if !self.same_template_shape(previous, head)
                    || self.graph.types.canonical_function(previous_ty)?
                        != self.graph.types.canonical_function(ty)?
                {
                    return Err(Error::at(
                        location,
                        "an alias template redeclared with a different template or type",
                    ));
                }
                // The frozen alias-template path keeps its first head and
                // defaults; it does not merge defaults on redeclaration.
                return Ok(earlier);
            }
            return Err(Error::at(location, "an alias template redeclares a name"));
        }
        self.check_template_defaults(head, location)?;
        let entity = self
            .graph
            .add_entity(Some(ty), Some(name), Keyword::Typedef)?;
        self.graph.entity_mut(entity).template_head = Some(head);
        self.record_alias_parameters(entity, head, ty);
        self.graph.add_binding(
            home,
            None,
            Binding {
                kind: BindingKind::Alias,
                name: Some(name),
                entity,
                ty: Some(ty),
                key: Keyword::Typedef,
            },
        )
    }
    pub(super) fn active_template_head(&self, scope: ScopeId) -> Option<HeadId> {
        self.template_head
            .filter(|&id| self.graph.templates.head(id).scope == scope)
    }
    pub(super) fn same_template_shape(&self, first: HeadId, second: HeadId) -> bool {
        self.graph.templates.head(first).shape == self.graph.templates.head(second).shape
    }
    pub(super) fn merge_template_head(
        &mut self,
        entity: EntityId,
        head: HeadId,
        location: SourceOffset,
    ) -> Result<(), Error> {
        let earlier = self.graph.entity(entity).template_head;
        if let Some(earlier) = earlier {
            let previous = self.graph.templates.head(earlier);
            let again = self.graph.templates.head(head);
            if previous.parameters.len() != again.parameters.len() {
                return Err(Error::at(
                    location,
                    "template redeclared with a different template parameter list",
                ));
            }
            for index in 0..again.parameters.len() {
                let old = self
                    .graph
                    .entity(previous.parameters[index])
                    .template_parameter
                    .ok_or(Error::Invariant("head without parameter"))?;
                let new = self
                    .graph
                    .entity(again.parameters[index])
                    .template_parameter
                    .ok_or(Error::Invariant("head without parameter"))?;
                if old.position.shape != new.position.shape
                    || old.value_type.map(|ty| self.graph.types.canonical_view(ty))
                        != new.value_type.map(|ty| self.graph.types.canonical_view(ty))
                {
                    let location =
                        self.tree.template_parameters.get(again.syntax.parameters)[index].location;
                    return Err(Error::at(
                        location,
                        "template redeclared with a different template parameter list",
                    ));
                }
            }
            for index in 0..self.graph.templates.head(head).defaults.len() {
                let default = self.graph.templates.head(head).defaults[index];
                if let Some(default) = default {
                    if self.graph.templates.head(earlier).defaults[index].is_some() {
                        let location = self
                            .tree
                            .template_parameters
                            .get(self.graph.templates.head(head).syntax.parameters)[index]
                            .location;
                        return Err(Error::at(location, "default template argument redefined"));
                    }
                    self.graph.templates.head_mut(earlier).defaults[index] = Some(default);
                }
            }
        } else {
            self.graph.entity_mut(entity).template_head = Some(head);
        }
        Ok(())
    }
}
