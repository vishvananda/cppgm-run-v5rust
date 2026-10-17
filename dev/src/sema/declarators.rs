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
pub(super) enum Placement {
    Declaration,
    ExplicitInstantiation,
}
#[derive(Clone, Copy)]
pub(super) struct DeclaratorEnvironment {
    pub placement: Placement,
    pub friendship: super::specifiers::Friendship,
    pub auto_operators: bool,
    pub placeholder_function: Option<ast::DeclaratorId>,
    pub home: Option<ScopeId>,
    pub lookup: ScopeId,
    pub declaring: ScopeId,
    pub print: ScopeId,
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
        self.type_syntax_with_auto(id, scope, false)
    }
    fn type_syntax_with_auto(
        &mut self,
        id: ast::TypeId,
        scope: ScopeId,
        auto_operators: bool,
    ) -> Result<TypeId, Error> {
        if (self.evaluating_noexcept || self.semantic())
            && let Some(ty) = self.graph.type_fact(id)
        {
            return Ok(ty);
        }
        let node = *self.tree.types.get(id);
        let specs = self.specifiers(node.specifiers, scope, false)?;
        let mut environment = self.declarator_environment(node.declarator, scope)?;
        environment.auto_operators = auto_operators;
        let ty = self
            .declarator_in(node.declarator, specs.ty, environment)?
            .ty;
        if matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
            return Err(Error::at(node.location, "auto in a type-id"));
        }
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
                    self.member_lookup_bridge(scope, home)?
                } else {
                    scope
                };
                break;
            }
            probe = node.nested;
        }
        Ok(DeclaratorEnvironment {
            placement: Placement::Declaration,
            friendship: super::specifiers::Friendship::Ordinary,
            auto_operators: false,
            placeholder_function: None,
            home,
            lookup: after,
            declaring: self.declaration_scope(scope),
            print: scope,
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
        let mut prototype = None;
        let mut specified = false;
        let mut derivations = 0_u8;
        let DeclaratorEnvironment {
            placement,
            friendship,
            home,
            lookup: after,
            declaring,
            print,
            declaration_location,
            auto_operators,
            placeholder_function,
        } = environment;
        if friendship != super::specifiers::Friendship::Friend
            && matches!(placement, Placement::Declaration)
            && home.is_some_and(|home| !self.graph.encloses(declaring, home))
        {
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
            if node.name.is_some() {
                name = self.declarator_name(Some(current))?;
                syntax_name = node.name;
            }
            for syntax_index in 0..self.tree.pointers.get(node.pointers).len() {
                let syntax_item = self.tree.pointers.get(node.pointers)[syntax_index];
                let pointer = &syntax_item;
                parameters = None;
                function_qualifiers = None;
                if !auto_operators && matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
                    return Err(Error::at(
                        node.location,
                        "auto with a declarator operator is not supported here",
                    ));
                }
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
                    Self::exception_derivation(specified, &mut derivations, node.location)?;
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
                Self::exception_derivation(specified, &mut derivations, node.location)?;
            }
            let skip = if Some(current) == root {
                root_suffix_skip
            } else {
                0
            };
            for index in (skip..self.tree.suffixes.get(node.suffixes).len()).rev() {
                let syntax_item = self.tree.suffixes.get(node.suffixes)[index];
                let suffix = &syntax_item;
                match *suffix {
                    ast::Suffix::Array(bound) => {
                        parameters = None;
                        function_qualifiers = None;
                        ty = self.array_suffix(ty, bound, after, node.location)?;
                    }
                    ast::Suffix::Function {
                        parameters: clause,
                        qualifiers,
                    } => {
                        let formed = self.function_type(
                            ty,
                            clause,
                            qualifiers,
                            after,
                            node.location,
                            placeholder_function == Some(current) && index == 0,
                        )?;
                        ty = formed.0;
                        prototype = formed.1.map(|scope| (clause.location, scope));
                        parameters = Some(clause);
                        function_qualifiers = Some(qualifiers);
                    }
                }
                if specified && matches!(suffix, ast::Suffix::Array(_)) {
                    return Err(Error::at(
                        node.location,
                        "an exception-specification on a function type nested in a declarator",
                    ));
                }
                specified = function_qualifiers.is_some_and(|list| {
                    self.tree.qualifiers.get(list).iter().any(|qualifier| {
                        matches!(
                            qualifier.kind,
                            ast::QualifierKind::Throw(_) | ast::QualifierKind::Noexcept(_)
                        )
                    })
                });
                derivations = 0;
            }
            id = node.nested;
        }
        // C++ forms the complete pattern before checking the root ellipsis.
        // In particular, invalid pointer/suffix types precede pack diagnostics.
        if let Some(root) = root {
            let node = self.tree.declarators.get(root);
            if node.pack {
                ty = self.pack_expansion_type(ty, node.location)?;
            }
        }
        if let Some((clause, prototype)) = prototype
            && parameters.is_some_and(|parameters| parameters.location == clause)
            && let Some(name) = name
        {
            self.graph.scope_mut(prototype).name = Some(name);
            self.graph.scope_mut(print).children.push(prototype);
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

    fn array_suffix(
        &mut self,
        ty: TypeId,
        bound: Option<ast::ExprId>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<TypeId, Error> {
        if self.holds_auto(ty) {
            return Err(Error::at(location, "array of auto"));
        }
        let failure = match self.graph.types.get(ty).kind {
            TypeKind::Reference { .. } => Some("array of references"),
            TypeKind::Function { .. } => Some("array of functions"),
            TypeKind::Scalar(Scalar::Void) => Some("array of void"),
            TypeKind::Array {
                bound: 0,
                operand: None,
                ..
            } => Some("array of arrays of unknown bound"),
            _ => None,
        };
        if let Some(message) = failure {
            return Err(Error::at(location, message));
        }
        self.check_abstract_object(ty, location)?;
        let count = bound
            .map(|bound| self.array_bound(bound, scope))
            .transpose()?
            .unwrap_or(0);
        let operand = if count == 0 && self.template_depth != 0 {
            bound
                .map(|expression| self.dependent_operand(expression, scope, false))
                .transpose()?
        } else {
            None
        };
        self.graph.types.intern(
            TypeKind::Array {
                element: ty,
                bound: count,
                operand,
            },
            Cv::default(),
        )
    }

    fn exception_derivation(
        specified: bool,
        derivations: &mut u8,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        if specified {
            *derivations += 1;
            if *derivations > 1 {
                return Err(Error::at(
                    location,
                    "an exception-specification on a function type nested in a declarator",
                ));
            }
        }
        Ok(())
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
    pub(super) fn allocation_bound(
        &self,
        id: ast::TypeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<ast::ExprId>, Error> {
        let syntax = self.tree.types.get(id);
        let suffix = syntax.declarator.and_then(|id| {
            self.tree
                .suffixes
                .get(self.tree.declarators.get(id).suffixes)
                .first()
        });
        match suffix {
            Some(ast::Suffix::Array(None)) => Err(Error::at(
                location,
                "an array new-expression without a bound",
            )),
            Some(ast::Suffix::Array(Some(bound))) => Ok(Some(*bound)),
            _ => Ok(None),
        }
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
        mut ty: TypeId,
        clause: ast::ParameterClause,
        qualifiers: crate::syntax::storage::List<ast::Qualifier>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
        placeholder_result: bool,
    ) -> Result<(TypeId, Option<ScopeId>), Error> {
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
        let mut placed = false;
        let mut types = Vec::new();
        let parameter_count = self.tree.parameters.get(clause.parameters).len();
        for index in 0..parameter_count {
            let syntax_item = self.tree.parameters.get(clause.parameters)[index];
            let parameter = &syntax_item;
            if let Some(name) = self.declarator_name(parameter.declarator)? {
                self.check_template_parameter_name(scope, name, parameter.location)?;
            }
            if prototype.is_none() && index != 0 && self.parameter_uses_values(*parameter) {
                let created =
                    self.graph
                        .add_scope(super::graph::ScopeKind::Function, scope, None, None)?;
                self.graph.scope_mut(created).guarded = true;
                for (previous_index, &ty) in types.iter().enumerate() {
                    let previous = self.tree.parameters.get(clause.parameters)[previous_index];
                    self.prototype_parameter(previous, ty, created, false, previous_index)?;
                }
                prototype = Some(created);
            } else if let Some(earlier) = prototype {
                // A placeholder retains the scope before its own parameter.
                // Publish that parameter in a child frame so substitution
                // cannot see itself or any later parameter.
                let current = if placed {
                    self.graph
                        .add_scope(super::graph::ScopeKind::Function, earlier, None, None)?
                } else {
                    earlier
                };
                let previous = self.tree.parameters.get(clause.parameters)[index - 1];
                self.graph.scope_mut(current).guarded = true;
                self.prototype_parameter(previous, types[index - 1], current, false, index - 1)?;
                prototype = Some(current);
            }
            let made = self.graph.template_operands.count();
            let lookup = prototype.unwrap_or(scope);
            let specs = self.specifiers(parameter.specifiers, lookup, false)?;
            if matches!(self.graph.types.get(specs.ty).kind, TypeKind::Auto) {
                return Err(Error::at(parameter.location, "parameter without a type"));
            }
            let info = self.declarator(parameter.declarator, specs.ty, lookup)?;
            types.push(info.ty);
            placed = self.graph.template_operands.count() != made;
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
        let mut trailing_prototype = None;
        for syntax_index in 0..self.tree.qualifiers.get(qualifiers).len() {
            let syntax_item = self.tree.qualifiers.get(qualifiers)[syntax_index];
            let qualifier = &syntax_item;
            match qualifier.kind {
                ast::QualifierKind::Cv(key) => cv.0 |= Self::cv(&[key]).0,
                ast::QualifierKind::Ref(op) => {
                    reference = Some(if op.token.as_punctuator() == Some(Punctuator::BitAnd) {
                        RefKind::Lvalue
                    } else {
                        RefKind::Rvalue
                    });
                }
                ast::QualifierKind::Trailing(result) => {
                    if !matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
                        return Err(Error::at(location, "a trailing return type needs auto"));
                    }
                    let prototype = self.graph.add_scope(
                        super::graph::ScopeKind::Function,
                        scope,
                        None,
                        None,
                    )?;
                    for (parameter_index, &ty) in types.iter().enumerate() {
                        let parameter =
                            self.tree.parameters.get(clause.parameters)[parameter_index];
                        self.prototype_parameter(parameter, ty, prototype, true, parameter_index)?;
                    }
                    ty = self.type_syntax_with_auto(result, prototype, placeholder_result)?;
                    trailing_prototype = Some(prototype);
                }
                ast::QualifierKind::Noexcept(_)
                | ast::QualifierKind::Throw(_)
                | ast::QualifierKind::Virt(_) => (),
            }
        }
        if self.holds_auto(ty) {
            return Err(if placeholder_result {
                Error::Missing("deduced function return types")
            } else {
                Error::at(location, "auto return type without a trailing return type")
            });
        }
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array { .. } | TypeKind::Function { .. }
        ) {
            return Err(Error::at(
                location,
                "function returning a function or an array",
            ));
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
        Ok((ty, trailing_prototype))
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
    fn prototype_parameter(
        &mut self,
        parameter: ast::Parameter,
        ty: TypeId,
        scope: ScopeId,
        printed: bool,
        position: usize,
    ) -> Result<(), Error> {
        let name = self.declarator_name(parameter.declarator)?;
        if printed || name.is_some() {
            let adjusted = if printed {
                ty
            } else {
                self.graph.types.parameter_object(ty)?
            };
            let entity = self.graph.add_entity(Some(adjusted), name, Keyword::Int)?;
            self.graph.record_prototype_parameter(entity, position);
            self.graph.add_binding(
                scope,
                // Prototype parameters must be enumerable for substitution.
                // Earlier-parameter scopes themselves are not attached to the
                // printed scope tree; their bindings still belong to them.
                Some(scope),
                super::graph::Binding {
                    kind: super::graph::BindingKind::Parameter,
                    name,
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
                let node = self.tree.names.get(name);
                if node.typename_keyword
                    && !node.global
                    && self.tree.name_lists.get(node.components).len() == 1
                {
                    return Err(Error::at(
                        node.location,
                        "typename applied to an unqualified name",
                    ));
                }
                let result = self.lookup_result(name, scope, super::lookup::Filter::Ordinary)?;
                let found = result.binding.ok_or_else(|| {
                    Error::at(self.tree.names.get(name).location, "undeclared name")
                })?;
                let binding = self.graph.binding(found);
                if !matches!(
                    binding.kind,
                    super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                ) {
                    return Err(Error::at(spec.location, "name does not denote a type"));
                }
                let entity = self.graph.entity(binding.entity);
                let node = self.tree.names.get(name);
                if !node.typename_keyword
                    && (node.global || self.tree.name_lists.get(node.components).len() > 1)
                    && self.graph.dependent_members.get(binding.entity).is_some()
                {
                    return Err(Error::at(
                        node.location,
                        "a dependent qualified name needs typename to name a type",
                    ));
                }
                if !node.typename_keyword
                    && binding.kind == super::graph::BindingKind::Type
                    && result.qualifier.is_some_and(|qualifier| {
                        self.graph.binding_scope(found) == qualifier
                            && self.graph.scope(qualifier).owner == Some(binding.entity)
                    })
                    && self
                        .tree
                        .name_lists
                        .get(node.components)
                        .last()
                        .is_some_and(|last| last.arguments.is_none())
                {
                    return Err(Error::at(
                        node.location,
                        "a class's qualified injected-class-name names its constructor, not a type",
                    ));
                }
                if binding.kind == super::graph::BindingKind::Type
                    && entity.template_head.is_some()
                    && self
                        .tree
                        .name_lists
                        .get(node.components)
                        .last()
                        .is_some_and(|last| last.arguments.is_none())
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
