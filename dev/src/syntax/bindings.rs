// Translated from syntax/parser_declarations.cpp's binding declarations. See NOTICE.
//! The hidden object's declarator precedes the names; instantiation clones syntax once.
use super::{
    ParseError,
    ast::{self, DeclKind, Declaration, InitDeclarator, Pointer},
    parser::Parser,
    storage::List,
};
use crate::{lex::pp_lexer::Punctuator as P, support::source::SourceOffset};

impl Parser<'_> {
    pub(super) fn at_structured_binding(&mut self) -> Result<bool, ParseError> {
        let ahead = usize::from(matches!(
            self.punctuator()?,
            Some(P::BitAnd | P::LogicalAnd)
        ));
        Ok(self.at_ahead(P::LeftBracket, ahead)? && !self.at_ahead(P::LeftBracket, ahead + 1)?)
    }
    pub(super) fn binding_names(
        &mut self,
        location: SourceOffset,
    ) -> Result<(ast::DeclaratorId, Vec<ast::DeclaratorId>), ParseError> {
        let reference = if matches!(self.punctuator()?, Some(P::BitAnd | P::LogicalAnd)) {
            let mut op = self.operator()?;
            // The hidden object's reference is synthesized, not a written operator.
            op.alternative = false;
            Some(op)
        } else {
            None
        };
        self.expect(P::LeftBracket)?;
        let mut names = Vec::new();
        let mut spelling = b"[".to_vec();
        loop {
            let location = self.location()?;
            let identifier = self.identifier()?;
            if !names.is_empty() {
                spelling.extend_from_slice(b", ");
            }
            spelling.extend_from_slice(self.cursor.name(identifier));
            let name = self.tree.implicit_name(identifier, location)?;
            names.push(self.tree.implicit_declarator(name, location)?);
            if !self.accept(P::Comma)? {
                break;
            }
        }
        self.expect(P::RightBracket)?;
        spelling.push(b']');
        let hidden = self.cursor.intern_name(&spelling)?;
        let name = self.tree.implicit_name(hidden, location)?;
        let object = self.tree.implicit_declarator(name, location)?;
        if let Some(op) = reference {
            let start = self.tree.pointers.begin();
            self.tree.pointers.push(Pointer {
                op,
                class_name: None,
                cv: List::default(),
            });
            let mut node = *self.tree.declarators.get(object);
            node.pointers = self.tree.pointers.finish(start)?;
            self.tree.declarators.replace(object, node);
        }
        Ok((object, names))
    }
    pub(super) fn binding_declaration(
        &mut self,
        specifiers: List<ast::Specifier>,
        location: SourceOffset,
        object: ast::DeclaratorId,
        names: &[ast::DeclaratorId],
        initializer: Option<ast::Initializer>,
    ) -> Result<ast::DeclId, ParseError> {
        let start = self.tree.init_declarators.begin();
        self.tree.init_declarators.push(InitDeclarator {
            tokens: ast::TokenSpan::default(),
            declarator: object,
            initializer,
        });
        for &declarator in names {
            self.declare_from_specifiers(declarator, specifiers)?;
            self.tree.init_declarators.push(InitDeclarator {
                tokens: ast::TokenSpan::default(),
                declarator,
                initializer: None,
            });
        }
        let declarators = self.tree.init_declarators.finish(start)?;
        self.tree.declarations.add(Declaration {
            attributes: List::default(),
            location,
            kind: DeclKind::Simple {
                decomposition: true,
                specifiers,
                declarators,
            },
        })
    }
    pub(super) fn structured_binding(
        &mut self,
        specifiers: List<ast::Specifier>,
        location: SourceOffset,
    ) -> Result<ast::DeclId, ParseError> {
        let (object, names) = self.binding_names(location)?;
        if !matches!(
            self.punctuator()?,
            Some(P::Assign | P::LeftBrace | P::LeftParen)
        ) {
            return self.decline_operand();
        }
        let initializer = self.initializer()?;
        self.expect(P::Semicolon)?;
        self.binding_declaration(specifiers, location, object, &names, Some(initializer))
    }
}
