// Translated from support/stack_budget.h. See NOTICE.
//! A typed host limit; compiler phases never query process configuration.

#[derive(Clone, Copy, Debug)]
pub enum StackLimit {
    Unknown,
    Unlimited,
    Bytes(u64),
}

#[derive(Clone, Copy, Debug)]
pub struct StackBudget {
    bytes: usize,
}
impl Default for StackBudget {
    fn default() -> Self {
        Self { bytes: 4 << 20 }
    }
}
impl StackBudget {
    /// Select half the soft limit, capped at 256 MiB, or the 4 MiB fallback.
    ///
    /// # Errors
    /// Returns a host-size conversion failure on an unsupported host width.
    pub fn from_limit(limit: StackLimit) -> Result<Self, &'static str> {
        let bytes = match limit {
            StackLimit::Unknown => 4 << 20,
            StackLimit::Unlimited => 256 << 20,
            StackLimit::Bytes(bytes) => (bytes / 2).min(256 << 20),
        };
        Ok(Self {
            bytes: usize::try_from(bytes).map_err(|_| "host stack budget exceeds usize")?,
        })
    }
    #[must_use]
    #[inline]
    pub fn exceeded(self, base: usize, here: usize) -> bool {
        base.abs_diff(here) > self.bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn half_limit_fallback_cap_and_direction_are_explicit() {
        for (limit, expected) in [
            (StackLimit::Unknown, 4 << 20),
            (StackLimit::Unlimited, 256 << 20),
            (StackLimit::Bytes(0), 0),
            (StackLimit::Bytes(3), 1),
            (StackLimit::Bytes(8 << 20), 4 << 20),
            (StackLimit::Bytes(u64::MAX), 256 << 20),
        ] {
            let budget = StackBudget::from_limit(limit).unwrap();
            assert_eq!(budget.bytes, expected);
            assert!(!budget.exceeded(0, expected));
            assert!(budget.exceeded(0, expected + 1));
            assert!(!budget.exceeded(expected, 0));
            assert!(budget.exceeded(expected + 1, 0));
        }
    }
}
