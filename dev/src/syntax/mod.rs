//! Parse-once source syntax over the shared preprocessing and phase-7 owners.
mod angles;
mod ast;
mod attributes;
mod classes;
mod cursor;
mod declarations;
mod declarators;
mod expressions;
mod lambdas;
mod names;
mod parser;
mod printer;
mod render;
mod scopes;
mod statements;
mod storage;
mod templates;

use crate::lex::{PostTokenizer, number::NumberDialect};
use crate::preprocess::{error::PreprocessError, unit::Preprocessor};
use crate::support::source::SourceOffset;
use ast::DeclId;
use storage::List;

#[derive(Debug)]
pub(crate) enum ParseError {
    Syntax {
        location: SourceOffset,
        message: &'static str,
    },
    CommittedSyntax {
        location: SourceOffset,
        message: &'static str,
    },
    Preprocess(PreprocessError),
    Limit(&'static str),
    Invariant(&'static str),
    NotImplemented(&'static str),
}
/// The parser and all syntax storage belong to one translation unit. Printing
/// borrows the tree; later phases will consume the same typed records.
pub(crate) struct SyntaxUnit<'a> {
    parser: parser::Parser<'a>,
    declarations: Option<List<DeclId>>,
}
impl<'a> SyntaxUnit<'a> {
    pub(crate) fn new(preprocessor: Preprocessor<'a>) -> Self {
        Self {
            parser: parser::Parser::new(cursor::Cursor::new(PostTokenizer::from_input(
                preprocessor,
                NumberDialect::Cxx11,
            ))),
            declarations: None,
        }
    }
    pub(crate) fn parse(&mut self) -> Result<(), ParseError> {
        if self.declarations.is_some() {
            return Ok(());
        }
        self.declarations = Some(self.parser.declarations(false)?);
        Ok(())
    }
    pub(crate) fn describe(&self, error: &ParseError) -> Result<Vec<u8>, PreprocessError> {
        self.parser.cursor.describe(error)
    }
    pub(crate) fn print(
        &self,
        output: &mut (impl std::io::Write + ?Sized),
    ) -> Result<(), std::io::Error> {
        let declarations = self
            .declarations
            .ok_or_else(|| std::io::Error::other("missing parsed translation unit"))?;
        printer::print(&self.parser.tree, &self.parser.cursor, declarations, output)
    }
}

#[cfg(test)]
mod tests;
