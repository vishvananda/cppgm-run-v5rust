//! Command-line boundary shared by the nine compiler tools.

/// Return the distinct scaffold status until this tool is implemented.
#[must_use]
pub fn run_tool(tool: &str) -> std::process::ExitCode {
    eprintln!("{tool}: Rust port is not implemented yet");
    std::process::ExitCode::from(86)
}

mod file_identity;
pub mod pptoken;

pub mod posttoken;
pub mod ppexpr;
pub mod preproc;

/// Errors retained until a source tool renders its diagnostics.
#[derive(Debug)]
pub enum TokenToolError {
    Internal(&'static str),
    SourceInitialization(crate::preprocess::source::SourceError),
    Source(crate::preprocess::source::SourceError),
    Output(std::io::Error),
    NotImplemented(&'static str),
    Limit(&'static str),
}

pub mod abimangle;
pub mod ast;
mod host_declarations;
pub mod lowir;
