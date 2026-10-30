//! Internal compiler component; source ownership stays in its phase modules.
pub use cppgm_core::{abi, support, types};
pub use cppgm_ir::lowir;
#[doc(hidden)]
pub mod lower;
pub use cppgm_sema::sema;
pub use cppgm_syntax::{lex, preprocess, syntax};
