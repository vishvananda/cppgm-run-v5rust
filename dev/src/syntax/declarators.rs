// Translated from syntax/parser_declarators.cpp. See NOTICE.
use super::ast::{InitializerKind, QualifierKind};
use super::{
    ParseError,
    ast::{
        Declarator, DeclaratorId, Initializer, Parameter, ParameterClause, Pointer, Qualifier,
        Suffix,
    },
    cursor::TokenKind,
    parser::{DeclaratorContext, Environment, Parser},
    scopes::Category,
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    Named,
    Parameter,
    Abstract,
    New,
    Conversion,
}
impl Parser<'_> {
    pub(super) fn declarator(&mut self, mode: Mode) -> Result<DeclaratorId, ParseError> {
        self.nested(|parser| parser.declarator_inner(mode))
    }
    fn declarator_inner(&mut self, mode: Mode) -> Result<DeclaratorId, ParseError> {
        let location = self.location()?;
        let start = self.tree.pointers.begin();
        while self.pointer_operator_ahead(0)? {
            let class_name = if matches!(
                self.punctuator()?,
                Some(P::Multiply | P::BitAnd | P::LogicalAnd | P::Xor)
            ) {
                None
            } else {
                let name = self.type_name()?.0;
                self.expect(P::Scope)?;
                self.expect(P::Multiply)?;
                Some(name)
            };
            let op = if class_name.is_some() {
                Self::make_operator(P::Scope)?
            } else {
                self.operator()?
            };
            let cv_start = self.tree.cvs.begin();
            if class_name.is_some()
                || matches!(op.token.as_punctuator(), Some(P::Multiply | P::Xor))
            {
                loop {
                    self.skip_attributes()?;
                    if matches!(self.token()?.kind, TokenKind::Identifier(name) if matches!(self.cursor.name(name), b"_Nonnull" | b"_Nullable" | b"_Null_unspecified" | b"_Nullable_result"))
                    {
                        self.cursor.advance()?;
                        continue;
                    }
                    if !matches!(self.keyword()?, Some(K::Const | K::Volatile)) {
                        break;
                    }
                    if let Some(word) = self.keyword()? {
                        self.tree.cvs.push(word);
                        self.cursor.advance()?;
                    }
                }
            }
            self.skip_attributes()?;
            let cv = self.tree.cvs.finish(cv_start)?;
            self.tree.pointers.push(Pointer { op, class_name, cv });
        }
        let pointers = self.tree.pointers.finish(start)?;
        let pack = !matches!(mode, Mode::New | Mode::Conversion)
            && self.at(P::Ellipsis)?
            && (mode != Mode::Parameter
                || !self.at_ahead(P::RightParen, 1)?
                || self.pack_mentions > self.parameter_pack_start);
        if pack {
            self.cursor.advance()?;
        }
        let mut name = None;
        let mut nested = None;
        if !matches!(mode, Mode::New | Mode::Conversion)
            && self.at(P::LeftParen)?
            && self.nested_declarator_ahead(mode)?
        {
            self.cursor.advance()?;
            nested = Some(self.declarator(mode)?);
            self.expect(P::RightParen)?;
        } else if matches!(mode, Mode::Named | Mode::Parameter)
            && (matches!(self.token()?.kind, TokenKind::Identifier(_))
                || self.at(P::Scope)?
                || self.at(P::Complement)?
                || self.keyword()? == Some(K::Operator))
        {
            name = Some(self.declarator_id()?.0);
            // Declarator-id attributes are collected below.
        } else if mode == Mode::Named
            || (mode == Mode::Parameter
                && self.at(P::LeftParen)?
                && (self.at_ahead(P::RightParen, 1)? || self.at_ahead(P::Ellipsis, 1)?))
        {
            return self.fail();
        }
        let attribute_start = self.tree.attributes.begin();
        if name.is_some() {
            let attributes = self.collect_attributes(false)?;
            self.tree.attributes.extend(attributes);
        }
        let declared_name = name.or_else(|| nested.and_then(|id| self.declarator_name(id)));
        let member = declared_name.and_then(|name| {
            let node = self.tree.names.get(name);
            node.prefix_scope.filter(|&scope| {
                node.prefix_state == super::ast::PrefixState::Known
                    && !(self.declarator_context == DeclaratorContext::Friend
                        && mode == Mode::Named
                        && self.scopes.kind(scope) == super::scopes::ScopeKind::Namespace)
            })
        });
        let names_directly = name.is_some() || nested.is_some_and(|id| self.names_directly(id));
        let suffixes = self.declarator_suffixes(mode, name, names_directly, member)?;
        let attributes = self.tree.attributes.finish(attribute_start)?;
        self.tree.declarators.add(Declarator {
            member_scope: member,
            attributes,
            location,
            abstract_form: matches!(mode, Mode::Abstract | Mode::New | Mode::Conversion),
            pointers,
            pack,
            name,
            nested,
            suffixes,
        })
    }
    // Both ordinary declarations and special members use the same suffix
    // grammar; the latter makes every parameter clause definite.
    pub(super) fn declarator_suffixes(
        &mut self,
        mode: Mode,
        name: Option<super::ast::Name>,
        names_directly: bool,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<List<Suffix>, ParseError> {
        let mut bound_scope = None;
        let suffix_start = self.tree.suffixes.begin();
        loop {
            if mode == Mode::Conversion {
                break;
            }
            if self.at(P::LeftBracket)? && !self.at_ahead(P::LeftBracket, 1)? {
                self.cursor.advance()?;
                let bound = if self.at(P::RightBracket)? {
                    None
                } else {
                    let enclosing = self.scopes.current;
                    if let Some(member) = member {
                        let scope = self.bound_lookup_scope(member, &mut bound_scope)?;
                        self.scopes.current = Some(scope);
                    }
                    let result = self.without_angle(Self::expression);
                    if member.is_some() {
                        self.scopes.current = enclosing;
                    }
                    Some(result?)
                };
                self.expect(P::RightBracket)?;
                let attributes = self.collect_attributes(true)?;
                self.tree.attributes.extend(attributes);
                self.tree.suffixes.push(Suffix::Array(bound));
                continue;
            }
            if matches!(mode, Mode::New | Mode::Conversion) || !self.at(P::LeftParen)? {
                break;
            }
            let definite = mode != Mode::Named
                || self.parameter_context == super::parser::ParameterContext::SpecialMember
                || name.is_some_and(|name| {
                    self.tree
                        .name_lists
                        .get(self.tree.names.get(name).components)
                        .last()
                        .is_some_and(|component| {
                            !matches!(component.kind, super::ast::NameKind::Identifier(_))
                        })
                });
            let first_suffix = self.tree.suffixes.begin() == suffix_start;
            let lookup_member = member.filter(|_| first_suffix);
            let result =
                self.attempt(|parser| parser.parameter_clause_member(definite, lookup_member));
            let Some(parameters) = result? else {
                break;
            };
            let parameter_scope = self
                .parameter_scope
                .ok_or(ParseError::Invariant("missing parameter scope"))?;
            if names_directly && first_suffix {
                self.function_scope = Some(parameter_scope);
            }
            let saved = self.scopes.current;
            if lookup_member.is_some() {
                self.scopes.current = Some(parameter_scope);
            }
            let result = self.function_qualifiers_with_attributes(mode == Mode::Named);
            if lookup_member.is_some() {
                self.scopes.current = saved;
            }
            let qualifiers = result?;
            self.tree.suffixes.push(Suffix::Function {
                parameters,
                qualifiers,
            });
        }
        let suffixes = self.tree.suffixes.finish(suffix_start)?;
        if mode == Mode::Named {
            self.declarator_asm_label()?;
        }
        Ok(suffixes)
    }
    pub(super) fn declarator_asm_label(&mut self) -> Result<(), ParseError> {
        if self.keyword()? == Some(K::Asm) {
            let location = self.location()?;
            self.cursor.advance()?;
            self.expect(P::LeftParen)?;
            let expression = self.attribute_string()?;
            self.expect(P::RightParen)?;
            self.tree.attributes.push(super::ast::Attribute {
                kind: super::ast::AttributeKind::AsmLabel,
                location,
                expression: Some(expression),
                ty: None,
                form: super::ast::AttributeForm::Gnu,
                site: super::ast::AttributeSite::Entity,
                continued: false,
            });
            let attributes = self.collect_attributes(false)?;
            self.tree.attributes.extend(attributes);
        }
        Ok(())
    }
    fn bound_lookup_scope(
        &mut self,
        member: super::scopes::ScopeId,
        cached: &mut Option<super::scopes::ScopeId>,
    ) -> Result<super::scopes::ScopeId, ParseError> {
        if let Some(scope) = *cached {
            return Ok(scope);
        }
        let scope = self.scopes.member_context(member)?;
        *cached = Some(scope);
        Ok(scope)
    }
    fn pointer_operator_ahead(&mut self, ahead: usize) -> Result<bool, ParseError> {
        let kind = self.cursor.peek(ahead)?.kind;
        let block = self.environment == Environment::Hosted && self.at_ahead(P::Xor, ahead)?;
        if block
            || matches!(kind, TokenKind::Simple(op) if matches!(op.token.as_punctuator(), Some(P::Multiply | P::BitAnd | P::LogicalAnd)))
        {
            return Ok(true);
        }
        if matches!(kind, TokenKind::Identifier(_))
            || matches!(kind, TokenKind::Simple(op) if op.token.as_punctuator() == Some(P::Scope))
        {
            return self.member_pointer_ahead(ahead);
        }
        Ok(false)
    }
    fn member_pointer_ahead(&mut self, mut ahead: usize) -> Result<bool, ParseError> {
        if self.at_ahead(P::Scope, ahead)? {
            Self::advance_pointer_lookahead(&mut ahead)?;
        }
        loop {
            if matches!(self.cursor.peek(ahead)?.kind, TokenKind::Simple(op) if op.token.as_keyword() == Some(K::Template))
            {
                Self::advance_pointer_lookahead(&mut ahead)?;
            }
            if !matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(_)) {
                return Ok(false);
            }
            Self::advance_pointer_lookahead(&mut ahead)?;
            if self.at_ahead(P::Less, ahead)? {
                let mut depth = 0i64;
                loop {
                    let token = self.cursor.peek(ahead)?;
                    let punct = match token.kind {
                        TokenKind::End => return Ok(false),
                        TokenKind::Simple(op) => op.token.as_punctuator(),
                        _ => None,
                    };
                    match punct {
                        Some(P::Semicolon | P::LeftBrace) => return Ok(false),
                        Some(P::Less | P::LeftParen | P::LeftBracket) => depth += 1,
                        Some(P::Greater | P::RightParen | P::RightBracket) => depth -= 1,
                        _ => (),
                    }
                    Self::advance_pointer_lookahead(&mut ahead)?;
                    if depth == 0 {
                        break;
                    }
                }
            }
            if !self.at_ahead(P::Scope, ahead)? {
                return Ok(false);
            }
            Self::advance_pointer_lookahead(&mut ahead)?;
            if self.at_ahead(P::Multiply, ahead)? {
                return Ok(true);
            }
        }
    }
    fn advance_pointer_lookahead(ahead: &mut usize) -> Result<(), ParseError> {
        *ahead = ahead
            .checked_add(1)
            .ok_or(ParseError::Limit("member pointer lookahead limit"))?;
        Ok(())
    }
    fn names_directly(&self, id: DeclaratorId) -> bool {
        let node = self.tree.declarators.get(id);
        node.name.is_some()
            || node.nested.is_some_and(|nested| {
                let child = self.tree.declarators.get(nested);
                child.pointers.is_empty()
                    && child.suffixes.is_empty()
                    && self.names_directly(nested)
            })
    }
    fn nested_declarator_ahead(&mut self, mode: Mode) -> Result<bool, ParseError> {
        if mode == Mode::Named
            || self.pointer_operator_ahead(1)?
            || self.at_ahead(P::LeftParen, 1)?
        {
            return Ok(true);
        }
        if self.at_ahead(P::LeftBracket, 1)? && !self.at_ahead(P::LeftBracket, 2)? {
            return Ok(true);
        }
        if mode == Mode::Abstract {
            return Ok(false);
        }
        if let TokenKind::Identifier(name) = self.cursor.peek(1)?.kind {
            if self.at_ahead(P::Scope, 2)? {
                return Ok(false);
            }
            let binding = self.scopes.lookup(name);
            return Ok(binding.map_or_else(
                || {
                    !self
                        .cursor
                        .name(name)
                        .iter()
                        .any(|byte| matches!(byte, b'C' | b'Y' | b'E' | b'T'))
                },
                |binding| !matches!(binding.category, Category::Type | Category::TypeTemplate),
            ));
        }
        Ok(false)
    }
    pub(super) fn declarator_empty(&self, id: DeclaratorId) -> bool {
        let node = self.tree.declarators.get(id);
        node.pointers.is_empty()
            && !node.pack
            && node.name.is_none()
            && node.nested.is_none()
            && node.suffixes.is_empty()
    }
    pub(super) fn is_function(&self, id: DeclaratorId) -> bool {
        self.declarator_name(id).is_some() && self.first_derivation(id) == Some(true)
    }
    fn first_derivation(&self, id: DeclaratorId) -> Option<bool> {
        let node = self.tree.declarators.get(id);
        if let Some(nested) = node.nested
            && let Some(derived) = self.first_derivation(nested)
        {
            return Some(derived);
        }
        if let Some(suffix) = self.tree.suffixes.get(node.suffixes).first() {
            return Some(matches!(suffix, Suffix::Function { .. }));
        }
        (!node.pointers.is_empty()).then_some(false)
    }
    pub(super) fn parameter_clause_member(
        &mut self,
        definite: bool,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<ParameterClause, ParseError> {
        self.nested(|parser| {
            parser.without_angle(|parser| {
                let pending_attributes = parser.pending_attributes.take();
                let leading_attributes = parser.leading_attributes;
                parser.leading_attributes = List::default();
                let pending = parser.pending_template;
                parser.pending_template = false;
                let result = parser.parameter_clause_inner(definite, member);
                parser.pending_template = pending;
                parser.pending_attributes = pending_attributes;
                parser.leading_attributes = leading_attributes;
                result
            })
        })
    }
    fn parameter_clause_inner(
        &mut self,
        definite: bool,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<ParameterClause, ParseError> {
        let location = self.location()?;
        self.expect(P::LeftParen)?;
        let enclosing = self.scopes.current;
        let scope = self.scopes.push()?;
        if let Some(member) = member {
            self.scopes.set_member(scope, member);
        }
        let result = self.parameter_clause_tail(definite, location);
        self.scopes.current = enclosing;
        let clause = result?;
        self.parameter_scope = Some(scope);
        Ok(clause)
    }
    fn parameter_clause_tail(
        &mut self,
        mut definite: bool,
        location: crate::support::source::SourceOffset,
    ) -> Result<ParameterClause, ParseError> {
        let start = self.tree.parameters.begin();
        let mut ellipsis = false;
        let mut ellipsis_without_comma = false;
        while !self.at(P::RightParen)? {
            if self.accept(P::Ellipsis)? {
                ellipsis = true;
                break;
            }
            let parameter_location = self.location()?;
            let enclosing_pack_start = self.parameter_pack_start;
            self.parameter_pack_start = self.pack_mentions;
            let specifiers = self.specifier_sequence(definite, false)?;
            definite = true;
            let declarator = if let Some(declarator) =
                self.attempt(|parser| parser.declarator(Mode::Parameter))?
            {
                declarator
            } else {
                self.declarator(Mode::Abstract)?
            };
            let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
            if declarator.is_some_and(|declarator| self.tree.declarators.get(declarator).pack) {
                self.pack_mentions = self.parameter_pack_start;
            }
            self.parameter_pack_start = enclosing_pack_start;
            self.skip_attributes()?;
            let default = if self.accept(P::Assign)? {
                Some(Initializer {
                    location: self.location()?,
                    kind: InitializerKind::Assign(self.initializer_clause()?),
                })
            } else {
                None
            };
            if let Some(declarator) = declarator {
                self.declare_value_declarator(declarator, false, false)?;
            }
            self.tree.parameters.push(Parameter {
                dependent_sole: false,
                location: parameter_location,
                specifiers,
                declarator,
                default,
            });
            if self.accept(P::Comma)? {
                continue;
            }
            if self.accept(P::Ellipsis)? {
                ellipsis = true;
                ellipsis_without_comma = true;
            }
            break;
        }
        self.expect(P::RightParen)?;
        let parameters = self.tree.parameters.finish(start)?;
        Ok(ParameterClause {
            location,
            parameters,
            ellipsis,
            ellipsis_without_comma,
        })
    }
    pub(super) fn function_qualifiers_with_attributes(
        &mut self,
        keep: bool,
    ) -> Result<super::storage::List<Qualifier>, ParseError> {
        let start = self.tree.qualifiers.begin();
        loop {
            if keep {
                let attributes = self.collect_attributes(false)?;
                for index in 0..self.tree.attributes.get(attributes).len() {
                    let mut attribute = self.tree.attributes.get(attributes)[index];
                    if attribute.form == super::ast::AttributeForm::Standard {
                        attribute.site = super::ast::AttributeSite::Type;
                    }
                    self.tree.attributes.push(attribute);
                }
            } else {
                self.skip_attributes()?;
            }
            let location = self.location()?;
            let kind = match self.keyword()? {
                Some(word @ (K::Const | K::Volatile)) => {
                    self.cursor.advance()?;
                    QualifierKind::Cv(word)
                }
                Some(K::Noexcept) => {
                    self.cursor.advance()?;
                    let operand = if self.accept(P::LeftParen)? {
                        let operand = self.without_angle(Self::expression)?;
                        self.expect(P::RightParen)?;
                        Some(operand)
                    } else {
                        None
                    };
                    QualifierKind::Noexcept(operand)
                }
                Some(K::Throw) => {
                    self.cursor.advance()?;
                    QualifierKind::Throw(self.throw_types()?)
                }
                _ if matches!(self.punctuator()?, Some(P::BitAnd | P::LogicalAnd)) => {
                    // The frozen qualifier stores the token kind only; unlike
                    // pointer operators, alternative spellings are not retained.
                    let mut op = self.operator()?;
                    op.alternative = false;
                    QualifierKind::Ref(op)
                }
                _ if self.accept(P::Arrow)? => QualifierKind::Trailing(self.type_id(true)?),
                _ if matches!(self.token()?.kind, TokenKind::Identifier(name) if matches!(self.cursor.name(name), b"final" | b"override")) => {
                    QualifierKind::Virt(self.identifier()?)
                }
                _ => break,
            };
            self.tree.qualifiers.push(Qualifier { location, kind });
        }
        self.tree.qualifiers.finish(start)
    }
    pub(super) fn throw_types(
        &mut self,
    ) -> Result<super::storage::List<super::ast::TypeId>, ParseError> {
        self.expect(P::LeftParen)?;
        let start = self.tree.type_lists.begin();
        if !self.at(P::RightParen)? {
            loop {
                let mentions = self.pack_mentions;
                let ty = self.type_id(true)?;
                if self
                    .tree
                    .types
                    .get(ty)
                    .declarator
                    .is_some_and(|declarator| self.tree.declarators.get(declarator).pack)
                {
                    self.pack_mentions = mentions;
                }
                self.tree.type_lists.push(ty);
                if !self.accept(P::Comma)? {
                    break;
                }
            }
        }
        self.expect(P::RightParen)?;
        self.tree.type_lists.finish(start)
    }
}
