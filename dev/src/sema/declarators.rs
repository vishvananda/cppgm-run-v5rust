// Translated from sema/analyzer_types.cpp's declarator/ptr_operator/apply_suffix. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    graph::ScopeId,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{identifiers::NameId, pp_lexer::Punctuator, vocabulary::Keyword},
    syntax::ast::{self, SpecifierKind},
};
#[derive(Clone, Copy)]
pub(super) struct DeclaratorEnvironment {
    pub home: Option<ScopeId>,
    pub lookup: ScopeId,
    pub declaring: ScopeId,
    pub declaration_location: Option<crate::support::source::SourceOffset>,
}
pub(super) struct DeclaratorInfo {
    pub ty: TypeId,
    pub name: Option<NameId>,
    pub home: Option<ScopeId>,
    pub lookup: ScopeId,
    pub parameters: Option<ast::ParameterClause>,
    pub qualifiers: Option<crate::syntax::storage::List<ast::Qualifier>>,
    pub syntax_name: Option<ast::Name>,
}
impl Analyzer<'_> {
    pub(super) fn conversion_declarator(&self, info: &DeclaratorInfo) -> bool {
        info.syntax_name.is_some_and(|name| {
            self.tree
                .name_lists
                .get(self.tree.names.get(name).components)
                .last()
                .is_some_and(|component| matches!(component.kind, ast::NameKind::Conversion(_)))
        })
    }
    pub(super) fn declarator_name(
        &self,
        mut id: Option<ast::DeclaratorId>,
    ) -> Result<Option<NameId>, Error> {
        while let Some(current) = id {
            let node = self.tree.declarators.get(current);
            if let Some(name) = node.name {
                let last = self
                    .tree
                    .name_lists
                    .get(self.tree.names.get(name).components)
                    .last()
                    .ok_or(Error::Invariant("empty declaration name"))?;
                return match last.kind {
                    ast::NameKind::Destructor(_)
                    | ast::NameKind::DestructorDecltype(_)
                    | ast::NameKind::Conversion(_)
                    | ast::NameKind::Operator { .. }
                    | ast::NameKind::LiteralOperator(_) => Ok(None),
                    ast::NameKind::Decltype(_) => Err(Error::at(
                        self.tree.names.get(name).location,
                        "a decltype-specifier does not name a declaration",
                    )),
                    ast::NameKind::Identifier(name) => Ok(Some(name)),
                };
            }
            id = node.nested;
        }
        Ok(None)
    }
    pub(super) fn declarator_location(
        &self,
        mut id: ast::DeclaratorId,
    ) -> Result<crate::support::source::SourceOffset, Error> {
        loop {
            let node = self.tree.declarators.get(id);
            if let Some(name) = node.name {
                return Ok(self.tree.names.get(name).location);
            }
            let Some(nested) = node.nested else {
                return Ok(node.location);
            };
            id = nested;
        }
    }
    pub(super) fn type_syntax(&mut self, id: ast::TypeId, scope: ScopeId) -> Result<TypeId, Error> {
        if (self.evaluating_noexcept || self.semantic())
            && let Some(ty) = self.graph.type_fact(id)
        {
            return Ok(ty);
        }
        let node = *self.tree.types.get(id);
        let specs = self.specifiers(node.specifiers, scope, false)?;
        let ty = self.declarator(node.declarator, specs.ty, scope)?.ty;
        super::record(&mut self.graph.type_facts, id.index(), ty);
        Ok(ty)
    }
    pub(super) fn declarator_environment(
        &mut self,
        id: Option<ast::DeclaratorId>,
        scope: ScopeId,
    ) -> Result<DeclaratorEnvironment, Error> {
        let mut home = None;
        let mut after = scope;
        // Qualified declarator-id controls lookup in subsequent suffixes.
        let mut probe = id;
        while let Some(current) = probe {
            let node = self.tree.declarators.get(current);
            if let Some(named) = node.name {
                home = self.prefix(named, scope)?;
                after = if let Some(home) = home {
                    if self.active_template_head(scope).is_some() {
                        self.qualified_template_scope(scope, home)?
                    } else {
                        home
                    }
                } else {
                    scope
                };
                break;
            }
            probe = node.nested;
        }
        Ok(DeclaratorEnvironment {
            home,
            lookup: after,
            declaring: self.declaration_scope(scope),
            declaration_location: None,
        })
    }
    pub(super) fn declarator(
        &mut self,
        id: Option<ast::DeclaratorId>,
        ty: TypeId,
        scope: ScopeId,
    ) -> Result<DeclaratorInfo, Error> {
        let environment = self.declarator_environment(id, scope)?;
        self.declarator_in(id, ty, environment)
    }
    pub(super) fn declarator_in(
        &mut self,
        id: Option<ast::DeclaratorId>,
        ty: TypeId,
        environment: DeclaratorEnvironment,
    ) -> Result<DeclaratorInfo, Error> {
        self.declarator_view(id, ty, environment, 0)
    }

    fn declarator_view(
        &mut self,
        mut id: Option<ast::DeclaratorId>,
        mut ty: TypeId,
        environment: DeclaratorEnvironment,
        root_suffix_skip: usize,
    ) -> Result<DeclaratorInfo, Error> {
        let root = id;
        let mut name = None;
        let mut syntax_name = None;
        let mut parameters = None;
        let mut function_qualifiers = None;
        let DeclaratorEnvironment {
            home,
            lookup: after,
            declaring,
            declaration_location,
        } = environment;
        if home.is_some_and(|home| !self.graph.encloses(declaring, home)) {
            return Err(Error::at(
                declaration_location.unwrap_or(
                    self.tree
                        .declarators
                        .get(id.ok_or(Error::Invariant("missing qualified declarator"))?)
                        .location,
                ),
                if self.graph.scope(declaring).kind == super::graph::ScopeKind::Namespace {
                    "qualified definition in a scope that does not enclose the member"
                } else {
                    "qualified name in a declaration outside namespace scope"
                },
            ));
        }
        while let Some(current) = id {
            self.check_stack()?;
            let node = *self.tree.declarators.get(current);
            if node.pack {
                return Err(Error::Missing("semantic parameter packs"));
            }
            if node.name.is_some() {
                name = self.declarator_name(Some(current))?;
                syntax_name = node.name;
            }
            for pointer in self.tree.pointers.get(node.pointers) {
                parameters = None;
                function_qualifiers = None;
                let cv = Self::cv(self.tree.cvs.get(pointer.cv));
                if let Some(owner) = pointer.class_name {
                    let binding =
                        self.lookup(owner, environment.lookup, super::lookup::Filter::Prefix)?;
                    let binding = self.graph.binding(binding);
                    let owner = if matches!(
                        binding.kind,
                        super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                    ) {
                        binding.ty
                    } else {
                        None
                    };
                    let owner = owner.ok_or_else(|| {
                        Error::at(node.location, "pointer to member of a non-class")
                    })?;
                    let owner = self.bare_type(owner)?;
                    if !matches!(
                        self.graph.types.get(owner).kind,
                        TypeKind::Class(_) | TypeKind::TemplateParameter { .. }
                    ) {
                        return Err(Error::at(node.location, "pointer to member of a non-class"));
                    }
                    match self.graph.types.get(ty).kind {
                        TypeKind::Reference { .. } => {
                            return Err(Error::at(
                                node.location,
                                "pointer to member of reference type",
                            ));
                        }
                        TypeKind::Scalar(Scalar::Void) => {
                            return Err(Error::at(node.location, "pointer to member of type void"));
                        }
                        _ => (),
                    }
                    let member = self.graph.types.member_pointer(owner, ty)?;
                    ty = self.graph.types.qualify(member, cv)?;
                    continue;
                }
                match pointer.op.token.as_punctuator() {
                    Some(Punctuator::Multiply) => {
                        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                            return Err(Error::at(node.location, "pointer to reference"));
                        }
                        let ptr = self.graph.types.pointer(ty)?;
                        ty = self.graph.types.qualify(ptr, cv)?;
                    }
                    Some(Punctuator::BitAnd | Punctuator::LogicalAnd) => {
                        if cv.0 != 0 {
                            return Err(Error::at(node.location, "cv-qualified reference"));
                        }
                        if matches!(
                            self.graph.types.get(ty).kind,
                            TypeKind::Scalar(Scalar::Void)
                        ) {
                            return Err(Error::at(node.location, "reference to void"));
                        }
                        let kind = if pointer.op.token.as_punctuator() == Some(Punctuator::BitAnd) {
                            RefKind::Lvalue
                        } else {
                            RefKind::Rvalue
                        };
                        ty = self.graph.types.reference(ty, kind)?;
                    }
                    _ => return Err(Error::Missing("extended declarator pointer operator")),
                }
            }
            let skip = if Some(current) == root {
                root_suffix_skip
            } else {
                0
            };
            for suffix in self
                .tree
                .suffixes
                .get(node.suffixes)
                .iter()
                .skip(skip)
                .rev()
            {
                match *suffix {
                    ast::Suffix::Array(bound) => {
                        parameters = None;
                        function_qualifiers = None;
                        let failure = match self.graph.types.get(ty).kind {
                            TypeKind::Reference { .. } => Some("array of references"),
                            TypeKind::Function { .. } => Some("array of functions"),
                            TypeKind::Scalar(Scalar::Void) => Some("array of void"),
                            TypeKind::Array { bound: 0, .. } => {
                                Some("array of arrays of unknown bound")
                            }
                            _ => None,
                        };
                        if let Some(message) = failure {
                            return Err(Error::at(node.location, message));
                        }
                        let bound = bound
                            .map(|bound| self.array_bound(bound, after))
                            .transpose()?
                            .unwrap_or(0);
                        ty = self
                            .graph
                            .types
                            .intern(TypeKind::Array { element: ty, bound }, Cv::default())?;
                    }
                    ast::Suffix::Function {
                        parameters: clause,
                        qualifiers,
                    } => {
                        ty = self.function_type(ty, clause, qualifiers, after, node.location)?;
                        parameters = Some(clause);
                        function_qualifiers = Some(qualifiers);
                    }
                }
            }
            id = node.nested;
        }
        // A suffix view is a query result, not the original declarator's fact.
        if root_suffix_skip == 0
            && let Some(root) = root
        {
            super::record(&mut self.graph.declarator_types, root.index(), ty);
        }
        Ok(DeclaratorInfo {
            ty,
            name,
            home,
            lookup: after,
            parameters,
            qualifiers: function_qualifiers,
            syntax_name,
        })
    }

    pub(super) fn allocation_key_type(
        &mut self,
        id: ast::TypeId,
        scope: ScopeId,
    ) -> Result<Option<(TypeId, Option<ast::ExprId>)>, Error> {
        let syntax = self.tree.types.get(id);
        if syntax.declarator.is_some_and(|id| {
            matches!(
                self.tree
                    .suffixes
                    .get(self.tree.declarators.get(id).suffixes)
                    .first(),
                Some(ast::Suffix::Array(None))
            )
        }) {
            return Ok(None);
        }
        self.allocation_type(id, scope).map(Some)
    }
    pub(super) fn allocation_type(
        &mut self,
        id: ast::TypeId,
        scope: ScopeId,
    ) -> Result<(TypeId, Option<ast::ExprId>), Error> {
        let syntax = *self.tree.types.get(id);
        let first = syntax.declarator.and_then(|id| {
            self.tree
                .suffixes
                .get(self.tree.declarators.get(id).suffixes)
                .first()
                .copied()
        });
        let (skip, bound) = if let Some(ast::Suffix::Array(Some(bound))) = first {
            (1, Some(bound))
        } else {
            (0, None)
        };
        let base = self.specifiers(syntax.specifiers, scope, false)?.ty;
        let environment = self.declarator_environment(syntax.declarator, scope)?;
        let ty = self
            .declarator_view(syntax.declarator, base, environment, skip)?
            .ty;
        Ok((ty, bound))
    }
    fn function_type(
        &mut self,
        ty: TypeId,
        clause: ast::ParameterClause,
        qualifiers: crate::syntax::storage::List<ast::Qualifier>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<TypeId, Error> {
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array { .. } | TypeKind::Function { .. }
        ) {
            return Err(Error::at(
                location,
                "function returning an array or function",
            ));
        }
        // Earlier parameter objects are needed only when a later parameter's
        // type may name a value. Ordinary prototypes allocate no semantic scope.
        let mut prototype = None;
        let mut types = Vec::new();
        let parameters = self.tree.parameters.get(clause.parameters);
        for (index, parameter) in parameters.iter().enumerate() {
            if let Some(name) = self.declarator_name(parameter.declarator)? {
                self.check_template_parameter_name(scope, name, parameter.location)?;
            }
            if prototype.is_none() && index != 0 && self.parameter_uses_values(*parameter) {
                let created =
                    self.graph
                        .add_scope(super::graph::ScopeKind::Function, scope, None, None)?;
                for (previous, &ty) in parameters[..index].iter().zip(&types) {
                    self.declare_parameter_object(*previous, ty, created)?;
                }
                prototype = Some(created);
            }
            let lookup = prototype.unwrap_or(scope);
            let specs = self.specifiers(parameter.specifiers, lookup, false)?;
            let info = self.declarator(parameter.declarator, specs.ty, lookup)?;
            types.push(info.ty);
            if let Some(prototype) = prototype {
                self.declare_parameter_object(*parameter, info.ty, prototype)?;
            }
        }
        if types.len() == 1
            && !clause.ellipsis
            && self
                .declarator_name(self.tree.parameters.get(clause.parameters)[0].declarator)?
                .is_none()
            && matches!(
                self.graph.types.get(types[0]).kind,
                TypeKind::Scalar(Scalar::Void)
            )
            && self.graph.types.get(types[0]).cv.0 == 0
        {
            types.clear();
        }
        if types.iter().any(|&ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        }) {
            return Err(Error::at(location, "parameter of type void"));
        }
        let mut cv = Cv::default();
        let mut reference = None;
        for qualifier in self.tree.qualifiers.get(qualifiers) {
            match qualifier.kind {
                ast::QualifierKind::Cv(key) => cv.0 |= Self::cv(&[key]).0,
                ast::QualifierKind::Ref(op) => {
                    reference = Some(if op.token.as_punctuator() == Some(Punctuator::BitAnd) {
                        RefKind::Lvalue
                    } else {
                        RefKind::Rvalue
                    });
                }
                ast::QualifierKind::Trailing(_) => {
                    return Err(Error::at(location, "a trailing return type needs auto"));
                }
                ast::QualifierKind::Noexcept(_) | ast::QualifierKind::Throw(_) => (),
                ast::QualifierKind::Virt(_) => {
                    return Err(Error::Missing("virtual override checks"));
                }
            }
        }
        let ty = self.graph.types.intern(
            TypeKind::Function {
                result: ty,
                parameters: types,
                variadic: clause.ellipsis,
                cv,
                reference,
            },
            Cv::default(),
        )?;
        if self.lowering() {
            // Preserve source parameter cv on bindings; publish the canonical
            // boundary type once for ABI and indirect-call consumers.
            self.graph.types.canonical_function(ty)?;
        }
        Ok(ty)
    }
    fn parameter_uses_values(&self, parameter: ast::Parameter) -> bool {
        let mut declarator = parameter.declarator;
        while let Some(id) = declarator {
            let node = self.tree.declarators.get(id);
            if self.tree.suffixes.get(node.suffixes).iter().any(|suffix| {
                matches!(suffix, ast::Suffix::Array(Some(bound)) if !matches!(self.tree.expressions.get(*bound).kind, ast::ExprKind::Literal(_)))
            }) { return true; }
            declarator = node.nested;
        }
        self.tree
            .specs
            .get(parameter.specifiers)
            .iter()
            .any(|specifier| match specifier.kind {
                SpecifierKind::Decltype(_) => true,
                SpecifierKind::Named(name) => self
                    .tree
                    .name_lists
                    .get(self.tree.names.get(name).components)
                    .iter()
                    .any(|component| {
                        component.arguments.is_some()
                            || matches!(component.kind, ast::NameKind::Decltype(_))
                    }),
                _ => false,
            })
    }
    pub(super) fn declare_parameter_object(
        &mut self,
        parameter: ast::Parameter,
        ty: TypeId,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if let Some(name) = self.declarator_name(parameter.declarator)? {
            let adjusted = self.graph.types.parameter_object(ty)?;
            let entity = self
                .graph
                .add_entity(Some(adjusted), Some(name), Keyword::Int)?;
            self.graph.add_binding(
                scope,
                None,
                super::graph::Binding {
                    kind: super::graph::BindingKind::Parameter,
                    name: Some(name),
                    entity,
                    ty: Some(adjusted),
                    key: Keyword::Int,
                },
            )?;
        }
        Ok(())
    }
    pub(super) fn cv(keys: &[Keyword]) -> Cv {
        Cv(keys.iter().fold(0, |bits, key| {
            bits | if *key == Keyword::Const { 1 } else { 2 }
        }))
    }
    pub(super) fn specifier_type(
        &mut self,
        spec: ast::Specifier,
        scope: ScopeId,
        lone: bool,
    ) -> Result<TypeId, Error> {
        match spec.kind {
            SpecifierKind::Named(name) => {
                let found = self.lookup(name, scope, super::lookup::Filter::Ordinary)?;
                let binding = self.graph.binding(found);
                if !matches!(
                    binding.kind,
                    super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                ) {
                    return Err(Error::at(spec.location, "name does not denote a type"));
                }
                let entity = self.graph.entity(binding.entity);
                if binding.kind == super::graph::BindingKind::Type
                    && entity.template_head.is_some()
                    && !entity.scope.is_some_and(|home| {
                        entity.name.is_some_and(|name| {
                            self.graph
                                .own(home, name, super::lookup::Filter::Elaborated)
                                == Some(found)
                        })
                    })
                {
                    return Err(Error::at(
                        self.tree.names.get(name).location,
                        "a class template name without a template argument list",
                    ));
                }
                if binding.kind == super::graph::BindingKind::Alias
                    && entity.template_head.is_some()
                {
                    return Err(Error::at(
                        self.tree.names.get(name).location,
                        "an alias template name without a template argument list",
                    ));
                }
                let ty = binding
                    .ty
                    .ok_or(Error::Invariant("type binding lacks a type"))?;
                if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::TemplateParameter { template: true, .. }
                ) {
                    return Err(Error::at(
                        self.tree.names.get(name).location,
                        "a template-name without a template argument list",
                    ));
                }
                Ok(ty)
            }
            SpecifierKind::Class(id) => self.class_type(id, scope),
            SpecifierKind::Elaborated { key, name } => self.elaborated_type(key, name, scope, lone),
            SpecifierKind::Enum(id) => self.enum_type(id, scope, lone),
            SpecifierKind::Decltype(expr) => self.decltype_type(expr, scope),
            SpecifierKind::Vector { .. } => Err(Error::Missing("vector types")),
            SpecifierKind::Keyword(_) => {
                Err(Error::Invariant("keyword reached named-type dispatcher"))
            }
        }
    }
}
