//! Reusable `LowIR` reader, function-owned model, validator and writer. See NOTICE.
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
        reader::Parser::new(&mut program, &source, path).parse()?;
    }
    program.legacy_roles();
    validator::validate(&program)?;
    let file = std::fs::File::create(output)
        .map_err(|_| Error::Diagnostic(format!("cannot write {output}")))?;
    writer::write(&program, std::io::BufWriter::new(file))
        .map_err(|_| Error::Diagnostic(format!("cannot write {output}")))
}
