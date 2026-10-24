// Translated from analyzer_bodies.cpp::known_argument_type/check_unbound_callee. See NOTICE.
//! Definition-time call-name validation, separate from expression typing and demand.
use super::{
    Error,
    analyzer::Analyzer,
    function_names::ValueKey,
    graph::{BindingKind, ScopeId, ScopeKind},
    lookup::{Filter, LookupResult},
    types::{Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, pp_lexer::Punctuator as P, vocabulary::Keyword},
    syntax::ast::{self, ExprKind, NameKind},
    types::FundamentalType,
};

impl Analyzer<'_> {
    pub(super) fn form_template_arguments(
        &mut self,
        component: ast::NameComponent,
        scope: ScopeId,
        head: Option<super::templates::HeadId>,
    ) -> Result<Vec<TypeId>, Error> {
        let syntax = component
            .arguments
            .ok_or(Error::Invariant("template-id without arguments"))?;
        let mut arguments = Vec::new();
        for index in 0..self.tree.template_arguments.get(syntax).len() {
            let argument = self.tree.template_arguments.get(syntax)[index];
            if self.integer_pack_argument(argument, scope, &mut arguments)? {
                continue;
            }
            let index = arguments.len();
            let position = if let Some(head) = head {
                let count = self.graph.templates.parameters(head).len();
                if index < count {
                    Some(index)
                } else {
                    count
                        .checked_sub(1)
                        .filter(|&last| self.argument_parameter_pack(head, last))
                }
            } else {
                None
            };
            let target = if let (Some(head), Some(position)) = (head, position) {
                if self.argument_parameter_pack(head, position) {
                    let parameter = self.graph.templates.parameters(head)[position];
                    self.graph
                        .entity(parameter)
                        .template_parameter
                        .and_then(|info| info.value_type)
                        .filter(|&ty| !self.graph.dependent_type(ty))
                } else {
                    self.argument_target(head, position, &arguments)?.known()
                }
            } else {
                None
            };
            let template = head
                .and_then(|head| self.template_argument_parameter(head, index))
                .map(|info| info.inner.is_some());
            let unknown = target.is_none()
                && head
                    .zip(position)
                    .and_then(|(head, index)| self.template_argument_parameter(head, index))
                    .is_none_or(|info| info.value_type.is_some());
            let mut ty = self.template_argument_at(
                argument,
                scope,
                component.location,
                target,
                template,
                unknown,
            )?;
            if argument.pack {
                ty = match self.graph.types.get(ty).kind {
                    TypeKind::BoundPack { arguments, .. } => arguments,
                    TypeKind::Pack(_) => ty,
                    _ => self.pack_expansion_type(ty, component.location)?,
                };
            }
            let expanded = argument.pack
                || matches!(argument.kind,
                ast::TemplateArgumentKind::Type(ty) if self.tree.types.get(ty).declarator
                    .is_some_and(|d| self.tree.declarators.get(d).pack));
            if expanded && matches!(self.graph.types.get(ty).kind, TypeKind::Expansion(_)) {
                arguments.extend(self.bound_argument_expansion(ty)?);
            } else if expanded && let TypeKind::Pack(elements) = &self.graph.types.get(ty).kind {
                arguments.extend(elements.iter().copied());
            } else {
                arguments.push(self.graph.types.canonical_function(ty)?);
            }
        }
        Ok(arguments)
    }

    /// C++ `class_templates.cpp::template_argument`: a parsed id-expression
    /// can denote a type even when parser lookup could not classify it.
    /// Keep the dependent-qualifier rule separate from that concrete lookup.
    pub(super) fn template_argument_identity(
        &mut self,
        argument: ast::TemplateArgument,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<Option<TypeId>, Error> {
        let ty = match argument.kind {
            ast::TemplateArgumentKind::Type(ty) => self.type_syntax(ty, scope)?,
            ast::TemplateArgumentKind::Expression(expression) => {
                let ExprKind::Id(name) = self.tree.expressions.get(expression).kind else {
                    return Ok(None);
                };
                let found = self.lookup(name, scope, Filter::Ordinary)?;
                let binding = self.graph.binding(found);
                if binding.kind == BindingKind::TemplateValue
                    && self
                        .graph
                        .entity(binding.entity)
                        .template_parameter
                        .is_some()
                {
                    return self
                        .graph
                        .entity(binding.entity)
                        .ty
                        .map(Some)
                        .ok_or(Error::Invariant("value parameter without placeholder"));
                }
                let node = self.tree.names.get(name);
                let components = self.tree.name_lists.get(node.components);
                let dependent = (node.global || components.len() > 1)
                    && components.last().is_some_and(|last| !last.template_keyword)
                    && self.graph.dependent_members.get(binding.entity).is_some();
                if dependent || !matches!(binding.kind, BindingKind::Type | BindingKind::Alias) {
                    return Ok(None);
                }
                binding
                    .ty
                    .ok_or(Error::Invariant("template type argument without type"))?
            }
        };
        if matches!(self.graph.types.get(ty).kind, TypeKind::Auto) {
            return Err(Error::at(location, "auto as a template argument"));
        }
        Ok(Some(ty))
    }

    pub(super) fn template_argument_names(
        &mut self,
        arguments: crate::syntax::storage::List<ast::TemplateArgument>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<(), Error> {
        let mut expanded = Vec::new();
        for syntax_index in 0..self.tree.template_arguments.get(arguments).len() {
            let syntax_item = self.tree.template_arguments.get(arguments)[syntax_index];
            if self.integer_pack_argument(syntax_item, scope, &mut expanded)? {
                continue;
            }
            let argument = &syntax_item;
            let ty = self.template_argument_at(*argument, scope, location, None, None, true)?;
            if argument.pack {
                self.pack_expansion_type(ty, location)?;
            }
        }
        Ok(())
    }
    fn template_local_type(&self, mut ty: TypeId) -> bool {
        while let TypeKind::Pointer(element)
        | TypeKind::Reference { element, .. }
        | TypeKind::Array { element, .. } = self.graph.types.get(ty).kind
        {
            ty = element;
        }
        let entity = match self.graph.types.get(ty).kind {
            TypeKind::Class(entity) | TypeKind::Enum(entity) => self.graph.entity(entity),
            _ => return false,
        };
        let mut current = entity.enumeration.map(|e| e.owner).or(entity.home);
        while let Some(scope) = current {
            let node = self.graph.scope(scope);
            match node.kind {
                ScopeKind::Namespace => return false,
                ScopeKind::Function | ScopeKind::Block => return true,
                _ => current = node.parent,
            }
        }
        false
    }

    fn known_argument_type(
        &mut self,
        mut id: ast::ExprId,
        scope: ScopeId,
        depth: u8,
    ) -> Result<Option<TypeId>, Error> {
        // This is C++'s bounded conservative type probe, not a stack limit.
        if depth > 32 {
            return Ok(None);
        }
        while let ExprKind::Paren(inner) = self.tree.expressions.get(id).kind {
            id = inner;
        }
        let fundamental = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))?;
        let next = depth + 1;
        let ty = match self.tree.expressions.get(id).kind {
            ExprKind::Literal(literal) => {
                return Ok(matches!(
                    literal.kind,
                    PostTokenKind::Literal { .. }
                        | PostTokenKind::Character {
                            suffix_begin: None,
                            ..
                        }
                        | PostTokenKind::String { suffix: None, .. }
                )
                .then_some(fundamental));
            }
            ExprKind::Keyword(keyword) => {
                return Ok((keyword != Keyword::This).then_some(fundamental));
            }
            ExprKind::Sizeof { .. } | ExprKind::SizeofPack(_) => return Ok(Some(fundamental)),
            ExprKind::TypeTrait { keyword, .. } => {
                return Ok((keyword != Keyword::Typeid).then_some(fundamental));
            }
            ExprKind::Unary { op, operand } => {
                if matches!(op.token.as_punctuator(), Some(P::BitAnd | P::Multiply)) {
                    return Ok(None);
                }
                return Ok(self
                    .known_argument_type(operand, scope, next)?
                    .filter(|&ty| self.is_arithmetic_type(ty))
                    .map(|_| fundamental));
            }
            ExprKind::Binary {
                left,
                right,
                assignment: false,
                ..
            } => {
                if !self
                    .known_argument_type(left, scope, next)?
                    .is_some_and(|ty| self.is_arithmetic_type(ty))
                {
                    return Ok(None);
                }
                return Ok(self
                    .known_argument_type(right, scope, next)?
                    .filter(|&ty| self.is_arithmetic_type(ty))
                    .map(|_| fundamental));
            }
            ExprKind::FunctionalCast { specifiers, .. } => {
                self.specifiers(specifiers, scope, false)?.ty
            }
            kind @ (ExprKind::Call { .. } | ExprKind::Id(_)) => {
                let (name, cast) = match kind {
                    ExprKind::Id(name) => (name, false),
                    ExprKind::Call { callee, .. } => {
                        let ExprKind::Id(name) = self.tree.expressions.get(callee).kind else {
                            return Ok(None);
                        };
                        (name, true)
                    }
                    _ => return Err(Error::Invariant("non-name argument probe")),
                };
                let syntax_item = *self.tree.names.get(name);
                let node = &syntax_item;
                let components = self.tree.name_lists.get(node.components);
                if node.global || components.len() != 1 || components[0].arguments.is_some() {
                    return Ok(None);
                }
                let NameKind::Identifier(identifier) = components[0].kind else {
                    return Ok(None);
                };
                let found = match self.unqualified_result(
                    scope,
                    ValueKey::Identifier(identifier),
                    Filter::Ordinary,
                ) {
                    Ok(found) => found,
                    Err(Error::Diagnostic {
                        message: "ambiguous name",
                        ..
                    }) => return Ok(None),
                    Err(error) => return Err(error),
                };
                let Some(binding) = found.binding.filter(|_| !found.ambiguous) else {
                    return Ok(None);
                };
                let binding = self.graph.binding(binding);
                let ty = match (cast, binding.kind) {
                    (true, BindingKind::Type | BindingKind::Alias)
                    | (false, BindingKind::Parameter) => binding.ty,
                    (false, BindingKind::Variable | BindingKind::Enumerator) => {
                        self.graph.entity(binding.entity).ty
                    }
                    _ => return Ok(None),
                };
                let Some(ty) = ty else { return Ok(None) };
                if self.exception_dependent_type(ty) || self.template_local_type(ty) {
                    return Ok(None);
                }
                return Ok(Some(
                    if !cast
                        && let TypeKind::Reference { element, .. } = self.graph.types.get(ty).kind
                    {
                        element
                    } else {
                        ty
                    },
                ));
            }
            _ => return Ok(None),
        };
        Ok((!self.exception_dependent_type(ty) && !self.template_local_type(ty)).then_some(ty))
    }

    pub(super) fn check_unbound_callee(
        &mut self,
        name: ast::Name,
        arguments: &[ast::ExprId],
        scope: ScopeId,
    ) -> Result<(), Error> {
        let syntax_item = *self.tree.names.get(name);
        let node = &syntax_item;
        let components = self.tree.name_lists.get(node.components);
        if node.global || components.len() != 1 || arguments.len() > 8 {
            return Ok(());
        }
        let NameKind::Identifier(identifier) = components[0].kind else {
            return Ok(());
        };
        let spelling = self.identifiers.bytes(identifier);
        if spelling.first() == Some(&b' ')
            || spelling.first() == Some(&b'_')
                && spelling
                    .get(1)
                    .is_some_and(|&b| b == b'_' || b.is_ascii_uppercase())
        {
            return Ok(());
        }
        let key = ValueKey::Identifier(identifier);
        if self
            .unqualified_result(scope, key, Filter::Ordinary)?
            .binding
            .is_some()
        {
            return Ok(());
        }
        let fundamental = self
            .graph
            .types
            .scalar(Scalar::Basic(FundamentalType::Int))?;
        let mut types = [fundamental; 8];
        let mut associates = false;
        for (&argument, slot) in arguments.iter().zip(&mut types) {
            let Some(ty) = self.known_argument_type(argument, scope, 0)? else {
                return Ok(());
            };
            *slot = ty;
            associates |= !self.is_arithmetic_type(ty);
        }
        if associates {
            let mut found = LookupResult::default();
            self.associated_type_functions(key, &types[..arguments.len()], &mut found)?;
            if !found.functions.as_slice().is_empty() {
                return Ok(());
            }
        }
        Err(Error::at(node.location, "undeclared name"))
    }
}
