//! Internal compiler component; source ownership stays in its phase modules.
pub use cppgm_core::{abi, support, types};
pub use cppgm_syntax::{lex, preprocess, syntax};
#[doc(hidden)]
pub mod sema;
