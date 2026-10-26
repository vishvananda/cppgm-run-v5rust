// Translated from syntax/parser_names.cpp. See NOTICE.
//! Structured name components; operator and conversion spellings are views.
use super::{
    ParseError,
    ast::{ExprId, Name, NameComponent, NameKind, PrefixState, QualifiedName},
    cursor::TokenKind,
    name_lookup::NamePrefix,
    parser::Parser,
    scopes::{Binding, Category},
};
use crate::lex::{identifiers::NameId, pp_lexer::Punctuator as P, vocabulary::Keyword as K};

#[derive(Clone, Copy)]
enum NameMode {
    Expression,
    Type,
    Member,
    Declarator,
    DeclaredType,
}

impl Parser<'_> {
    pub(super) fn type_specifier_start(&mut self, ahead: usize) -> Result<bool, ParseError> {
        let kind = self.cursor.peek(ahead)?.kind;
        Ok(match kind {
            TokenKind::Identifier(_) => true,
            TokenKind::Simple(op) => {
                op.token.as_punctuator() == Some(P::Scope)
                    || op.token.as_keyword().is_some_and(|word| {
                        matches!(
                            word,
                            K::Const
                                | K::Volatile
                                | K::Typename
                                | K::Decltype
                                | K::Class
                                | K::Struct
                                | K::Union
                                | K::Enum
                        ) || Self::has_type_keyword(word)
                    })
            }
            _ => false,
        })
    }
    pub(super) fn name_starts_ahead(&mut self, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            (matches!(self.cursor.peek(ahead)?.kind, TokenKind::Identifier(_))
                && self.complex_component_ahead(ahead)?.is_none()
                && self.at_type_predicate(ahead)?.is_none()
                && !self.at_offsetof(ahead)?
                && !self.at_integer_pack(ahead)?
                && !self.at_bit_int(ahead)?
                && !self.at_typeof(ahead)?
                && !self.at_attribute(ahead)?)
                || (self.at_ahead(P::Scope, ahead)?
                    && matches!(self.cursor.peek(ahead + 1)?.kind, TokenKind::Identifier(_))),
        )
    }

    pub(super) fn name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        self.name_in_context(NameMode::Expression)
    }
    pub(super) fn type_name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        self.name_in_context(NameMode::Type)
    }
    pub(super) fn member_name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        if self.keyword()? == Some(K::Typename) {
            return self.fail();
        }
        self.name_in_context(NameMode::Member)
    }
    pub(super) fn declarator_id(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        if self.keyword()? == Some(K::Typename) {
            return self.fail();
        }
        self.name_in_context(NameMode::Declarator)
    }
    pub(super) fn declared_type_name(&mut self) -> Result<(Name, Option<Binding>), ParseError> {
        if self.keyword()? == Some(K::Typename) {
            return self.fail();
        }
        self.name_in_context(NameMode::DeclaredType)
    }
    fn name_in_context(&mut self, mode: NameMode) -> Result<(Name, Option<Binding>), ParseError> {
        let type_context = matches!(mode, NameMode::Type | NameMode::DeclaredType);
        let member_context = matches!(mode, NameMode::Member);
        let declared = matches!(mode, NameMode::Declarator | NameMode::DeclaredType);
        let mentions = self.template_mentions;
        let typename_keyword =
            matches!(mode, NameMode::Expression) && self.accept_keyword(K::Typename)?;
        let location = self.location()?;
        let global = self.accept(P::Scope)?;
        let start = self.tree.name_lists.begin();
        let mut prefix = NamePrefix::new(global);
        let mut qualified = global;
        let mut conflict = None;
        let binding = loop {
            let location = self.location()?;
            let template_keyword =
                (qualified || member_context) && self.accept_keyword(K::Template)?;
            // GNU type operators are names only with a following `(`. Keep
            // this bounded dispatch lookahead even when the head is invalid.
            let gnu_decltype = !qualified
                && self.tree.name_lists.begin() == start
                && matches!(mode, NameMode::Expression | NameMode::Type)
                && self.at_ahead(P::LeftParen, 1)?
                && (self.at_type_transform()?.is_some()
                    || self.at_typeof(0)?
                    || self.at_bit_int(0)?);
            if !qualified && (self.keyword()? == Some(K::Decltype) || gnu_decltype) {
                let operand = if self.at_typeof(0)? {
                    self.typeof_operand()?
                } else if self.at_bit_int(0)? {
                    self.bit_int_operand()?
                } else if let Some(transform) = self.at_type_transform()? {
                    self.transform_operand(transform)?
                } else {
                    self.cursor.advance()?;
                    self.decltype_operand()?
                };
                self.tree.name_lists.push(NameComponent {
                    location,
                    kind: NameKind::Decltype(operand),
                    template_keyword,
                    expanded: false,
                    arguments: None,
                });
                if self.scope_continues_name(false)? {
                    self.cursor.advance()?;
                    qualified = true;
                    prefix.make_dependent(false);
                    continue;
                }
                break Some(Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: false,
                    category: Category::Type,
                    scope: None,
                });
            }
            if !type_context && self.keyword()? == Some(K::Operator) {
                let member = prefix.scope.filter(|scope| {
                    matches!(
                        self.scopes.kind(*scope),
                        super::scopes::ScopeKind::Class(_)
                            | super::scopes::ScopeKind::AnonymousClass
                    )
                });
                let kind = self.operator_name(member)?;
                let arguments = if self.at(P::Less)? && !matches!(kind, NameKind::Conversion(_)) {
                    self.attempt(Self::template_arguments)?
                } else {
                    None
                };
                self.tree.name_lists.push(NameComponent {
                    location,
                    kind,
                    template_keyword,
                    expanded: false,
                    arguments,
                });
                break Some(Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: false,
                    category: Category::Value,
                    scope: None,
                });
            }
            if !type_context && self.accept(P::Complement)? {
                let kind = if self.accept_keyword(K::Decltype)? {
                    NameKind::DestructorDecltype(self.decltype_operand()?)
                } else {
                    NameKind::Destructor(self.identifier()?)
                };
                let arguments =
                    if !matches!(kind, NameKind::DestructorDecltype(_)) && self.at(P::Less)? {
                        Some(self.template_arguments()?)
                    } else {
                        None
                    };
                self.tree.name_lists.push(NameComponent {
                    location,
                    kind,
                    template_keyword,
                    expanded: false,
                    arguments,
                });
                break None;
            }
            let identifier = self.identifier()?;
            let uses = self.conflicts.uses;

            let argument_mentions = self.template_mentions;
            let arguments =
                self.arguments_for_name(identifier, &prefix, member_context, template_keyword)?;
            self.tree.name_lists.push(NameComponent {
                location,
                kind: NameKind::Identifier(identifier),
                template_keyword,
                expanded: false,
                arguments,
            });
            if !self.scope_continues_name(true)? {
                let binding = self.resolve_component(identifier, &prefix, false);
                if !member_context {
                    self.observe_name_binding(binding);
                }
                self.note_component_conflict(uses, start, &mut conflict)?;
                break binding;
            }
            self.cursor.advance()?;
            self.advance_name_prefix(
                identifier,
                arguments,
                self.template_mentions != argument_mentions,
                &mut prefix,
                declared,
            );
            self.note_component_conflict(uses, start, &mut conflict)?;
            qualified = true;
        };
        let components = self.tree.name_lists.finish(start)?;
        let name = self.tree.names.add(QualifiedName {
            location,
            global,
            typename_keyword,
            components,
            prefix_scope: prefix.scope,
            prefix_state: prefix.state,
            prefix_template: prefix.template,
            prefix_guesses: super::scopes::Guesses {
                approximate: prefix.approximate,
                dependent: prefix.state == PrefixState::Dependent,
            },
            mentions_parameters: self.template_mentions != mentions,
            binding,
        })?;
        if self.conflicts.statement.is_none()
            && let Some((component, category)) = conflict
        {
            self.conflicts.statement = Some(super::conflicts::StatementConflict {
                name,
                component,
                category,
            });
        }
        Ok((name, binding))
    }
    fn note_component_conflict(
        &self,
        uses: u32,
        start: usize,
        conflict: &mut Option<(u32, Option<Category>)>,
    ) -> Result<(), ParseError> {
        if conflict.is_none() && self.conflicts.uses != uses {
            let component = u32::try_from(self.tree.name_lists.begin() - start - 1)
                .map_err(|_| ParseError::Limit("name component ordinal limit"))?;
            // The frozen signed component field uses negative values as
            // unpublished sentinels, including ordinals with the sign bit set.
            if i32::try_from(component).is_ok() {
                *conflict = Some((component, self.conflicts.last));
            }
        }
        Ok(())
    }
    fn scope_continues_name(&mut self, operator: bool) -> Result<bool, ParseError> {
        if !self.at(P::Scope)? {
            return Ok(false);
        }
        Ok(match self.cursor.peek(1)?.kind {
            TokenKind::Identifier(_) => true,
            TokenKind::Simple(op) => {
                op.token.as_punctuator() == Some(P::Complement)
                    || op.token.as_keyword() == Some(K::Template)
                    || (operator && op.token.as_keyword() == Some(K::Operator))
            }
            _ => false,
        })
    }
    fn operator_name(
        &mut self,
        member: Option<super::scopes::ScopeId>,
    ) -> Result<NameKind, ParseError> {
        self.cursor.advance()?;
        if matches!(self.keyword()?, Some(K::New | K::Delete)) {
            let op = self.operator()?;
            let array = self.at(P::LeftBracket)? && self.at_ahead(P::RightBracket, 1)?;
            if array {
                self.cursor.advance()?;
                self.cursor.advance()?;
            }
            return Ok(NameKind::Operator { op, array });
        }
        if (self.at(P::LeftParen)? && self.at_ahead(P::RightParen, 1)?)
            || (self.at(P::LeftBracket)? && self.at_ahead(P::RightBracket, 1)?)
        {
            let op = self.operator()?;
            self.cursor.advance()?;
            return Ok(NameKind::Operator { op, array: false });
        }
        if self.at(P::Greater)?
            && self.token()?.shift_half == 1
            && self.cursor.peek(1)?.shift_half == 2
        {
            self.cursor.advance()?;
            self.cursor.advance()?;
            return Ok(NameKind::Operator {
                op: Self::make_operator(P::ShiftRight)?,
                array: false,
            });
        }
        // A split literal-operator suffix can be a keyword token. Its
        // adjacency check is part of dispatch, including invalid heads.
        let next = self.cursor.peek(1)?;
        if let TokenKind::Literal(literal) = self.token()?.kind
            && self.cursor.spellings.get(literal.spelling) == b"\"\""
        {
            let suffix = match next.kind {
                TokenKind::Identifier(name) => Some(name),
                TokenKind::Simple(_) if next.location.0 == self.location()?.0.wrapping_add(2) => {
                    next.suffix_name
                }
                _ => None,
            };
            if let Some(suffix) = suffix {
                self.cursor.advance()?;
                self.cursor.advance()?;
                return Ok(NameKind::LiteralOperator(suffix));
            }
        }
        if matches!(
            self.punctuator()?,
            Some(
                P::Add
                    | P::Subtract
                    | P::Multiply
                    | P::Divide
                    | P::Remainder
                    | P::Xor
                    | P::BitAnd
                    | P::BitOr
                    | P::Complement
                    | P::LogicalNot
                    | P::Assign
                    | P::Less
                    | P::Greater
                    | P::AddAssign
                    | P::SubtractAssign
                    | P::MultiplyAssign
                    | P::DivideAssign
                    | P::RemainderAssign
                    | P::XorAssign
                    | P::AndAssign
                    | P::OrAssign
                    | P::ShiftLeft
                    | P::ShiftRightAssign
                    | P::ShiftLeftAssign
                    | P::Equal
                    | P::NotEqual
                    | P::LessEqual
                    | P::GreaterEqual
                    | P::LogicalAnd
                    | P::LogicalOr
                    | P::Increment
                    | P::Decrement
                    | P::Comma
                    | P::ArrowStar
                    | P::Arrow
            )
        ) {
            return Ok(NameKind::Operator {
                op: self.operator()?,
                array: false,
            });
        }
        let enclosing = self.scopes.current;
        let result = (|| {
            if let Some(member) = member {
                self.scopes.member_context(member)?;
            }
            self.type_id_in_mode(true, super::declarators::Mode::Conversion)
        })();
        self.scopes.current = enclosing;
        Ok(NameKind::Conversion(result?))
    }
    pub(super) fn decltype_operand(&mut self) -> Result<ExprId, ParseError> {
        self.expect(P::LeftParen)?;
        let operand = self.without_angle(Self::expression)?;
        self.expect(P::RightParen)?;
        Ok(operand)
    }
    pub(super) fn single_decltype_name(&mut self) -> Result<Name, ParseError> {
        let location = self.location()?;
        let mentions = self.template_mentions;
        self.cursor.advance()?;
        let operand = self.decltype_operand()?;
        let start = self.tree.name_lists.begin();
        self.tree.name_lists.push(NameComponent {
            location,
            kind: NameKind::Decltype(operand),
            template_keyword: false,
            expanded: false,
            arguments: None,
        });
        let components = self.tree.name_lists.finish(start)?;
        self.tree.names.add(QualifiedName {
            location,
            global: false,
            typename_keyword: false,
            components,
            prefix_scope: None,
            prefix_state: PrefixState::Unqualified,
            prefix_template: super::ast::TemplatePrefix::Ordinary,
            prefix_guesses: super::scopes::Guesses::default(),
            mentions_parameters: self.template_mentions != mentions,
            binding: None,
        })
    }
    pub(super) fn is_type_name(
        &self,
        name: Name,
        binding: Option<Binding>,
        definite: bool,
        followed_by_identifier: bool,
    ) -> bool {
        let name = self.tree.names.get(name);
        if name.typename_keyword {
            return true;
        }
        if let Some(binding) = binding {
            return matches!(binding.category, Category::Type | Category::TypeTemplate);
        }
        if name.prefix_state == PrefixState::Dependent && !followed_by_identifier {
            return false;
        }
        let Some(last) = self.tree.name_lists.get(name.components).last() else {
            return false;
        };
        let NameKind::Identifier(identifier) = last.kind else {
            return matches!(last.kind, NameKind::Decltype(_));
        };
        definite
            || followed_by_identifier
            || self
                .cursor
                .name(identifier)
                .iter()
                .any(|byte| matches!(byte, b'C' | b'Y' | b'E' | b'T'))
    }
    pub(super) fn last_identifier(&self, name: Name) -> Option<NameId> {
        match self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()?
            .kind
        {
            NameKind::Identifier(identifier) => Some(identifier),
            _ => None,
        }
    }
    pub(super) fn name_has_arguments(&self, name: Name) -> bool {
        self.tree
            .name_lists
            .get(self.tree.names.get(name).components)
            .last()
            .is_some_and(|component| component.arguments.is_some())
    }
    pub(super) fn simple_name(&self, name: Name) -> Option<NameId> {
        let node = self.tree.names.get(name);
        if !node.global && self.tree.name_lists.get(node.components).len() == 1 {
            self.last_identifier(name)
        } else {
            None
        }
    }
}
