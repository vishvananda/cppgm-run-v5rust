//! Preprocessing composes the shared lexer and physical-source owners.
pub use crate::lex::pp_lexer as lexer;
pub use crate::support::source;

pub mod arithmetic;
mod conditionals;
pub mod counters;
pub mod error;
pub mod expression;
pub mod files;
mod inclusion;
mod input;
mod line_control;
mod locations;
pub(crate) use locations::SourcePositions;
mod macros;
pub mod options;
mod pragmas;
pub mod unit;
