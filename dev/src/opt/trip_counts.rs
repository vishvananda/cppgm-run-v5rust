//! Counted integer loop exit arithmetic from `opt/loop_passes.cpp`. See NOTICE.
use crate::lowir::vocabulary::{Compare, Type};

fn unsigned(pred: Compare) -> bool {
    matches!(
        pred,
        Compare::Ult | Compare::Ule | Compare::Ugt | Compare::Uge
    )
}
fn holds(pred: Compare, continues: bool, value: i128, bound: i128) -> bool {
    let result = match pred {
        Compare::Eq => value == bound,
        Compare::Ne => value != bound,
        Compare::Lt | Compare::Ult => value < bound,
        Compare::Le | Compare::Ule => value <= bound,
        Compare::Gt | Compare::Ugt => value > bound,
        Compare::Ge | Compare::Uge => value >= bound,
    };
    result == continues
}
/// Test number (zero-based) and tested exit value, without wrapping the
/// comparison's range. C++ only counts types of at most 64 bits here.
pub(super) fn count(
    ty: Type,
    pred: Compare,
    continues: bool,
    init: i128,
    step: i64,
    bound: i128,
    next: bool,
) -> Option<(i64, i128)> {
    let bits = ty.bits();
    if bits == 0 || bits > 64 || step == 0 {
        return None;
    }
    let unsigned = unsigned(pred);
    let lo = if unsigned { 0 } else { -(1_i128 << (bits - 1)) };
    let hi = if unsigned {
        (1_i128 << bits) - 1
    } else {
        (1_i128 << (bits - 1)) - 1
    };
    let mask = (1_i128 << bits) - 1;
    let order = |v: i128| {
        let v = v & mask;
        if !unsigned && v > hi { v - mask - 1 } else { v }
    };
    let init = order(init);
    let bound = order(bound);
    let step = i128::from(step);
    let first = init + if next { step } else { 0 };
    let in_range = |v| v >= lo && v <= hi;
    if !in_range(first) {
        return None;
    }
    if !holds(pred, continues, first, bound) {
        return Some((0, first));
    }
    let gap = bound - first;
    let k = if if step > 0 { gap < 0 } else { gap > 0 } {
        1
    } else {
        gap / step
    };
    for probe in (if k > 1 { k - 1 } else { 1 })..=k + 2 {
        let x = first + probe * step;
        if !in_range(x) {
            return None;
        }
        if holds(pred, continues, x, bound) {
            continue;
        }
        for q in 1..probe.min(3) {
            if !holds(pred, continues, first + q * step, bound) {
                return None;
            }
        }
        if probe > 0x7fff_ffff {
            return None;
        }
        return Some((i64::try_from(probe).ok()?, x));
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::lowir::vocabulary::{Compare as C, Type as T};
    #[test]
    fn signedness_belongs_to_the_comparison_and_wrap_is_not_a_count() {
        assert_eq!(
            super::count(T::U8, C::Lt, true, 255, 1, 0, false),
            Some((1, 0))
        );
        assert_eq!(
            super::count(T::U8, C::Ult, true, 255, 1, 0, false),
            Some((0, 255))
        );
        assert_eq!(super::count(T::I8, C::Ne, true, 127, 1, -128, false), None);
        assert_eq!(
            super::count(T::I64, C::Lt, true, 0, 1, 2_147_483_648, false),
            None
        );
        assert_eq!(super::count(T::I128, C::Lt, true, 0, 1, 5, false), None);
    }
}
