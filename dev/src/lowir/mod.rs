//! Reusable `LowIR` reader, function-owned model, validator and writer. See NOTICE.
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};
pub(crate) mod builder;
mod construction_names;
mod exercises;
mod globals;
mod instructions;
mod lexer;
pub(crate) mod literals;
pub(crate) mod model;
mod reader;
mod runtime;
mod statistics;
pub use statistics::Statistics;
pub(crate) mod validator;
#[cfg(test)]
pub(crate) fn test_program(source: &[u8]) -> Result<model::Program, Error> {
    test_program_dialect(source, false)
}
#[cfg(test)]
pub(crate) fn test_program_dialect(
    source: &[u8],
    object_facts: bool,
) -> Result<model::Program, Error> {
    let mut program = model::Program::default();
    let mut parser = reader::Parser::new(&mut program, source, Path::new("optimizer-test"))?;
    parser.object_facts = object_facts;
    parser.program.object_facts |= object_facts;
    parser.parse()?;
    program.finish_symbols()?;
    program.legacy_roles();
    validator::validate(&program)?;
    Ok(program)
}
#[cfg(test)]
pub(crate) fn validate_test_program(program: &model::Program) -> Result<(), Error> {
    validator::validate_constructed(program)
}
pub(crate) mod vocabulary;
pub(crate) mod writer;

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
    let (program, mut stats) = read_files(paths, statistics)?;
    let start = std::time::Instant::now();
    write_file(&program, output)?;
    stats.write_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(stats)
}
/// The text-tool boundary produces one validated unit, without a text roundtrip
/// between the reader and its native or serialization consumer.
pub(crate) fn read_files(
    paths: &[PathBuf],
    statistics: bool,
) -> Result<(model::Program, Statistics), Error> {
    read_program(paths, statistics, false)
}
/// The object driver's explicit text adapter accepts host ABI facts; course
/// tools continue to reject host-only class spellings.
pub(crate) fn read_object_file(path: &Path) -> Result<model::Program, Error> {
    read_program(&[path.to_path_buf()], false, true).map(|(program, _)| program)
}
fn read_program(
    paths: &[PathBuf],
    statistics: bool,
    object_facts: bool,
) -> Result<(model::Program, Statistics), Error> {
    let mut program = model::Program::default();
    let start = std::time::Instant::now();
    let mut source_bytes = 0_usize;
    for path in paths {
        let source =
            std::fs::read(path).map_err(|_| Error::argument("cannot read ", path.as_os_str()))?;
        source_bytes = source_bytes.checked_add(source.len()).ok_or(Error::Limit)?;
        let mut parser = reader::Parser::new(&mut program, &source, path)?;
        parser.object_facts = object_facts;
        parser.program.object_facts |= object_facts;
        parser.parse()?;
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
    Ok((program, stats))
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
pub(crate) fn write_file(program: &model::Program, output: &Path) -> Result<(), Error> {
    let file = std::fs::File::create(output)
        .map_err(|_| Error::argument("cannot write ", output.as_os_str()))?;
    writer::write(program, std::io::BufWriter::new(file))
        .map_err(|_| Error::argument("cannot write ", output.as_os_str()))
}
