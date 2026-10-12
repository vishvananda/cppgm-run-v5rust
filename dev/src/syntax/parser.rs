// Translated from syntax/parser{,_state,_names}.cpp. See NOTICE.
//! Shared cursor, transactions and name-category decisions.
use super::{
    ParseError,
    ast::{DeclId, DeclaratorId, Name, Operator, Tree, TreeMark},
    cursor::{Cursor, Token, TokenKind},
    scopes::{Binding, Category, ScopeId, ScopeMark, Scopes},
    storage::List,
};
use crate::lex::{
    identifiers::NameId,
    pp_lexer::Punctuator as P,
    vocabulary::{Keyword as K, SimpleToken},
};
use crate::support::source::SourceOffset;

pub(super) struct Parser<'a> {
    pub cursor: Cursor<'a>,
    pub tree: Tree,
    pub scopes: Scopes,
    pub function_scope: Option<ScopeId>,
    depth: u32,
    tentative_depth: usize,
    pub checkpoints: u64,
    pub rewinds: u64,
    pub peak_nesting: u32,
    furthest: SourceOffset,
    error_position: u64,
}
struct Checkpoint {
    position: usize,
    tree: TreeMark,
    scopes: ScopeMark,
    function_scope: Option<ScopeId>,
}
impl<'a> Parser<'a> {
    pub(super) fn new(cursor: Cursor<'a>) -> Self {
        Self {
            cursor,
            tree: Tree::default(),
            scopes: Scopes::default(),
            function_scope: None,
            depth: 0,
            tentative_depth: 0,
            checkpoints: 0,
            rewinds: 0,
            peak_nesting: 0,
            furthest: SourceOffset(0),
            error_position: 0,
        }
    }
    pub(super) fn token(&mut self) -> Result<Token, ParseError> {
        self.cursor.peek(0)
    }
    pub(super) fn location(&mut self) -> Result<SourceOffset, ParseError> {
        Ok(self.token()?.location)
    }
    pub(super) fn keyword(&mut self) -> Result<Option<K>, ParseError> {
        Ok(match self.token()?.kind {
            TokenKind::Simple(op) => op.token.as_keyword(),
            _ => None,
        })
    }
    pub(super) fn punctuator(&mut self) -> Result<Option<P>, ParseError> {
        Ok(match self.token()?.kind {
            TokenKind::Simple(op) => op.token.as_punctuator(),
            _ => None,
        })
    }
    pub(super) fn at(&mut self, punct: P) -> Result<bool, ParseError> {
        Ok(self.punctuator()? == Some(punct))
    }
    pub(super) fn at_ahead(&mut self, punct: P, ahead: usize) -> Result<bool, ParseError> {
        Ok(
            matches!(self.cursor.peek(ahead)?.kind, TokenKind::Simple(op) if op.token.as_punctuator() == Some(punct)),
        )
    }
    pub(super) fn accept(&mut self, punct: P) -> Result<bool, ParseError> {
        if self.at(punct)? {
            self.cursor.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(super) fn accept_keyword(&mut self, word: K) -> Result<bool, ParseError> {
        if self.keyword()? == Some(word) {
            self.cursor.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    pub(super) fn expect(&mut self, punct: P) -> Result<(), ParseError> {
        if self.accept(punct)? {
            Ok(())
        } else {
            self.fail()
        }
    }
    pub(super) fn fail<T>(&mut self) -> Result<T, ParseError> {
        let location = self.location()?;
        let position = self.cursor.absolute_position()?;
        if position >= self.error_position {
            self.error_position = position;
            self.furthest = location;
        }
        Err(ParseError::Syntax {
            location: self.furthest,
            message: "syntax error",
        })
    }
    pub(super) fn identifier(&mut self) -> Result<NameId, ParseError> {
        if let TokenKind::Identifier(name) = self.token()?.kind {
            self.cursor.advance()?;
            Ok(name)
        } else {
            self.fail()
        }
    }
    pub(super) fn operator(&mut self) -> Result<Operator, ParseError> {
        if let TokenKind::Simple(op) = self.token()?.kind {
            self.cursor.advance()?;
            Ok(op)
        } else {
            self.fail()
        }
    }
    pub(super) fn nested<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<T, ParseError> {
        // Safe recursion boundary until the oracle's host stack-budget policy is ported.
        if self.depth >= 256 {
            return Err(ParseError::NotImplemented(
                "host stack-budget nesting policy",
            ));
        }
        self.depth += 1;
        self.peak_nesting = self.peak_nesting.max(self.depth);
        let result = parse(self);
        self.depth -= 1;
        result
    }
    pub(super) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Option<T>, ParseError> {
        let mark = Checkpoint {
            position: self.cursor.position,
            tree: self.tree.mark(),
            scopes: self.scopes.mark(),
            function_scope: self.function_scope,
        };
        self.cursor.retain();
        self.tentative_depth += 1;
        self.checkpoints += 1;
        let result = parse(self);
        self.tentative_depth -= 1;
        if result.is_err() {
            self.cursor.position = mark.position;
            self.tree.rewind(&mark.tree);
            self.scopes.rewind(mark.scopes);
            self.function_scope = mark.function_scope;
            self.rewinds += 1;
        }
        self.cursor.release();
        match result {
            Ok(value) => Ok(Some(value)),
            Err(ParseError::Syntax { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn declarations(&mut self, until_brace: bool) -> Result<List<DeclId>, ParseError> {
        let start = self.tree.decl_lists.begin();
        loop {
            if until_brace && self.at(P::RightBrace)? {
                break;
            }
            if matches!(self.token()?.kind, TokenKind::End) {
                if until_brace {
                    return self.fail();
                }
                break;
            }
            let declaration = self.declaration()?;
            self.tree.decl_lists.push(declaration);
            if self.tentative_depth == 0 {
                self.scopes.settle();
            }
        }
        self.tree.decl_lists.finish(start)
    }
    pub(super) fn declare_declarator(
        &mut self,
        declarator: DeclaratorId,
        typedef: bool,
        type_scope: Option<ScopeId>,
    ) {
        if let Some(name) = self
            .declarator_name(declarator)
            .and_then(|name| self.simple_name(name))
        {
            self.scopes.declare(
                name,
                Binding {
                    category: if typedef {
                        Category::Type
                    } else {
                        Category::Value
                    },
                    scope: if typedef
                        && self.tree.declarators.get(declarator).pointers.is_empty()
                        && self.tree.declarators.get(declarator).suffixes.is_empty()
                        && self.tree.declarators.get(declarator).nested.is_none()
                    {
                        type_scope
                    } else {
                        None
                    },
                },
            );
        }
    }
    pub(super) fn specifier_scope(
        &self,
        specifiers: List<super::ast::Specifier>,
    ) -> Option<ScopeId> {
        for specifier in self.tree.specs.get(specifiers) {
            match specifier {
                super::ast::Specifier::Named(name) => {
                    return self
                        .tree
                        .names
                        .get(*name)
                        .binding
                        .and_then(|binding| binding.scope);
                }
                super::ast::Specifier::Enum(enumeration) => {
                    if let super::ast::DeclKind::Enum { scope, .. } =
                        self.tree.declarations.get(*enumeration).kind
                    {
                        return scope;
                    }
                }
                super::ast::Specifier::Keyword(_) | super::ast::Specifier::Decltype(_) => (),
            }
        }
        None
    }
    pub(super) fn declarator_name(&self, mut declarator: DeclaratorId) -> Option<Name> {
        loop {
            let node = self.tree.declarators.get(declarator);
            if node.name.is_some() {
                return node.name;
            }
            declarator = node.nested?;
        }
    }
    pub(super) fn has_type_keyword(word: K) -> bool {
        matches!(
            word,
            K::Auto
                | K::Bool
                | K::Char
                | K::Char16T
                | K::Char32T
                | K::Double
                | K::Float
                | K::Int
                | K::Long
                | K::Short
                | K::Signed
                | K::Unsigned
                | K::Void
                | K::WcharT
        )
    }
    pub(super) fn is_specifier(word: K) -> bool {
        Self::has_type_keyword(word)
            || matches!(
                word,
                K::Typedef
                    | K::Friend
                    | K::Static
                    | K::Extern
                    | K::Inline
                    | K::Virtual
                    | K::Explicit
                    | K::Constexpr
                    | K::ThreadLocal
                    | K::Mutable
                    | K::Register
                    | K::Const
                    | K::Volatile
            )
    }
    pub(super) fn attributes_boundary(&mut self) -> Result<(), ParseError> {
        if self.keyword()? == Some(K::Alignas)
            || (self.at(P::LeftBracket)? && self.at_ahead(P::LeftBracket, 1)?)
            || matches!(self.token()?.kind, TokenKind::Identifier(name) if matches!(self.cursor.name(name), b"__attribute__" | b"__attribute"))
        {
            return Err(ParseError::NotImplemented(
                "attribute syntax and retained attribute facts",
            ));
        }
        Ok(())
    }
    pub(super) fn make_operator(punct: P) -> Result<Operator, ParseError> {
        Ok(Operator {
            token: SimpleToken::punctuator(punct)
                .ok_or(ParseError::Invariant("invalid syntax operator"))?,
            alternative: false,
        })
    }
}
