//! Command-line boundary shared by the nine compiler tools.

mod compatibility;
pub mod debug_options;
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
pub mod host_environment;
pub mod lowir;
pub mod lowir2native;

pub mod toolchain;

pub mod lowiropt;
