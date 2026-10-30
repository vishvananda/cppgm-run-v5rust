//! Parse-once source syntax over the shared preprocessing and phase-7 owners.
mod angles;
mod assembly;
pub mod ast;
mod attributes;
mod bindings;
mod classes;
pub mod cloning;
mod conflicts;
pub mod cursor;
mod declarations;
mod declarators;
mod expressions;
mod lambdas;
mod name_lookup;
mod names;
pub mod observations;
mod parameter_bases;
mod parser;
mod printer;
mod render;
pub use render::operator_spelling;
mod scopes;
mod statements;
pub mod storage;
mod templates;
mod type_transforms;

use crate::lex::PostTokenizer;
use crate::preprocess::{error::PreprocessError, unit::Preprocessor};
use crate::support::source::SourceOffset;
use ast::DeclId;
use storage::List;

/// A source-facing special-member label, never a semantic lookup key.
pub fn write_expression_name(
    tree: &ast::Tree,
    cursor: &cursor::Cursor<'_>,
    name: ast::Name,
    output: &mut (impl std::io::Write + ?Sized),
) -> std::io::Result<()> {
    let mut renderer = render::Renderer::new(tree, cursor);
    renderer.render(render::Render::Name(name, true))?;
    output.write_all(&renderer.bytes)
}

/// A source-facing special-member label, never a semantic lookup key.
pub fn write_member_label(
    tree: &ast::Tree,
    cursor: &cursor::Cursor<'_>,
    name: ast::Name,
    output: &mut (impl std::io::Write + ?Sized),
) -> std::io::Result<()> {
    let component = *tree
        .name_lists
        .get(tree.names.get(name).components)
        .last()
        .ok_or_else(|| std::io::Error::other("empty special member name"))?;
    let mut renderer = render::Renderer::new(tree, cursor);
    renderer.render(render::Render::Component(
        component,
        !matches!(component.kind, ast::NameKind::Operator { .. }),
    ))?;
    if matches!(component.kind, ast::NameKind::Conversion(_)) {
        renderer.bytes.retain(|&byte| byte != b' ');
    }
    output.write_all(&renderer.bytes)
}

#[derive(Debug)]
pub enum ParseError {
    /// The pack expander owns the typed error that stopped this copy.
    CloneAbort,
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
}
/// The parser and all syntax storage belong to one translation unit. Printing
/// borrows the tree; later phases will consume the same typed records.
pub struct SyntaxUnit<'a> {
    parser: parser::Parser<'a>,
    declarations: Option<List<DeclId>>,
}
impl<'a> SyntaxUnit<'a> {
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn new(preprocessor: Preprocessor<'a>) -> Self {
        Self::with_token_origin(
            preprocessor,
            0,
            crate::lex::number::NumberOptions {
                dialect: crate::lex::number::NumberDialect::Gnu,
                binary_exponents: false,
            },
        )
    }
    #[must_use]
    pub fn with_token_origin(
        mut preprocessor: Preprocessor<'a>,
        origin: u64,
        numbers: crate::lex::number::NumberOptions,
    ) -> Self {
        let hosted = preprocessor.hosted();
        preprocessor.set_number_options(numbers);
        let mut cursor =
            cursor::Cursor::new(PostTokenizer::from_input(preprocessor, numbers.dialect));
        cursor.set_origin(origin);
        let mut unit = Self {
            parser: parser::Parser::new(cursor),
            declarations: None,
        };
        unit.set_hosted(hosted);
        unit
    }
    pub fn set_hosted(&mut self, hosted: bool) {
        self.parser.environment = if hosted {
            parser::Environment::Hosted
        } else {
            parser::Environment::Course
        };
    }
    #[inline]
    pub fn set_source_origin(&mut self, origin: u64) {
        self.parser.cursor.set_source_origin(origin);
    }
    pub fn parse(&mut self) -> Result<(), ParseError> {
        if self.declarations.is_some() {
            return Ok(());
        }
        let marker = 0u8;
        self.parser.stack_base = Some(std::ptr::from_ref(&marker).addr());
        self.declarations = Some(self.parser.declarations(false)?);
        Ok(())
    }
    #[inline]
    pub fn set_stack_budget(&mut self, budget: crate::support::stack::StackBudget) {
        self.parser.stack_budget = budget;
    }
    #[inline]
    pub fn describe(&self, error: &ParseError) -> Result<Vec<u8>, PreprocessError> {
        self.parser.cursor.describe(error)
    }
    #[inline]
    pub fn semantic_inputs(
        &mut self,
    ) -> Result<(&mut ast::Tree, &mut cursor::Cursor<'a>, List<DeclId>), ParseError> {
        let declarations = self
            .declarations
            .ok_or(ParseError::Invariant("missing parsed translation unit"))?;
        Ok((&mut self.parser.tree, &mut self.parser.cursor, declarations))
    }
    pub fn print(&self, output: &mut (impl std::io::Write + ?Sized)) -> Result<(), std::io::Error> {
        let declarations = self
            .declarations
            .ok_or_else(|| std::io::Error::other("missing parsed translation unit"))?;
        printer::print(&self.parser.tree, &self.parser.cursor, declarations, output)
    }
}

#[cfg(test)]
mod tests;
