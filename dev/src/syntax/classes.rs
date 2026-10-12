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
            self.scopes.kind(self.scopes.current),
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
        self.attributes_boundary()?;
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
            return self.elaborated_class(key, name, friend, first);
        }
        let enclosing = self.scopes.current;
        let identifier = name.and_then(|name| self.last_identifier(name));
        let home = name
            .and_then(|name| self.tree.names.get(name).prefix_scope)
            .unwrap_or(enclosing);
        let existing = identifier.and_then(|id| self.scopes.own(home, id));
        let scope = if let Some(scope) = existing.and_then(|binding| binding.scope)
            && !self.scopes.complete(scope)
        {
            scope
        } else {
            self.scopes.current = home;
            let scope = self
                .scopes
                .push_kind(identifier.map_or(ScopeKind::AnonymousClass, ScopeKind::Class))?;
            self.scopes.current = enclosing;
            scope
        };
        let binding = Binding {
            category: Category::Type,
            scope: Some(scope),
        };
        if let Some(identifier) = identifier {
            self.scopes.declare_in(home, identifier, binding);
            // The injected-class-name is visible to its own members and bodies.
            self.scopes.declare_in(scope, identifier, binding);
        }
        if let Some(name) = name {
            let mut node = *self.tree.names.get(name);
            node.binding = Some(binding);
            self.tree.names.replace(name, node);
        }
        let bases = if self.accept(P::Colon)? {
            self.base_clause(scope)?
        } else {
            List::default()
        };
        self.expect(P::LeftBrace)?;
        self.scopes.current = scope;
        self.class_depth += 1;
        let first_deferred = self.deferred.len();
        let runs = self.cursor.runs_mark();
        let result = self.declarations(true).and_then(|members| {
            self.expect(P::RightBrace)?;
            Ok(members)
        });
        self.class_depth -= 1;
        self.scopes.current = enclosing;
        let members = result?;
        self.scopes.complete_class(scope);
        let class = self.tree.classes.add(ClassSyntax {
            location,
            key,
            name,
            scope,
            final_specifier,
            bases,
            members,
        })?;
        if self.class_depth == 0 {
            self.parse_deferred_bodies(first_deferred)?;
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
        if let Some(identifier) = self.simple_name(name) {
            let standalone = first && self.at(P::Semicolon)?;
            let existing = if standalone {
                self.scopes.own(self.scopes.current, identifier)
            } else {
                self.scopes.lookup(identifier)
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
                category: Category::Type,
                scope,
            };
            if existing.is_none() && !friend {
                let where_declared = if standalone {
                    self.scopes.current
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
    fn base_clause(&mut self, class: ScopeId) -> Result<List<Base>, ParseError> {
        let start = self.tree.bases.begin();
        loop {
            let location = self.location()?;
            self.attributes_boundary()?;
            let virtual_first = self.accept_keyword(K::Virtual)?;
            let access = if matches!(self.keyword()?, Some(K::Public | K::Protected | K::Private)) {
                let access = self.keyword()?;
                self.cursor.advance()?;
                access
            } else {
                None
            };
            let is_virtual = virtual_first || self.accept_keyword(K::Virtual)?;
            let (name, binding) = self.type_name()?;
            let pack = self.accept(P::Ellipsis)?;
            if let Some(base) = binding.and_then(|binding| binding.scope) {
                let enclosing = self.scopes.current;
                self.scopes.current = class;
                self.scopes.add_edge(base);
                self.scopes.current = enclosing;
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
            self.attributes_boundary()?;
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
                || matches!((self.scopes.kind(self.scopes.current), last.kind),
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
        let enclosing = self.scopes.current;
        let in_class = self.in_class();
        self.function_scope = None;
        if let Some(scope) = self.tree.names.get(name).prefix_scope {
            self.scopes.current = scope;
        }
        let parameters = self.parameter_clause(true)?;
        self.scopes.current = self
            .function_scope
            .ok_or(ParseError::Invariant("missing special member scope"))?;
        let result = self.function_qualifiers();
        self.scopes.current = enclosing;
        let qualifiers = result?;
        let start = self.tree.suffixes.begin();
        self.tree.suffixes.push(Suffix::Function {
            parameters,
            qualifiers,
        });
        let suffixes = self.tree.suffixes.finish(start)?;
        let declarator = self.tree.declarators.add(Declarator {
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
                self.declare_declarator(declarator, false, None);
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
        let result = self.function_body(constructor);
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
            let result = self.function_body(entry.constructor).and_then(|body| {
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
    fn skip_balanced(&mut self) -> Result<(), ParseError> {
        let close = match self.punctuator()? {
            Some(P::LeftParen) => P::RightParen,
            Some(P::LeftBracket) => P::RightBracket,
            Some(P::LeftBrace) => P::RightBrace,
            _ => return self.fail(),
        };
        self.cursor.advance()?;
        self.nested(|parser| {
            while !parser.at(close)? {
                match parser.punctuator()? {
                    Some(P::LeftParen | P::LeftBracket | P::LeftBrace) => parser.skip_balanced()?,
                    Some(P::RightParen | P::RightBracket | P::RightBrace) => return parser.fail(),
                    _ if matches!(parser.token()?.kind, TokenKind::End) => return parser.fail(),
                    _ => parser.cursor.advance()?,
                }
            }
            parser.cursor.advance()
        })
    }
    pub(super) fn ctor_initializers(&mut self) -> Result<List<MemInitializer>, ParseError> {
        self.expect(P::Colon)?;
        let start = self.tree.mem_initializers.begin();
        loop {
            let location = self.location()?;
            let name = self.type_name()?.0;
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
