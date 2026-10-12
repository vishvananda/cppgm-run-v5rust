// Translated from cppgm++.cpp::run_emit_ast_mode. See NOTICE.
//! One source owner and one parsed syntax unit at a time.
use crate::preprocess::{error::PreprocessError, options::PredefinedValues, unit::Preprocessor};
use crate::support::source::SourceBuffer;
use crate::syntax::{ParseError, SyntaxUnit};
#[derive(Debug)]
pub enum AstToolError {
    Initialization(PreprocessError),
    Diagnostic(Vec<u8>),
    Missing(&'static str),
    Internal(&'static str),
    Output(std::io::Error),
}
/// Parse and print one translation unit using the shared typed front end.
///
/// # Errors
/// Returns a located input failure, missing group, or output error.
pub fn emit_ast(
    source: &SourceBuffer,
    path: &[u8],
    values: &PredefinedValues,
    output: &mut (impl std::io::Write + ?Sized),
    ordinal: usize,
    stack_budget: crate::support::stack::StackBudget,
) -> Result<(), AstToolError> {
    let preprocessor = Preprocessor::with_options(
        source,
        path,
        crate::preprocess::files::FileServices {
            identity: super::file_identity::query,
            read: super::file_identity::read,
        },
        values,
    )
    .map_err(AstToolError::Initialization)?;
    let mut unit = SyntaxUnit::new(preprocessor);
    unit.set_stack_budget(stack_budget);
    if let Err(error) = unit.parse() {
        return Err(match error {
            ParseError::NotImplemented(group)
            | ParseError::Preprocess(PreprocessError::NotImplemented(group)) => {
                AstToolError::Missing(group)
            }
            ParseError::Invariant(message)
            | ParseError::Preprocess(PreprocessError::Invariant(message)) => {
                AstToolError::Internal(message)
            }
            _ => AstToolError::Diagnostic(
                unit.describe(&error)
                    .map_err(AstToolError::Initialization)?,
            ),
        });
    }
    writeln!(output, "start translation unit {ordinal}").map_err(AstToolError::Output)?;
    unit.print(output).map_err(AstToolError::Output)?;
    writeln!(output, "end translation unit").map_err(AstToolError::Output)
}
