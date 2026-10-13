//! Reusable `LowIR` reader, function-owned model, validator and writer. See NOTICE.
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
mod builder;
mod exercises;
mod globals;
mod instructions;
mod lexer;
mod literals;
mod model;
mod reader;
mod runtime;
mod statistics;
pub use statistics::Statistics;
mod validator;
mod vocabulary;
mod writer;

/// A `LowIR` failure retained until the command-line boundary renders it.
#[derive(Debug)]
pub enum Error {
    Diagnostic(String),
    ByteDiagnostic(Vec<u8>),
    Construction(String),
    NotImplemented(&'static str),
    Limit,
    Output(std::io::Error),
}
impl Error {
    pub(crate) fn argument(prefix: &str, argument: &OsStr) -> Self {
        let mut message = prefix.as_bytes().to_vec();
        message.extend_from_slice(argument.as_encoded_bytes());
        Self::ByteDiagnostic(message)
    }
    pub(super) fn at_path(message: impl AsRef<[u8]>, path: &Path, line: u32) -> Self {
        let mut message = message.as_ref().to_vec();
        message.extend_from_slice(b" at ");
        message.extend_from_slice(path.as_os_str().as_encoded_bytes());
        message.extend_from_slice(format!(":{line}").as_bytes());
        Self::ByteDiagnostic(message)
    }
}
/// Read one shared unit in file order, validate it, then serialize it.
///
/// # Errors
/// Returns structural, input/output or explicitly unimplemented feature failures.
pub fn roundtrip(paths: &[PathBuf], output: &Path, statistics: bool) -> Result<Statistics, Error> {
    let mut program = model::Program::default();
    let start = std::time::Instant::now();
    let mut source_bytes = 0_usize;
    for path in paths {
        let source =
            std::fs::read(path).map_err(|_| Error::argument("cannot read ", path.as_os_str()))?;
        source_bytes = source_bytes.checked_add(source.len()).ok_or(Error::Limit)?;
        reader::Parser::new(&mut program, &source, path)?.parse()?;
    }
    let parse_ms = start.elapsed().as_secs_f64() * 1000.0;
    let start = std::time::Instant::now();
    program.finish_symbols()?;
    program.legacy_roles();
    validator::validate(&program)?;
    let validate_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut stats = if statistics {
        program.statistics()
    } else {
        Statistics::default()
    };
    stats.source_bytes = source_bytes;
    stats.parse_ms = parse_ms;
    stats.validate_ms = validate_ms;
    let start = std::time::Instant::now();
    write_file(&program, output)?;
    stats.write_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}
/// Construct and validate a PA8 exercise through the shared typed model.
///
/// # Errors
/// Returns invalid exercise, construction, storage or output failures.
pub fn exercise(name: &str, output: &Path, statistics: bool) -> Result<Statistics, Error> {
    let mut program = model::Program::default();
    let start = std::time::Instant::now();
    exercises::build(&mut program, name)?;
    validator::validate_constructed(&program).map_err(|error| match error {
        Error::Diagnostic(message) => Error::Construction(message),
        error => error,
    })?;
    let build_ms = start.elapsed().as_secs_f64() * 1000.0;
    let mut stats = if statistics {
        program.statistics()
    } else {
        Statistics::default()
    };
    stats.build_ms = build_ms;
    let start = std::time::Instant::now();
    write_file(&program, output)?;
    stats.write_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}
fn write_file(program: &model::Program, output: &Path) -> Result<(), Error> {
    let file = std::fs::File::create(output)
        .map_err(|_| Error::argument("cannot write ", output.as_os_str()))?;
    writer::write(program, std::io::BufWriter::new(file))
        .map_err(|_| Error::argument("cannot write ", output.as_os_str()))
}
