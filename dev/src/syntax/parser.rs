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
    pub conflicts: super::conflicts::NameConflicts,
    pub shared_lambdas: super::conflicts::SharedLambdas,
    pub function_scope: Option<ScopeId>,
    pub template_mentions: u64,
    pub pack_mentions: u64,
    pub parameter_pack_start: u64,
    pub angle: bool,
    pub pending_template: bool,
    pub template_head: Option<super::ast::TemplateParameterClause>,
    pub specialization_records: Vec<List<super::ast::TemplateArgument>>,
    pub pending_attributes: Option<usize>,
    pub leading_attributes: List<super::ast::Attribute>,
    pub pending_class_tags: List<super::ast::Attribute>,
    pub angles: super::angles::Angles,
    pub class_depth: usize,
    pub deferred: Vec<super::classes::DeferredBody>,
    pub lambdas: Vec<super::lambdas::DeferredLambda>,
    pub lambda_changes: Vec<super::scopes::Change>,
    pub speculation: usize,
    closers: Vec<P>,
    pub deferred_lambdas: u64,
    depth: u32,
    pub stack_budget: crate::support::stack::StackBudget,
    pub stack_base: Option<usize>,
    tentative_depth: usize,
    pub checkpoints: u64,
    pub rewinds: u64,
    pub peak_nesting: u32,
    furthest: SourceOffset,
    error_position: u64,
    initialized: bool,
    pub extended_types: [Option<NameId>; 10],
    pub complex_types: [Option<NameId>; 3],
}
struct Checkpoint {
    conflict: Option<super::conflicts::StatementConflict>,
    shared_lambdas: usize,
    position: usize,
    tree: TreeMark,
    scopes: ScopeMark,
    function_scope: Option<ScopeId>,
    template_mentions: u64,
    pack_mentions: u64,
    parameter_pack_start: u64,
    angle: bool,
    pending_template: bool,
    template_head: Option<super::ast::TemplateParameterClause>,
    specialization_records: usize,
    pending_attributes: Option<usize>,
    leading_attributes: List<super::ast::Attribute>,
    pending_class_tags: List<super::ast::Attribute>,
    class_depth: usize,
    deferred: usize,
    lambdas: usize,
    runs: usize,
}
impl<'a> Parser<'a> {
    pub(super) fn new(cursor: Cursor<'a>) -> Self {
        Self {
            cursor,
            tree: Tree::default(),
            scopes: Scopes::default(),
            conflicts: super::conflicts::NameConflicts::default(),
            shared_lambdas: super::conflicts::SharedLambdas::default(),
            function_scope: None,
            template_mentions: 0,
            pack_mentions: 0,
            parameter_pack_start: 0,
            angle: false,
            pending_template: false,
            template_head: None,
            specialization_records: Vec::new(),
            pending_attributes: None,
            leading_attributes: List::default(),
            pending_class_tags: List::default(),
            angles: super::angles::Angles::default(),
            class_depth: 0,
            deferred: Vec::new(),
            lambdas: Vec::new(),
            lambda_changes: Vec::new(),
            speculation: 0,
            closers: Vec::new(),
            deferred_lambdas: 0,
            depth: 0,
            stack_budget: crate::support::stack::StackBudget::default(),
            stack_base: None,
            tentative_depth: 0,
            checkpoints: 0,
            rewinds: 0,
            peak_nesting: 0,
            furthest: SourceOffset(0),
            error_position: 0,
            initialized: false,
            extended_types: [None; 10],
            complex_types: [None; 3],
        }
    }
    pub(super) fn token(&mut self) -> Result<Token, ParseError> {
        self.initialize()?;
        self.cursor.peek(0)
    }
    fn initialize(&mut self) -> Result<(), ParseError> {
        if !self.initialized {
            let mut extended = 0;
            let mut complex = 0;
            for (spelling, category) in [
                (b"nullptr_t".as_slice(), Category::Type),
                (b"__func__".as_slice(), Category::Value),
                (b"__FUNCTION__".as_slice(), Category::Value),
                (b"__PRETTY_FUNCTION__".as_slice(), Category::Value),
                (b"__int128".as_slice(), Category::Type),
                (b"__int128_t".as_slice(), Category::Type),
                (b"__uint128_t".as_slice(), Category::Type),
                (b"_Float16".as_slice(), Category::Type),
                (b"_Float32".as_slice(), Category::Type),
                (b"_Float64".as_slice(), Category::Type),
                (b"_Float128".as_slice(), Category::Type),
                (b"_Float32x".as_slice(), Category::Type),
                (b"_Float64x".as_slice(), Category::Type),
                (b"__float128".as_slice(), Category::Type),
                (b"_Complex".as_slice(), Category::Type),
                (b"__complex__".as_slice(), Category::Type),
                (b"__complex".as_slice(), Category::Type),
                (b"__bf16".as_slice(), Category::Type),
            ] {
                let name = self.cursor.intern_name(spelling)?;
                if matches!(
                    spelling,
                    b"__int128"
                        | b"__int128_t"
                        | b"__uint128_t"
                        | b"_Float16"
                        | b"_Float32"
                        | b"_Float64"
                        | b"_Float128"
                        | b"_Float32x"
                        | b"_Float64x"
                        | b"__float128"
                ) {
                    self.extended_types[extended] = Some(name);
                    extended += 1;
                } else if matches!(spelling, b"_Complex" | b"__complex__" | b"__complex") {
                    self.complex_types[complex] = Some(name);
                    complex += 1;
                }
                self.scopes.declare_in(
                    Scopes::global(),
                    name,
                    Binding {
                        pack: false,
                        alternatives: super::scopes::Categories::default(),
                        guesses: super::scopes::Guesses::default(),
                        specialization: 0,
                        parameter: false,
                        category,
                        scope: None,
                    },
                );
            }
            self.initialized = true;
        }
        Ok(())
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
    pub(super) fn skip_balanced(&mut self) -> Result<(), ParseError> {
        // The oracle uses one reusable closer stack, independent of parser
        // recursion. Only this balanced region is demanded, not its grammar.
        self.closers.clear();
        loop {
            if matches!(self.token()?.kind, TokenKind::End) {
                return self.fail();
            }
            match self.punctuator()? {
                Some(P::LeftParen) => self.closers.push(P::RightParen),
                Some(P::LeftBracket) => self.closers.push(P::RightBracket),
                Some(P::LeftBrace) => self.closers.push(P::RightBrace),
                Some(close @ (P::RightParen | P::RightBracket | P::RightBrace)) => {
                    if self.closers.last() != Some(&close) {
                        return self.fail();
                    }
                    self.closers.pop();
                }
                _ if self.closers.is_empty() => return self.fail(),
                _ => (),
            }
            self.cursor.advance()?;
            if self.closers.is_empty() {
                return Ok(());
            }
        }
    }
    pub(super) fn error_mark(&self) -> (u64, SourceOffset) {
        (self.error_position, self.furthest)
    }
    pub(super) fn restore_error(&mut self, mark: (u64, SourceOffset)) {
        self.error_position = mark.0;
        self.furthest = mark.1;
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
        self.stack_check()?;
        self.depth += 1;
        self.peak_nesting = self.peak_nesting.max(self.depth);
        let result = parse(self);
        self.depth -= 1;
        result
    }
    #[cfg(test)]
    pub(super) fn active_nesting(&self) -> u32 {
        self.depth
    }
    pub(super) fn stack_check(&mut self) -> Result<(), ParseError> {
        // Only numeric stack-distance accounting uses an address. The pointer
        // is never retained or dereferenced; all syntax still owns typed IDs.
        let marker = 0u8;
        let here = std::ptr::from_ref(&marker).addr();
        let base = *self.stack_base.get_or_insert(here);
        if self.stack_budget.exceeded(base, here) {
            // Like the C++ ParseError exception, this failure escapes all
            // tentative grammar alternatives instead of becoming a decline.
            return Err(ParseError::CommittedSyntax {
                location: self.location()?,
                message: "constructs nested too deeply",
            });
        }
        Ok(())
    }
    pub(super) fn attempt<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Option<T>, ParseError> {
        self.transaction(true, parse)
    }
    // Lookahead returns a decision, never a syntax ID from its rewound store.
    pub(super) fn lookahead<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Option<T>, ParseError> {
        self.transaction(false, parse)
    }
    fn transaction<T>(
        &mut self,
        commit: bool,
        parse: impl FnOnce(&mut Self) -> Result<T, ParseError>,
    ) -> Result<Option<T>, ParseError> {
        self.initialize()?;
        let mark = Checkpoint {
            conflict: self.conflicts.statement,
            shared_lambdas: self.shared_lambdas.mark(),
            position: self.cursor.position,
            tree: self.tree.mark(),
            scopes: self.scopes.mark(),
            function_scope: self.function_scope,
            template_mentions: self.template_mentions,
            pack_mentions: self.pack_mentions,
            parameter_pack_start: self.parameter_pack_start,
            angle: self.angle,
            pending_template: self.pending_template,
            template_head: self.template_head,
            specialization_records: self.specialization_records.len(),
            pending_attributes: self.pending_attributes,
            leading_attributes: self.leading_attributes,
            pending_class_tags: self.pending_class_tags,
            class_depth: self.class_depth,
            deferred: self.deferred.len(),
            lambdas: self.lambdas.len(),
            runs: self.cursor.runs_mark(),
        };
        self.cursor.retain();
        self.tentative_depth += 1;
        self.speculation += 1;
        self.checkpoints += 1;
        let result = parse(self);
        self.speculation -= 1;
        if result.is_err() || !commit {
            self.shared_lambdas.rewind(mark.shared_lambdas);
            self.conflicts.statement = mark.conflict;
            self.cursor.position = mark.position;
            self.tree.rewind(&mark.tree);
            self.scopes.rewind(mark.scopes);
            self.function_scope = mark.function_scope;
            self.template_mentions = mark.template_mentions;
            self.pack_mentions = mark.pack_mentions;
            self.parameter_pack_start = mark.parameter_pack_start;
            self.angle = mark.angle;
            self.pending_template = mark.pending_template;
            self.template_head = mark.template_head;
            self.specialization_records
                .truncate(mark.specialization_records);
            self.pending_attributes = mark.pending_attributes;
            self.leading_attributes = mark.leading_attributes;
            self.pending_class_tags = mark.pending_class_tags;
            self.class_depth = mark.class_depth;
            self.deferred.truncate(mark.deferred);
            self.lambdas.truncate(mark.lambdas);
            self.cursor.rewind_runs(mark.runs);
            self.rewinds += 1;
        }
        // The outer alternative has committed. Body failure is terminal:
        // the same body occurs in every reading, so no enclosing fallback
        // may turn this error into a different declaration/expression parse.
        let bodies = if commit && result.is_ok() && self.speculation == 0 {
            self.parse_deferred_lambdas(mark.lambdas)
        } else {
            Ok(())
        };
        self.tentative_depth -= 1;
        self.cursor.release();
        bodies?;
        match result {
            Ok(value) => Ok(Some(value)),
            Err(ParseError::Syntax { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn declaration_token_ordinal(&self) -> Result<u32, ParseError> {
        // The frozen init-declarator range explicitly keeps the low target
        // 32 bits of the unit ordinal, independently of host usize width.
        let bytes = self.cursor.absolute_position()?.to_le_bytes();
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
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
            if self.tentative_depth == 0 && self.class_depth == 0 {
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
        specifiers: List<super::ast::Specifier>,
    ) {
        if let Some(name) = self
            .declarator_name(declarator)
            .filter(|name| !self.name_has_arguments(*name))
            .and_then(|name| self.simple_name(name))
        {
            let plain_typedef = typedef
                && self.tree.declarators.get(declarator).pointers.is_empty()
                && self.tree.declarators.get(declarator).suffixes.is_empty()
                && self.tree.declarators.get(declarator).nested.is_none();
            let specialization = if plain_typedef {
                self.typedef_record(specifiers)
            } else {
                0
            };
            let mut binding = Binding {
                pack: false,
                alternatives: super::scopes::Categories::default(),
                guesses: super::scopes::Guesses::default(),
                specialization,
                parameter: false,
                category: if typedef {
                    Category::Type
                } else if self.pending_template {
                    Category::FunctionTemplate
                } else {
                    Category::Value
                },
                scope: if plain_typedef { type_scope } else { None },
            };
            if plain_typedef {
                self.note_type_guesses(&mut binding, specifiers, false);
            }
            self.scopes
                .declare_in(self.scopes.declaration_scope(), name, binding);
        }
    }
    pub(super) fn specifier_scope(
        &self,
        specifiers: List<super::ast::Specifier>,
    ) -> Option<ScopeId> {
        for specifier in self.tree.specs.get(specifiers) {
            match &specifier.kind {
                super::ast::SpecifierKind::Named(name) => {
                    return self
                        .tree
                        .names
                        .get(*name)
                        .binding
                        .and_then(|binding| binding.scope);
                }
                super::ast::SpecifierKind::Enum(enumeration) => {
                    if let super::ast::DeclKind::Enum { scope, .. } =
                        self.tree.declarations.get(*enumeration).kind
                    {
                        return scope;
                    }
                }
                super::ast::SpecifierKind::Class(id) => {
                    return Some(self.tree.classes.get(*id).scope);
                }
                super::ast::SpecifierKind::Elaborated { name, .. } => {
                    return self
                        .tree
                        .names
                        .get(*name)
                        .binding
                        .and_then(|binding| binding.scope);
                }
                super::ast::SpecifierKind::Keyword(_)
                | super::ast::SpecifierKind::Decltype(_)
                | super::ast::SpecifierKind::Vector { .. } => (),
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
    pub(super) fn make_operator(punct: P) -> Result<Operator, ParseError> {
        Ok(Operator {
            token: SimpleToken::punctuator(punct)
                .ok_or(ParseError::Invariant("invalid syntax operator"))?,
            alternative: false,
        })
    }
}
