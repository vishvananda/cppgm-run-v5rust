//! Internal compiler component; source ownership stays in its phase modules.
pub use cppgm_core::{support, types};
#[doc(hidden)]
pub mod lex;
#[doc(hidden)]
pub mod preprocess;
#[doc(hidden)]
pub mod syntax;
