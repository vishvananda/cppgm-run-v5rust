// Translated from cppgm++.cpp::debug_info_request. See NOTICE.
//! The last level request wins; modifiers retain the preceding request.
#[derive(Default)]
pub struct DebugOptions {
    pub level_stated: bool,
    pub locations: bool,
}
impl DebugOptions {
    /// Consume a debug flag, retaining level state across modifier flags.
    pub fn consume(&mut self, argument: &[u8]) -> bool {
        if !argument.starts_with(b"-g") {
            return false;
        }
        self.level_stated = true;
        if matches!(argument, b"-g0" | b"-ggdb0") {
            self.locations = false;
        } else if !(argument.starts_with(b"-gno-")
            || argument.starts_with(b"-gz")
            || argument.starts_with(b"-gstrict-dwarf")
            || matches!(
                argument,
                b"-gcolumn-info" | b"-gpubnames" | b"-gsplit-dwarf" | b"-grecord-gcc-switches"
            ))
        {
            self.locations = true;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::DebugOptions;
    #[test]
    fn modifiers_preserve_the_last_level_and_still_select_the_object_dialect() {
        let mut options = DebugOptions::default();
        assert!(!options.consume(b"-O2"));
        assert!(options.consume(b"-gcolumn-info"));
        assert!(options.level_stated);
        assert!(!options.locations);
        for flag in [b"-g".as_slice(), b"-gno-column-info", b"-gz=zlib"] {
            assert!(options.consume(flag));
            assert!(options.locations);
        }
        assert!(options.consume(b"-ggdb0"));
        assert!(options.consume(b"-gpubnames"));
        assert!(!options.locations);
        assert!(options.consume(b"-gdwarf-5"));
        assert!(options.locations);
        assert!(options.consume(b"-g0"));
        assert!(!options.locations);
    }
}
