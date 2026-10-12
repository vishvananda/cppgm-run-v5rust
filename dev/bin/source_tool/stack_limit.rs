use cppgm_rust::support::stack::StackLimit;

// Linux exposes the same soft RLIMIT_STACK value through procfs. Reading it
// here avoids authored FFI and keeps host configuration out of compiler phases.
pub(super) fn query() -> StackLimit {
    std::fs::read_to_string("/proc/self/limits").map_or(StackLimit::Unknown, |text| parse(&text))
}
fn parse(text: &str) -> StackLimit {
    let Some(value) = text.lines().find_map(|line| {
        line.strip_prefix("Max stack size")?
            .split_whitespace()
            .next()
    }) else {
        return StackLimit::Unknown;
    };
    if value == "unlimited" {
        StackLimit::Unlimited
    } else {
        value.parse().map_or(StackLimit::Unknown, StackLimit::Bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proc_soft_limit_is_distinct_from_hard_limit_and_missing_data() {
        assert!(matches!(
            parse("Max stack size 8388608 unlimited bytes"),
            StackLimit::Bytes(8_388_608)
        ));
        assert!(matches!(
            parse("Max stack size unlimited unlimited bytes"),
            StackLimit::Unlimited
        ));
        assert!(matches!(
            parse("Max stack size malformed unlimited bytes"),
            StackLimit::Unknown
        ));
        assert!(matches!(
            parse("Max data size 10 100 bytes"),
            StackLimit::Unknown
        ));
    }
}
