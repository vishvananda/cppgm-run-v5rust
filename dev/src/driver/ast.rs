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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EmitMode {
    Ast,
    Types,
}
/// Parse and print one translation unit using the shared typed front end.
///
/// # Errors
/// Returns a located input failure, missing group, or output error.
pub fn emit_unit(
    source: &SourceBuffer,
    path: &[u8],
    values: &PredefinedValues,
    output: &mut (impl std::io::Write + ?Sized),
    ordinal: usize,
    stack_budget: crate::support::stack::StackBudget,
    mode: EmitMode,
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
    let graph = if mode == EmitMode::Types {
        let (tree, cursor, declarations) = unit
            .semantic_inputs()
            .map_err(|_| AstToolError::Internal("missing syntax"))?;
        let analysis = crate::sema::Analyzer::new(tree, cursor, stack_budget)
            .and_then(|analyzer| analyzer.analyze(declarations));
        Some(analysis.map_err(|error| match error {
            crate::sema::Error::Diagnostic { location, message }
            | crate::sema::Error::NonConstant { location, message } => {
                match unit.describe(&ParseError::Syntax { location, message }) {
                    Ok(bytes) => AstToolError::Diagnostic(bytes),
                    Err(error) => AstToolError::Initialization(error),
                }
            }
            crate::sema::Error::Missing(group) => AstToolError::Missing(group),
            crate::sema::Error::Limit(message) => {
                AstToolError::Diagnostic(message.as_bytes().to_vec())
            }
            crate::sema::Error::Invariant(message) => AstToolError::Internal(message),
        })?)
    } else {
        None
    };
    writeln!(output, "start translation unit {ordinal}").map_err(AstToolError::Output)?;
    if let Some(graph) = graph {
        let (tree, cursor, _) = unit
            .semantic_inputs()
            .map_err(|_| AstToolError::Internal("missing syntax"))?;
        graph
            .print(tree, cursor, output)
            .map_err(AstToolError::Output)?;
    } else {
        unit.print(output).map_err(AstToolError::Output)?;
    }
    writeln!(output, "end translation unit").map_err(AstToolError::Output)
}
