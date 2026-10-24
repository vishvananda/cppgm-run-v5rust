//! Typed target representation facts shared by compiler phases.
mod fundamental;
pub use fundamental::FundamentalType;

pub(crate) mod float_value;
pub mod floating;

pub(crate) mod interchange;
