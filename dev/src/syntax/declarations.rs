// Translated from syntax/parser_declarations.cpp and parser_specifiers.cpp. See NOTICE.
use super::ast::{InitializerKind, SpecifierKind};
use super::{
    ParseError,
    ast::{
        DeclId, DeclKind, Declaration, Enumerator, InitDeclarator, Initializer, Specifier, TypeId,
        TypeSyntax,
    },
    cursor::TokenKind,
    parser::Parser,
    scopes::{Binding, Category},
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
use crate::support::source::SourceOffset;
impl Parser<'_> {
    pub(super) fn declaration(&mut self) -> Result<DeclId, ParseError> {
        self.declaration_with_attributes(List::default())
    }
    pub(super) fn declaration_with_attributes(
        &mut self,
        leading: List<super::ast::Attribute>,
    ) -> Result<DeclId, ParseError> {
        self.nested(|parser| {
            let location = parser.location()?;
            let pending = parser.pending_attributes;
            let saved_leading = parser.leading_attributes;
            let saved_tags = parser.pending_class_tags;
            parser.pending_class_tags = List::default();
            let start = parser.tree.attributes.begin();
            parser.pending_attributes = Some(start);
            let result = (|| {
                let first = parser.tree.attributes.begin();
                parser.tree.attributes.extend(leading);
                let attributes = parser.collect_attributes(false)?;
                parser.tree.attributes.extend(attributes);
                parser.leading_attributes = parser.tree.attributes.finish(first)?;
                parser.tree.attributes.extend(parser.leading_attributes);
                let declaration = parser.declaration_inner()?;
                let attributes = parser.tree.attributes.finish(start)?;
                let attributes = if parser.pending_class_tags.is_empty() {
                    attributes
                } else {
                    parser.tree.attributes.extend(parser.pending_class_tags);
                    parser.tree.attributes.extend(attributes);
                    parser.tree.attributes.finish(start)?
                };
                let mut node = *parser.tree.declarations.get(declaration);
                if matches!(node.kind, DeclKind::Empty) {
                    node.location = location;
                    parser.tree.declarations.replace(declaration, node);
                }
                if matches!(
                    node.kind,
                    DeclKind::Simple { .. }
                        | DeclKind::Function { .. }
                        | DeclKind::SpecialMember { .. }
                        | DeclKind::BitField { .. }
                ) {
                    node.attributes = attributes;
                    parser.tree.declarations.replace(declaration, node);
                }
                Ok(declaration)
            })();
            parser.pending_attributes = pending;
            parser.leading_attributes = saved_leading;
            parser.pending_class_tags = saved_tags;
            result
        })
    }
    fn declaration_inner(&mut self) -> Result<DeclId, ParseError> {
        let location = self.location()?;
        if self.accept(P::Semicolon)? {
            return self.tree.declarations.add(Declaration {
                attributes: super::storage::List::default(),
                location,
                kind: DeclKind::Empty,
            });
        }
        if self.in_class()
            && matches!(self.keyword()?, Some(K::Public | K::Protected | K::Private))
            && self.at_ahead(P::Colon, 1)?
        {
            let access = self
                .keyword()?
                .ok_or(ParseError::Invariant("missing access keyword"))?;
            self.cursor.advance()?;
            self.cursor.advance()?;
            return self.tree.declarations.add(Declaration {
                attributes: super::storage::List::default(),
                location,
                kind: DeclKind::Access(access),
            });
        }
        match self.keyword()? {
            Some(K::Namespace) => self.namespace(location, false),
            Some(K::Inline) if matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op) if op.token.as_keyword() == Some(K::Namespace)) =>
            {
                self.cursor.advance()?;
                self.namespace(location, true)
            }
            Some(K::Using) => self.using_declaration(location),
            Some(K::StaticAssert) => self.static_assert(location),
            Some(K::Extern) if matches!(self.cursor.peek(1)?.kind, TokenKind::Literal(literal) if matches!(literal.kind, crate::lex::PostTokenKind::String { .. })) => {
                self.linkage(location)
            }
            Some(K::Template) if self.at_ahead(P::Less, 1)? => self.template_declaration(location),
            Some(K::Template) => self.explicit_instantiation(location),
            Some(K::Extern) if matches!(self.cursor.peek(1)?.kind, TokenKind::Simple(op) if op.token.as_keyword() == Some(K::Template)) => {
                self.explicit_instantiation(location)
            }
            _ => {
                if matches!(
                    self.scopes.kind(self.scopes.declaration_scope()),
                    super::scopes::ScopeKind::Namespace
                        | super::scopes::ScopeKind::Class(_)
                        | super::scopes::ScopeKind::AnonymousClass
                ) && let Some((specifiers, name)) = self.attempt(Self::special_member_head)?
                {
                    return self.special_member(location, specifiers, name);
                }
                self.simple_declaration(location)
            }
        }
    }
    fn namespace(&mut self, location: SourceOffset, inline: bool) -> Result<DeclId, ParseError> {
        self.cursor.advance()?;
        if matches!(self.token()?.kind, TokenKind::Identifier(_)) && self.at_ahead(P::Assign, 1)? {
            return self.namespace_alias(location);
        }
        let attributes = self.collect_attributes(false)?;
        let name = if matches!(self.token()?.kind, TokenKind::Identifier(_)) {
            Some(self.identifier()?)
        } else {
            None
        };
        let trailing = self.collect_attributes(false)?;
        let start = self.tree.attributes.begin();
        self.tree.attributes.extend(attributes);
        self.tree.attributes.extend(trailing);
        let attributes = self.tree.attributes.finish(start)?;
        self.expect(P::LeftBrace)?;
        let enclosing = self.scopes.current;
        let existing = name
            .and_then(|name| self.scopes.own(enclosing, name))
            .filter(|binding| binding.category == Category::Namespace)
            .and_then(|binding| binding.scope);
        let existing = if name.is_none() {
            self.scopes.unnamed(enclosing)
        } else {
            existing
        };
        let scope = if let Some(scope) = existing {
            scope
        } else {
            self.scopes.push_kind(super::scopes::ScopeKind::Namespace)?
        };
        self.scopes.current = enclosing;
        if let Some(name) = name.filter(|_| existing.is_none()) {
            self.scopes.declare(
                name,
                Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: false,
                    category: Category::Namespace,
                    scope: Some(scope),
                },
            );
        }
        if name.is_none() && existing.is_none() {
            self.scopes.set_unnamed(enclosing, scope);
        }
        if name.is_none() || inline {
            self.scopes.add_edge(scope);
        }
        self.scopes.current = scope;
        let declarations = self.declarations(true)?;
        self.expect(P::RightBrace)?;
        self.scopes.current = enclosing;
        self.tree.declarations.add(Declaration {
            attributes,
            location,
            kind: DeclKind::Namespace {
                name,
                inline,
                declarations,
            },
        })
    }
    fn namespace_alias(&mut self, location: SourceOffset) -> Result<DeclId, ParseError> {
        let name = self.identifier()?;
        self.expect(P::Assign)?;
        if self.keyword()? == Some(K::Typename) {
            return self.fail();
        }
        let (target, binding) = self.type_name()?;
        self.expect(P::Semicolon)?;
        self.scopes.declare(
            name,
            Binding {
                pack: false,
                alternatives: super::scopes::Categories::default(),
                guesses: super::scopes::Guesses::default(),
                specialization: 0,
                parameter: false,
                category: Category::Namespace,
                scope: binding.and_then(|binding| binding.scope),
            },
        );
        self.tree.declarations.add(Declaration {
            attributes: List::default(),
            location,
            kind: DeclKind::NamespaceAlias { name, target },
        })
    }
    fn using_declaration(&mut self, location: SourceOffset) -> Result<DeclId, ParseError> {
        self.cursor.advance()?;
        let is_template = self.pending_template;
        self.pending_template = false;
        let directive = self.accept_keyword(K::Namespace)?;
        if !directive
            && matches!(self.token()?.kind, TokenKind::Identifier(_))
            && (self.at_ahead(P::Assign, 1)? || self.at_attribute(1)?)
        {
            let name = self.identifier()?;
            let attributes = self.collect_attributes(false)?;
            self.expect(P::Assign)?;
            let ty = self.type_id(true)?;
            self.expect(P::Semicolon)?;
            if self
                .tree
                .attributes
                .get(attributes)
                .iter()
                .any(|attribute| {
                    matches!(
                        attribute.kind,
                        super::ast::AttributeKind::VectorSize
                            | super::ast::AttributeKind::ExtVectorType
                    )
                })
            {
                let mut type_node = *self.tree.types.get(ty);
                let start = self.tree.specs.begin();
                self.tree.specs.extend(type_node.specifiers);
                self.vector_specifiers(attributes, false)?;
                type_node.specifiers = self.tree.specs.finish(start)?;
                self.tree.types.replace(ty, type_node);
            }
            let type_node = *self.tree.types.get(ty);
            let scope = if type_node.declarator.is_none() {
                self.specifier_scope(type_node.specifiers)
            } else {
                None
            };
            let specialization = if is_template {
                0
            } else {
                type_node.specialization
            };
            let mut binding = Binding {
                pack: false,
                alternatives: super::scopes::Categories::default(),
                guesses: super::scopes::Guesses::default(),
                specialization,
                parameter: false,
                category: if is_template {
                    Category::TypeTemplate
                } else {
                    Category::Type
                },
                scope,
            };
            if is_template && self.tree.specs.get(type_node.specifiers).iter().any(|specifier| {
                matches!(specifier.kind, SpecifierKind::Named(name) if { let node = self.tree.names.get(name); node.prefix_state == super::ast::PrefixState::Known && (node.prefix_template != super::ast::TemplatePrefix::Ordinary) })
            }) { binding.scope = None; }
            self.note_type_guesses(&mut binding, type_node.specifiers, is_template);
            self.scopes
                .declare_in(self.scopes.declaration_scope(), name, binding);
            return self.tree.declarations.add(Declaration {
                attributes,
                location,
                kind: DeclKind::Alias { name, ty },
            });
        }
        let (target, binding) = if directive {
            if self.keyword()? == Some(K::Typename) {
                return self.fail();
            }
            self.type_name()?
        } else {
            self.declarator_id()?
        };
        let attributes = if directive {
            List::default()
        } else {
            self.collect_attributes(false)?
        };
        self.expect(P::Semicolon)?;
        if let Some(binding) = binding {
            if directive {
                if let Some(scope) = binding.scope
                    && binding.category == Category::Namespace
                {
                    self.scopes.add_using_directive(scope);
                }
            } else if let Some(name) = self.last_identifier(target) {
                self.scopes.declare(
                    name,
                    Binding {
                        pack: false,
                        category: binding.category,
                        scope: binding.scope,
                        parameter: false,
                        specialization: 0,
                        alternatives: super::scopes::Categories::default(),
                        guesses: super::scopes::Guesses::default(),
                    },
                );
            }
        } else if !directive
            && self.tree.names.get(target).typename_keyword
            && let Some(name) = self.last_identifier(target)
        {
            self.scopes.declare(
                name,
                Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: false,
                    category: Category::Type,
                    scope: None,
                },
            );
        }
        self.tree.declarations.add(Declaration {
            attributes,
            location,
            kind: DeclKind::Using { directive, target },
        })
    }
    fn static_assert(&mut self, location: SourceOffset) -> Result<DeclId, ParseError> {
        self.cursor.advance()?;
        self.expect(P::LeftParen)?;
        let condition = self.assignment_expression()?;
        let message = if self.accept(P::Comma)? {
            let TokenKind::Literal(literal) = self.token()?.kind else {
                return self.fail();
            };
            if !matches!(literal.kind, crate::lex::PostTokenKind::String { .. }) {
                return self.fail();
            }
            self.cursor.advance()?;
            Some(literal)
        } else {
            None
        };
        self.expect(P::RightParen)?;
        self.expect(P::Semicolon)?;
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::StaticAssert { condition, message },
        })
    }
    fn linkage(&mut self, location: SourceOffset) -> Result<DeclId, ParseError> {
        self.cursor.advance()?;
        let TokenKind::Literal(literal) = self.token()?.kind else {
            return self.fail();
        };
        let language = super::storage::TextStore::unquote(literal.spelling);
        self.cursor.advance()?;
        let braced = self.accept(P::LeftBrace)?;
        let declarations = if braced {
            let declarations = self.declarations(true)?;
            self.expect(P::RightBrace)?;
            declarations
        } else {
            let start = self.tree.decl_lists.begin();
            let declaration = self.declaration()?;
            self.tree.decl_lists.push(declaration);
            self.tree.decl_lists.finish(start)?
        };
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::Linkage {
                language,
                braced,
                declarations,
            },
        })
    }
    pub(super) fn specifier_sequence(
        &mut self,
        definite: bool,
        type_only: bool,
    ) -> Result<List<Specifier>, ParseError> {
        self.specifier_sequence_with_leading(definite, type_only, List::default(), None)
    }
    fn specifier_sequence_with_leading(
        &mut self,
        definite: bool,
        type_only: bool,
        leading: List<super::ast::Attribute>,
        initial: Option<(super::ast::Name, SourceOffset)>,
    ) -> Result<List<Specifier>, ParseError> {
        let start = self.tree.specs.begin();
        self.vector_specifiers(leading, false)?;
        if let Some((name, location)) = initial {
            self.tree.specs.push(self.named_specifier(name, location));
        }
        let mut has_type = initial.is_some();
        let mut sign_only = false;
        let mut complex_only = false;
        let mut has_friend = false;
        loop {
            let attributes = self.collect_attributes(self.tree.specs.begin() != start)?;
            self.vector_specifiers(attributes, !type_only)?;
            let location = self.location()?;
            match self.keyword()? {
                Some(word)
                    if Self::is_specifier(word)
                        && (!type_only
                            || Self::has_type_keyword(word)
                            || matches!(word, K::Const | K::Volatile)) =>
                {
                    if Self::has_type_keyword(word) {
                        sign_only =
                            (!has_type || sign_only) && matches!(word, K::Signed | K::Unsigned);
                        complex_only = false;
                    }
                    has_type |= Self::has_type_keyword(word);
                    has_friend |= word == K::Friend;
                    self.cursor.advance()?;
                    self.tree.specs.push(Specifier {
                        location,
                        kind: SpecifierKind::Keyword(word),
                    });
                }
                Some(K::Enum) if !has_type => {
                    let enumeration = self.enum_specifier()?;
                    self.tree.specs.push(Specifier {
                        location,
                        kind: SpecifierKind::Enum(enumeration),
                    });
                    has_type = true;
                }
                Some(K::Class | K::Struct | K::Union) if !has_type => {
                    let specifier =
                        self.class_specifier(has_friend, self.tree.specs.begin() == start)?;
                    self.tree.specs.push(specifier);
                    has_type = true;
                }
                _ if (!has_type
                    || matches!(self.token()?.kind, TokenKind::Identifier(name)
                    if self.complex_types.contains(&Some(name))
                        || (sign_only && self.extended_types[0] == Some(name))
                        || (complex_only && self.extended_types.contains(&Some(name)))))
                    && (matches!(self.token()?.kind, TokenKind::Identifier(_))
                        || self.at(P::Scope)?
                        || matches!(self.keyword()?, Some(K::Typename | K::Decltype))) =>
                {
                    let complex = !has_type
                        && matches!(self.token()?.kind, TokenKind::Identifier(name) if self.complex_types.contains(&Some(name)));
                    let named = self.attempt(|parser| {
                        let (name, binding) = parser.type_name()?;
                        let followed = matches!(parser.token()?.kind, TokenKind::Identifier(_));
                        if !parser.named_declaration_is_type(name, binding, definite, followed) {
                            // A category mismatch abandons this reading without
                            // recording a grammar failure in the frozen parser.
                            return Err(ParseError::Syntax {
                                location: parser.location()?,
                                message: "syntax error",
                            });
                        }
                        Ok(name)
                    })?;
                    let Some(name) = named else {
                        break;
                    };
                    self.tree.specs.push(self.named_specifier(name, location));
                    has_type = true;
                    sign_only = false;
                    complex_only = complex;
                }
                _ => break,
            }
        }
        if self.tree.specs.begin() == start || (type_only && !has_type) {
            return self.fail();
        }
        self.tree.specs.finish(start)
    }
    pub(super) fn type_id(&mut self, definite: bool) -> Result<TypeId, ParseError> {
        self.type_id_in_mode(definite, super::declarators::Mode::Abstract)
    }
    fn named_specifier(&self, name: super::ast::Name, location: SourceOffset) -> Specifier {
        let node = self.tree.names.get(name);
        let kind = if !node.global
            && self.tree.name_lists.get(node.components).len() == 1
            && let super::ast::NameKind::Decltype(operand) =
                self.tree.name_lists.get(node.components)[0].kind
        {
            SpecifierKind::Decltype(operand)
        } else {
            SpecifierKind::Named(name)
        };
        Specifier { location, kind }
    }
    pub(super) fn type_from_name(
        &mut self,
        name: super::ast::Name,
        location: SourceOffset,
    ) -> Result<TypeId, ParseError> {
        let specifiers = self.specifier_sequence_with_leading(
            false,
            true,
            List::default(),
            Some((name, location)),
        )?;
        let declarator = self.declarator(super::declarators::Mode::Abstract)?;
        let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
        // The frozen shared-name builder does not set SpecifierInfo::type_name.
        // It consequently records no alias specialization for this reading.
        self.tree.types.add(TypeSyntax {
            specialization: 0,
            location,
            specifiers,
            declarator,
        })
    }
    pub(super) fn type_id_in_mode(
        &mut self,
        definite: bool,
        mode: super::declarators::Mode,
    ) -> Result<TypeId, ParseError> {
        let location = self.location()?;
        let specifiers = self.specifier_sequence(definite, true)?;
        let declarator = self.declarator(mode)?;
        let declarator = (!self.declarator_empty(declarator)).then_some(declarator);
        let specialization = if declarator.is_none() {
            self.typedef_record(specifiers)
        } else {
            0
        };
        self.tree.types.add(TypeSyntax {
            specialization,
            location,
            specifiers,
            declarator,
        })
    }
    pub(super) fn specifiers_have_type(&self, specifiers: List<Specifier>) -> bool {
        self.tree
            .specs
            .get(specifiers)
            .iter()
            .any(|specifier| match &specifier.kind {
                SpecifierKind::Keyword(word) => Self::has_type_keyword(*word),
                SpecifierKind::Vector { .. } => false,
                SpecifierKind::Named(_)
                | SpecifierKind::Enum(_)
                | SpecifierKind::Class(_)
                | SpecifierKind::Elaborated { .. }
                | SpecifierKind::Decltype(_) => true,
            })
    }
    fn enum_specifier(&mut self) -> Result<DeclId, ParseError> {
        let location = self.location()?;
        self.cursor.advance()?;
        let key = if matches!(self.keyword()?, Some(K::Class | K::Struct)) {
            let key = self.keyword()?;
            self.cursor.advance()?;
            key
        } else {
            None
        };
        let attributes = self.collect_attributes(false)?;
        let start = self.tree.attributes.begin();
        for index in 0..self.tree.attributes.get(attributes).len() {
            let attribute = self.tree.attributes.get(attributes)[index];
            if attribute.kind == super::ast::AttributeKind::AbiTag
                || (attribute.kind == super::ast::AttributeKind::NoReturn
                    && attribute.form == super::ast::AttributeForm::Standard)
            {
                self.tree.attributes.push(attribute);
            }
        }
        let attributes = self.tree.attributes.finish(start)?;
        let name = if matches!(self.token()?.kind, TokenKind::Identifier(_)) || self.at(P::Scope)? {
            Some(self.declared_type_name()?.0)
        } else {
            None
        };
        let underlying = if self.accept(P::Colon)? {
            let location = self.location()?;
            let specifiers = self.specifier_sequence(true, true)?;
            Some(self.tree.types.add(TypeSyntax {
                specialization: 0,
                location,
                specifiers,
                declarator: None,
            })?)
        } else {
            None
        };
        let has_body = self.accept(P::LeftBrace)?;
        if !has_body && name.is_none() {
            return self.fail();
        }
        let enclosing = self.scopes.current;
        let members = self.scopes.declaration_scope();
        let scope = if has_body {
            let scope = self.scopes.push_kind(super::scopes::ScopeKind::Enum)?;
            self.scopes.current = enclosing;
            Some(scope)
        } else {
            None
        };
        if let Some(simple) = name
            .filter(|&name| !self.name_has_arguments(name))
            .and_then(|name| self.simple_name(name))
            && (has_body
                || !self
                    .scopes
                    .lookup_name(simple, true)
                    .is_some_and(|binding| binding.category == Category::Type))
        {
            self.scopes.declare_in(
                members,
                simple,
                Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    parameter: false,
                    category: Category::Type,
                    scope,
                },
            );
        }
        let start = self.tree.enumerators.begin();
        let mut trailing_comma = false;
        if has_body {
            while !self.at(P::RightBrace)? {
                let location = self.location()?;
                let name = self.identifier()?;
                self.skip_attributes()?;
                let value = if self.accept(P::Assign)? {
                    Some(self.assignment_expression()?)
                } else {
                    None
                };
                self.tree.enumerators.push(Enumerator {
                    location,
                    name,
                    value,
                });
                if let Some(scope) = scope {
                    self.scopes.declare_in(
                        scope,
                        name,
                        Binding {
                            pack: false,
                            alternatives: super::scopes::Categories::default(),
                            guesses: super::scopes::Guesses::default(),
                            specialization: 0,
                            parameter: false,
                            category: Category::Value,
                            scope: None,
                        },
                    );
                }
                if key.is_none() {
                    self.scopes.declare_in(
                        members,
                        name,
                        Binding {
                            pack: false,
                            alternatives: super::scopes::Categories::default(),
                            guesses: super::scopes::Guesses::default(),
                            specialization: 0,
                            parameter: false,
                            category: Category::Value,
                            scope: None,
                        },
                    );
                }
                if !self.accept(P::Comma)? {
                    break;
                }
                trailing_comma = self.at(P::RightBrace)?;
            }
            self.expect(P::RightBrace)?;
        }
        let enumerators = self.tree.enumerators.finish(start)?;
        self.tree.declarations.add(Declaration {
            attributes,
            location,
            kind: DeclKind::Enum {
                scope,
                name,
                key,
                underlying,
                enumerators,
                has_body,
                trailing_comma,
            },
        })
    }
    fn simple_declaration(&mut self, location: SourceOffset) -> Result<DeclId, ParseError> {
        let specifiers =
            self.specifier_sequence_with_leading(false, false, self.leading_attributes, None)?;
        let typedef = self
            .tree
            .specs
            .get(specifiers)
            .iter()
            .any(|specifier| matches!(specifier.kind, SpecifierKind::Keyword(K::Typedef)));
        if self.accept(P::Semicolon)? {
            if let [specifier] = self.tree.specs.get(specifiers) {
                match specifier.kind {
                    SpecifierKind::Enum(enumeration) if !self.in_class() => return Ok(enumeration),
                    SpecifierKind::Class(_) | SpecifierKind::Elaborated { .. }
                        if !self.pending_class_attributes() =>
                    {
                        return self.tree.declarations.add(Declaration {
                            attributes: super::storage::List::default(),
                            location,
                            kind: DeclKind::Class(*specifier),
                        });
                    }
                    _ => (),
                }
            }
            return self.tree.declarations.add(Declaration {
                attributes: super::storage::List::default(),
                location,
                kind: DeclKind::Simple {
                    specifiers,
                    declarators: List::default(),
                },
            });
        }
        if self.in_class() && self.at(P::Colon)? {
            return self.bit_field(location, specifiers, None);
        }
        self.function_scope = None;
        let start = self.tree.init_declarators.begin();
        loop {
            let first_token = self.token_ordinal()?;
            let declarator = self.declarator(super::declarators::Mode::Named)?;
            self.declare_declarator(
                declarator,
                typedef,
                self.specifier_scope(specifiers),
                specifiers,
            );
            if self.is_function(declarator)
                && (self.at(P::LeftBrace)? || self.keyword()? == Some(K::Try))
            {
                self.pending_template = false;
                let body = self.finish_function_body(self.in_class(), false)?;
                return self.tree.declarations.add(Declaration {
                    attributes: super::storage::List::default(),
                    location,
                    kind: DeclKind::Function {
                        specifiers,
                        declarator,
                        body,
                    },
                });
            }
            if self.in_class() && self.at(P::Colon)? {
                return self.bit_field(location, specifiers, Some(declarator));
            }
            let enclosing = self.scopes.current;
            if let Some(scope) = self
                .declarator_name(declarator)
                .and_then(|name| self.tree.names.get(name).prefix_scope)
            {
                self.scopes.member_context(scope)?;
            }
            let result = if matches!(
                self.punctuator()?,
                Some(P::Assign | P::LeftBrace | P::LeftParen)
            ) {
                self.initializer().map(Some)
            } else {
                Ok(None)
            };
            self.scopes.current = enclosing;
            let initializer = result?;
            self.tree.init_declarators.push(InitDeclarator {
                tokens: super::ast::TokenSpan {
                    begin: first_token,
                    end: self.token_ordinal()?,
                },
                declarator,
                initializer,
            });
            if !self.accept(P::Comma)? {
                break;
            }
        }
        self.pending_template = false;
        self.expect(P::Semicolon)?;
        let declarators = self.tree.init_declarators.finish(start)?;
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::Simple {
                specifiers,
                declarators,
            },
        })
    }
    pub(super) fn initializer(&mut self) -> Result<Initializer, ParseError> {
        let location = self.location()?;
        let kind = self.without_angle(Self::initializer_kind)?;
        Ok(Initializer { location, kind })
    }
    fn initializer_kind(&mut self) -> Result<InitializerKind, ParseError> {
        if self.accept(P::Assign)? {
            if self.at_ahead(P::Semicolon, 1)? || self.at_ahead(P::Comma, 1)? {
                if self.accept_keyword(K::Default)? {
                    return Ok(InitializerKind::Default);
                }
                if self.accept_keyword(K::Delete)? {
                    return Ok(InitializerKind::Delete);
                }
            }
            return Ok(InitializerKind::Assign(self.initializer_clause()?));
        }
        if self.at(P::LeftBrace)? {
            return Ok(InitializerKind::Braced(self.braced_list()?));
        }
        self.expect(P::LeftParen)?;
        Ok(InitializerKind::Parenthesized(
            self.expression_list(P::RightParen)?,
        ))
    }
}
