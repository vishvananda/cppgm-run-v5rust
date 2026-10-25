//! Read the reviewed build-owned host configuration; never probe at runtime.
use crate::builtin_host_configuration as captured;

pub(super) struct Configuration {
    pub macros: Vec<u8>,
    pub strict_macros: Vec<u8>,
    pub includes: Vec<u8>,
}

pub(super) fn query() -> Configuration {
    Configuration {
        macros: captured::MACROS.to_vec(),
        strict_macros: captured::STRICT.to_vec(),
        includes: captured::INCLUDES.to_vec(),
    }
}
