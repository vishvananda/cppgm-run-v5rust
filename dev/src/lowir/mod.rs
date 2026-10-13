//! Reusable `LowIR` reader, function-owned model, validator and writer. See NOTICE.
mod builder;
mod exercises;
mod globals;
mod instructions;
mod lexer;
mod model;
mod reader;
mod validator;
mod vocabulary;
mod writer;

/// A `LowIR` failure retained until the command-line boundary renders it.
#[derive(Debug)]
pub enum Error {
    Diagnostic(String),
    Construction(String),
    NotImplemented(&'static str),
    Limit,
    Output(std::io::Error),
}
/// Read one shared unit in file order, validate it, then serialize it.
///
/// # Errors
/// Returns structural, input/output or explicitly unimplemented feature failures.
pub fn roundtrip(paths: &[String], output: &str) -> Result<(), Error> {
    let mut program = model::Program::default();
    for path in paths {
        let source =
            std::fs::read(path).map_err(|_| Error::Diagnostic(format!("cannot read {path}")))?;
        reader::Parser::new(&mut program, &source, path)?.parse()?;
    }
    program.finish_symbols()?;
    program.legacy_roles();
    validator::validate(&program)?;
    write_file(&program, output)
}
/// Construct and validate a PA8 exercise through the shared typed model.
///
/// # Errors
/// Returns invalid exercise, construction, storage or output failures.
pub fn exercise(name: &str, output: &str) -> Result<(), Error> {
    let mut program = model::Program::default();
    exercises::build(&mut program, name)?;
    validator::validate_constructed(&program).map_err(|error| match error {
        Error::Diagnostic(message) => Error::Construction(message),
        error => error,
    })?;
    write_file(&program, output)
}
fn write_file(program: &model::Program, output: &str) -> Result<(), Error> {
    let file = std::fs::File::create(output)
        .map_err(|_| Error::Diagnostic(format!("cannot write {output}")))?;
    writer::write(program, std::io::BufWriter::new(file))
        .map_err(|_| Error::Diagnostic(format!("cannot write {output}")))
}
