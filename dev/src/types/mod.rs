//! Typed target representation facts shared by compiler phases.
mod fundamental;
pub use fundamental::FundamentalType;

pub mod float_value;
pub mod floating;

pub mod interchange;
