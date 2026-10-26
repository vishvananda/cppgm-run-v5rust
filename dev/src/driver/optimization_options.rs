// Translated from cppgm++.cpp and driver/toolchain.cpp::optimization_level. See NOTICE.
//! The source view and ordinary driver retain distinct decimal-spelling rules.

#[derive(Clone, Copy)]
pub enum OptimizationSyntax {
    Driver,
    Emit,
}
impl OptimizationSyntax {
    /// Parse a complete optimization flag, including the frozen level aliases.
    #[must_use]
    pub fn parse(self, argument: &[u8]) -> Option<u8> {
        let level = argument.strip_prefix(b"-O")?;
        match level {
            b"" | b"g" => Some(1),
            b"s" | b"z" => Some(2),
            b"fast" => Some(3),
            _ if !level.is_empty() && level.iter().all(u8::is_ascii_digit) => {
                Some(if matches!(self, Self::Emit) && level.len() > 1 {
                    3
                } else {
                    level
                        .iter()
                        .fold(0_u8, |n, b| n.saturating_mul(10).saturating_add(b - b'0'))
                        .min(3)
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OptimizationSyntax::{Driver, Emit};

    #[test]
    fn aliases_and_decimal_spelling_keep_both_driver_contracts() {
        for (argument, level) in [
            (b"-O".as_slice(), 1),
            (b"-Og", 1),
            (b"-Os", 2),
            (b"-Oz", 2),
            (b"-Ofast", 3),
            (b"-O0", 0),
            (b"-O1", 1),
            (b"-O2", 2),
            (b"-O3", 3),
            (b"-O9", 3),
            (b"-O999999999999999999999999999999999999", 3),
        ] {
            assert_eq!(Driver.parse(argument), Some(level));
            assert_eq!(Emit.parse(argument), Some(level));
        }
        for (argument, driver) in [(b"-O00".as_slice(), 0), (b"-O01", 1), (b"-O0002", 2)] {
            assert_eq!(Driver.parse(argument), Some(driver));
            assert_eq!(Emit.parse(argument), Some(3));
        }
        for argument in [b"-O+1".as_slice(), b"-O-1", b"-O1x", b"-O\xff", b"-g0"] {
            assert_eq!(Driver.parse(argument), None);
            assert_eq!(Emit.parse(argument), None);
        }
    }
}
