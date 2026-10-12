// Translated from syntax/parser_specifiers.cpp, parser_declarations.cpp and parser.cpp. See NOTICE.
//! Class category ownership and bodies queued on the outermost class completion.
use super::{
    ParseError,
    ast::{
        Base, BitField, Body, BodyId, ClassSyntax, DeclId, DeclKind, Declaration, Declarator,
        DeclaratorId, MemInitializer, Name, NameKind, Specifier, Suffix,
    },
    cursor::{TokenKind, TokenRun},
    declarators::Mode,
    parser::Parser,
    scopes::{Binding, Category, ScopeId, ScopeKind},
    storage::List,
};
use crate::lex::{pp_lexer::Punctuator as P, vocabulary::Keyword as K};
use crate::support::source::SourceOffset;

#[derive(Clone, Copy)]
pub(super) struct DeferredBody {
    target: BodyId,
    tokens: TokenRun,
    scope: ScopeId,
    constructor: bool,
}
impl Parser<'_> {
    pub(super) fn in_class(&self) -> bool {
        matches!(
            self.scopes.kind(self.scopes.declaration_scope()),
            ScopeKind::Class(_) | ScopeKind::AnonymousClass
        )
    }
    pub(super) fn class_specifier(
        &mut self,
        friend: bool,
        first: bool,
    ) -> Result<Specifier, ParseError> {
        let location = self.location()?;
        let key = self
            .keyword()?
            .ok_or(ParseError::Invariant("missing class key"))?;
        self.cursor.advance()?;
        let attributes = self.collect_attributes(false)?;
        let name = if matches!(self.token()?.kind, TokenKind::Identifier(_)) || self.at(P::Scope)? {
            Some(self.type_name()?.0)
        } else {
            None
        };
        let final_specifier = matches!(self.token()?.kind, TokenKind::Identifier(id) if self.cursor.name(id) == b"final")
            && (self.at_ahead(P::LeftBrace, 1)? || self.at_ahead(P::Colon, 1)?);
        if final_specifier {
            self.cursor.advance()?;
        }
        let has_body = self.at(P::LeftBrace)? || self.at(P::Colon)?;
        if !has_body {
            let Some(name) = name else {
                return self.fail();
            };
            let start = self.tree.attributes.begin();
            for index in 0..self.tree.attributes.get(attributes).len() {
                let mut attribute = self.tree.attributes.get(attributes)[index];
                if attribute.kind == super::ast::AttributeKind::AbiTag
                    || (attribute.kind == super::ast::AttributeKind::Alignas
                        && attribute.form != super::ast::AttributeForm::Alignment)
                    || (attribute.kind == super::ast::AttributeKind::NoReturn
                        && attribute.form == super::ast::AttributeForm::Standard)
                {
                    attribute.site = super::ast::AttributeSite::Class;
                    self.tree.attributes.push(attribute);
                }
            }
            let tags = self.tree.attributes.finish(start)?;
            if self.pending_attributes.is_some() {
                self.pending_class_tags = tags;
            }
            return self.elaborated_class(key, name, friend, first);
        }
        let enclosing = self.scopes.current;
        let is_template = self.pending_template;
        self.pending_template = false;
        let identifier = name.and_then(|name| self.last_identifier(name));
        let qualified = name.is_some_and(|name| {
            let name = self.tree.names.get(name);
            name.global || self.tree.name_lists.get(name.components).len() > 1
        });
        let mut home = name
            .and_then(|name| self.tree.names.get(name).prefix_scope)
            .unwrap_or(self.scopes.declaration_scope());
        if let Some(identifier) = identifier
            && name.is_some_and(|name| self.tree.names.get(name).prefix_scope.is_some())
        {
            home = self.scopes.class_home(home, identifier);
        }
        let specialization = name.is_some_and(|name| self.name_has_arguments(name));
        let primary = if specialization {
            identifier.and_then(|id| self.scopes.lookup_name(id, true))
        } else {
            None
        };
        let existing = if specialization {
            None
        } else {
            identifier.and_then(|id| self.scopes.own(home, id))
        };
        let scope = if let Some(scope) = existing.and_then(|binding| binding.scope)
            && identifier
                .is_some_and(|identifier| self.scopes.kind(scope) == ScopeKind::Class(identifier))
            && !self.scopes.complete(scope)
        {
            self.scopes.reopen(scope, enclosing)?;
            scope
        } else {
            self.scopes.current = enclosing;
            let scope = self
                .scopes
                .push_kind(identifier.map_or(ScopeKind::AnonymousClass, ScopeKind::Class))?;
            self.scopes.current = enclosing;
            scope
        };
        if qualified {
            self.scopes.set_member(scope, home);
        }
        let binding = Binding {
            specialization: 0,
            parameter: false,
            category: if is_template || name.is_some_and(|name| self.name_has_arguments(name)) {
                Category::TypeTemplate
            } else {
                Category::Type
            },
            scope: Some(scope),
        };
        if let Some(identifier) = identifier {
            if specialization && !qualified {
                if let Some(target) = primary
                    .filter(|binding| binding.category == Category::TypeTemplate)
                    .and_then(|binding| binding.scope)
                {
                    let saved = self.scopes.current;
                    self.scopes.current = target;
                    self.scopes.add_edge(scope);
                    self.scopes.current = saved;
                }
            } else if !qualified
                || existing.is_some_and(|previous| {
                    matches!(previous.category, Category::Type | Category::TypeTemplate)
                        && previous
                            .scope
                            .is_none_or(|scope| !self.scopes.complete(scope))
                })
            {
                self.scopes.declare_in(home, identifier, binding);
            }
            // The injected-class-name is visible to its own members and bodies.
            self.scopes.declare_in(scope, identifier, binding);
        }
        if let Some(name) = name {
            let mut node = *self.tree.names.get(name);
            node.binding = Some(binding);
            self.tree.names.replace(name, node);
        }
        let bases = if self.accept(P::Colon)? {
            self.base_clause(
                scope,
                is_template && !specialization && identifier.is_some(),
            )?
        } else {
            List::default()
        };
        self.expect(P::LeftBrace)?;
        self.scopes.current = scope;
        self.class_depth += 1;
        let first_deferred = self.deferred.len();
        let runs = self.cursor.runs_mark();
        let result = self.speculation_boundary(|parser| {
            let members = parser.declarations(true)?;
            parser.expect(P::RightBrace)?;
            Ok(members)
        });
        self.class_depth -= 1;
        self.scopes.current = enclosing;
        let members = result?;
        let trailing = if matches!(self.token()?.kind, TokenKind::Identifier(id) if matches!(self.cursor.name(id), b"__attribute__" | b"__attribute"))
        {
            self.collect_attributes(false)?
        } else {
            List::default()
        };
        let start = self.tree.attributes.begin();
        self.tree.attributes.extend(attributes);
        self.tree.attributes.extend(trailing);
        let attributes = self.tree.attributes.finish(start)?;
        self.scopes.complete_class(scope);
        let class = self.tree.classes.add(ClassSyntax {
            attributes,
            location,
            key,
            name,
            scope,
            final_specifier,
            bases,
            members,
        })?;
        if self.class_depth == 0 {
            self.speculation_boundary(|parser| parser.parse_deferred_bodies(first_deferred))?;
            self.cursor.rewind_runs(runs);
        }
        Ok(Specifier::Class(class))
    }
    fn elaborated_class(
        &mut self,
        key: K,
        name: Name,
        friend: bool,
        first: bool,
    ) -> Result<Specifier, ParseError> {
        let is_template = self.pending_template;
        self.pending_template = false;
        if let Some(identifier) = self
            .simple_name(name)
            .filter(|_| !self.name_has_arguments(name))
        {
            let standalone = first && self.at(P::Semicolon)?;
            let existing = if standalone {
                self.scopes.own(self.scopes.declaration_scope(), identifier)
            } else {
                self.scopes.lookup_name(identifier, true)
            };
            let scope = if let Some(scope) = existing.and_then(|binding| binding.scope) {
                Some(scope)
            } else if !friend {
                let enclosing = self.scopes.current;
                let scope = self.scopes.push_kind(ScopeKind::Class(identifier))?;
                self.scopes.current = enclosing;
                Some(scope)
            } else {
                None
            };
            let binding = Binding {
                specialization: 0,
                parameter: false,
                category: if is_template {
                    Category::TypeTemplate
                } else {
                    Category::Type
                },
                scope,
            };
            if is_template {
                self.scopes
                    .declare_in(self.scopes.declaration_scope(), identifier, binding);
            } else if existing.is_none() && !friend {
                let where_declared = if standalone {
                    self.scopes.declaration_scope()
                } else {
                    self.scopes.enclosing_declaration_scope()
                };
                self.scopes.declare_in(where_declared, identifier, binding);
            }
            let mut node = *self.tree.names.get(name);
            node.binding = Some(binding);
            self.tree.names.replace(name, node);
        }
        Ok(Specifier::Elaborated { key, name })
    }
    fn base_clause(&mut self, class: ScopeId, primary: bool) -> Result<List<Base>, ParseError> {
        let start = self.tree.bases.begin();
        loop {
            let location = self.location()?;
            self.skip_attributes()?;
            let virtual_first = self.accept_keyword(K::Virtual)?;
            let access = if matches!(self.keyword()?, Some(K::Public | K::Protected | K::Private)) {
                let access = self.keyword()?;
                self.cursor.advance()?;
                access
            } else {
                None
            };
            let is_virtual = virtual_first || self.accept_keyword(K::Virtual)?;
            if self.keyword()? == Some(K::Typename) {
                return self.fail();
            }
            let (name, binding) = self.type_name()?;
            let pack = self.accept(P::Ellipsis)?;
            if let Some(base) = binding.and_then(|binding| binding.scope) {
                let enclosing = self.scopes.current;
                self.scopes.current = class;
                self.scopes
                    .add_base(base, self.tree.names.get(name).mentions_parameters);
                self.scopes.current = enclosing;
            } else if primary
                && self.tree.names.get(name).mentions_parameters
                && !self.name_has_arguments(name)
                && let Some(identifier) = self.simple_name(name)
            {
                self.record_parameter_base(class, identifier, pack)?;
            }
            self.tree.bases.push(Base {
                location,
                name,
                access,
                is_virtual,
                virtual_first,
                pack,
            });
            if !self.accept(P::Comma)? {
                break;
            }
        }
        self.tree.bases.finish(start)
    }
    pub(super) fn special_member_head(&mut self) -> Result<(List<Specifier>, Name), ParseError> {
        let start = self.tree.specs.begin();
        loop {
            let attributes = self.collect_attributes(false)?;
            self.append_pending_attributes(attributes);
            if let Some(
                word
                @ (K::Inline | K::Virtual | K::Explicit | K::Constexpr | K::Friend | K::Static),
            ) = self.keyword()?
            {
                self.cursor.advance()?;
                self.tree.specs.push(Specifier::Keyword(word));
            } else {
                break;
            }
        }
        if !(matches!(self.token()?.kind, TokenKind::Identifier(_))
            || self.at(P::Scope)?
            || (self.in_class()
                && (self.keyword()? == Some(K::Operator)
                    || (self.at(P::Complement)?
                        && matches!(self.cursor.peek(1)?.kind, TokenKind::Identifier(_))))))
        {
            return Err(ParseError::Syntax {
                location: self.location()?,
                message: "syntax error",
            });
        }
        let (name, _) = self.name()?;
        if !self.at(P::LeftParen)? {
            return Err(ParseError::Syntax {
                location: self.location()?,
                message: "syntax error",
            });
        }
        let node = self.tree.names.get(name);
        let components = self.tree.name_lists.get(node.components);
        let Some(last) = components.last() else {
            return Err(ParseError::Syntax {
                location: self.location()?,
                message: "syntax error",
            });
        };
        let special = if components.len() >= 2 {
            matches!(last.kind, NameKind::Destructor(_) | NameKind::Conversion(_))
                || matches!((components[components.len() - 2].kind, last.kind),
                    (NameKind::Identifier(owner), NameKind::Identifier(last)) if owner == last)
        } else if self.in_class() {
            matches!(last.kind, NameKind::Destructor(_) | NameKind::Conversion(_))
                || matches!((self.scopes.kind(self.scopes.declaration_scope()), last.kind),
                    (ScopeKind::Class(owner), NameKind::Identifier(last)) if owner == last)
        } else {
            false
        };
        if !special {
            return Err(ParseError::Syntax {
                location: self.location()?,
                message: "syntax error",
            });
        }
        Ok((self.tree.specs.finish(start)?, name))
    }
    pub(super) fn special_member(
        &mut self,
        location: SourceOffset,
        specifiers: List<Specifier>,
        name: Name,
    ) -> Result<DeclId, ParseError> {
        self.pending_template = false;
        let enclosing = self.scopes.current;
        let in_class = self.in_class();
        self.function_scope = None;
        let member = self.tree.names.get(name).prefix_scope;
        let parameters = self.parameter_clause_member(true, member)?;
        self.scopes.current = self
            .function_scope
            .ok_or(ParseError::Invariant("missing special member scope"))?;
        let function_scope = self.function_scope;
        let attribute_start = self.tree.attributes.begin();
        let result = self.function_qualifiers_with_attributes(true);
        self.function_scope = function_scope;
        self.scopes.current = enclosing;
        let qualifiers = result?;
        let attributes = self.tree.attributes.finish(attribute_start)?;
        let start = self.tree.suffixes.begin();
        self.tree.suffixes.push(Suffix::Function {
            parameters,
            qualifiers,
        });
        let suffixes = self.tree.suffixes.finish(start)?;
        let declarator = self.tree.declarators.add(Declarator {
            attributes,
            location,
            abstract_form: false,
            pointers: List::default(),
            pack: false,
            name: Some(name),
            nested: None,
            suffixes,
        })?;
        let initializer = if self.at(P::Assign)? {
            Some(self.initializer()?)
        } else {
            None
        };
        let body = if initializer.is_some() || self.accept(P::Semicolon)? {
            if initializer.is_some() {
                self.expect(P::Semicolon)?;
            }
            None
        } else {
            Some(self.finish_function_body(in_class, true)?)
        };
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::SpecialMember {
                specifiers,
                name,
                declarator,
                initializer,
                body,
            },
        })
    }
    pub(super) fn bit_field(
        &mut self,
        location: SourceOffset,
        specifiers: List<Specifier>,
        first: Option<DeclaratorId>,
    ) -> Result<DeclId, ParseError> {
        let start = self.tree.bit_fields.begin();
        let mut declarator = first;
        loop {
            if let Some(declarator) = declarator {
                self.declare_declarator(declarator, false, None, super::storage::List::default());
            }
            self.expect(P::Colon)?;
            let width = self.assignment_expression()?;
            self.tree.bit_fields.push(BitField { declarator, width });
            if !self.accept(P::Comma)? {
                break;
            }
            declarator = if self.at(P::Colon)? {
                None
            } else {
                Some(self.declarator(Mode::Named)?)
            };
        }
        self.expect(P::Semicolon)?;
        let declarators = self.tree.bit_fields.finish(start)?;
        self.tree.declarations.add(Declaration {
            attributes: super::storage::List::default(),
            location,
            kind: DeclKind::BitField {
                specifiers,
                declarators,
            },
        })
    }
    pub(super) fn finish_function_body(
        &mut self,
        defer: bool,
        constructor: bool,
    ) -> Result<BodyId, ParseError> {
        let scope = self
            .function_scope
            .ok_or(ParseError::Invariant("missing function parameter scope"))?;
        if defer {
            let begin = self.cursor.position;
            let absolute = self.cursor.absolute_position()?;
            self.cursor.retain();
            let result = self
                .skip_function_body()
                .and_then(|()| self.cursor.capture(begin, absolute));
            self.cursor.release();
            let tokens = result?;
            let target = self.tree.bodies.add(Body::Deferred)?;
            self.deferred.push(DeferredBody {
                target,
                tokens,
                scope,
                constructor,
            });
            return Ok(target);
        }
        let enclosing = self.scopes.current;
        self.scopes.current = scope;
        let result =
            self.without_declaration_attributes(|parser| parser.function_body(constructor));
        self.scopes.current = enclosing;
        self.tree.bodies.add(Body::Parsed(result?))
    }
    fn parse_deferred_bodies(&mut self, first: usize) -> Result<(), ParseError> {
        let last = self.deferred.len();
        for index in first..last {
            let entry = self.deferred[index];
            let enclosing = self.scopes.current;
            self.scopes.current = entry.scope;
            self.cursor.push_run(entry.tokens);
            let result = self
                .without_declaration_attributes(|parser| parser.function_body(entry.constructor))
                .and_then(|body| {
                    if matches!(self.token()?.kind, TokenKind::End) {
                        Ok(body)
                    } else {
                        self.fail()
                    }
                });
            self.cursor.pop_run()?;
            self.scopes.current = enclosing;
            self.tree.complete_body(entry.target, result?)?;
        }
        self.deferred.truncate(first);
        Ok(())
    }
    fn skip_function_body(&mut self) -> Result<(), ParseError> {
        let try_block = self.accept_keyword(K::Try)?;
        if self.accept(P::Colon)? {
            let mut after_name = false;
            loop {
                if matches!(self.token()?.kind, TokenKind::End) {
                    return self.fail();
                }
                if self.at(P::LeftBrace)? && !after_name {
                    break;
                }
                if matches!(
                    self.punctuator()?,
                    Some(P::LeftParen | P::LeftBracket | P::LeftBrace)
                ) {
                    self.skip_balanced()?;
                    after_name = false;
                } else {
                    after_name = matches!(self.token()?.kind, TokenKind::Identifier(_))
                        || self.at(P::Greater)?;
                    self.cursor.advance()?;
                }
            }
        }
        if !self.at(P::LeftBrace)? {
            return self.fail();
        }
        self.skip_balanced()?;
        if try_block {
            if self.keyword()? != Some(K::Catch) {
                return self.fail();
            }
            while self.accept_keyword(K::Catch)? {
                if !self.at(P::LeftParen)? {
                    return self.fail();
                }
                self.skip_balanced()?;
                if !self.at(P::LeftBrace)? {
                    return self.fail();
                }
                self.skip_balanced()?;
            }
        }
        Ok(())
    }
    pub(super) fn ctor_initializers(&mut self) -> Result<List<MemInitializer>, ParseError> {
        self.expect(P::Colon)?;
        let start = self.tree.mem_initializers.begin();
        loop {
            let location = self.location()?;
            let name = self.type_name()?.0;
            if !matches!(self.punctuator()?, Some(P::LeftBrace | P::LeftParen)) {
                return self.fail();
            }
            let initializer = self.initializer()?;
            let pack = self.accept(P::Ellipsis)?;
            self.tree.mem_initializers.push(MemInitializer {
                location,
                name,
                initializer,
                pack,
            });
            if !self.accept(P::Comma)? {
                break;
            }
        }
        self.tree.mem_initializers.finish(start)
    }
}
