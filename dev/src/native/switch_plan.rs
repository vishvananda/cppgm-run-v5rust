//! Frozen switch clustering, window costs and deferred search ownership. See NOTICE.
use super::{mir::BlockId, scalar_mir::signed, x86::Register};
use crate::lowir::{Error, model::DebugLocation, vocabulary::Type};

#[derive(Clone, Copy)]
pub(super) struct Table {
    pub(super) first: u32,
    pub(super) count: u32,
}
impl Table {
    pub(super) fn range(self) -> std::ops::Range<usize> {
        self.first as usize..self.first as usize + self.count as usize
    }
}
#[derive(Clone, Copy)]
pub(super) struct Test {
    pub(super) lo: i64,
    pub(super) hi: i64,
    pub(super) target: BlockId,
    pub(super) window: bool,
}
pub(super) struct Tree {
    pub(super) selector: Register,
    pub(super) ty: Type,
    pub(super) sign: bool,
    pub(super) fallback: BlockId,
    pub(super) location: Option<DebugLocation>,
    pub(super) tests: Vec<Test>,
    pub(super) masks: Vec<(u64, BlockId)>,
    pub(super) binary_search: bool,
    pub(super) window_first: bool,
}
#[derive(Clone)]
pub(super) struct Node {
    pub(super) tree: usize,
    pub(super) tests: std::ops::Range<usize>,
    pub(super) lo: i64,
    pub(super) hi: i64,
    pub(super) outside: bool,
    pub(super) mir: BlockId,
}
#[derive(Default)]
pub(super) struct Plans {
    pub(super) trees: Vec<Tree>,
    pub(super) nodes: Vec<Node>,
    pub(super) next: u32,
}
impl Plans {
    pub(super) fn defer(&mut self, mut node: Node) -> Result<BlockId, Error> {
        node.mir = BlockId(self.next);
        self.next = self.next.checked_add(1).ok_or(Error::Limit)?;
        let id = node.mir;
        self.nodes.push(node);
        Ok(id)
    }
}
pub(super) fn bounds(ty: Type) -> (i64, i64) {
    match ty {
        Type::I8 => (i8::MIN.into(), i8::MAX.into()),
        Type::U8 => (0, u8::MAX.into()),
        Type::I16 => (i16::MIN.into(), i16::MAX.into()),
        Type::U16 => (0, u16::MAX.into()),
        Type::I32 => (i32::MIN.into(), i32::MAX.into()),
        Type::U32 => (0, u32::MAX.into()),
        _ => (i64::MIN, i64::MAX),
    }
}
pub(super) fn clusters(cases: &mut [(i64, BlockId)]) -> (Vec<Test>, usize) {
    // Stable ordering retains the first target for duplicate values.
    cases.sort_by_key(|&(value, _)| value);
    let mut clusters: Vec<Test> = Vec::new();
    let mut targets = Vec::new();
    for &(value, target) in cases.iter() {
        if let Some(last) = clusters.last_mut() {
            if last.hi == value {
                continue;
            }
            if last.target == target && last.hi.checked_add(1) == Some(value) {
                last.hi = value;
            } else {
                clusters.push(Test {
                    lo: value,
                    hi: value,
                    target,
                    window: false,
                });
            }
        } else {
            clusters.push(Test {
                lo: value,
                hi: value,
                target,
                window: false,
            });
        }
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    (clusters, targets.len())
}
pub(super) fn table_size(cases: &[(i64, BlockId)], targets: usize) -> Option<usize> {
    let low = cases.first()?.0;
    let span = cases
        .last()?
        .0
        .cast_unsigned()
        .wrapping_sub(low.cast_unsigned());
    (targets > 5
        && cases.len() >= 5
        && span < 4096
        && u128::from(span) < 10 * cases.len() as u128
        && i32::try_from(low).is_ok())
    .then(|| usize::try_from(span + 1).ok())
    .flatten()
}
// At most 64 distinct values fit a candidate window. Preserve the frozen
// tie break by covered values after instruction savings.
fn window(clusters: &[Test], ty: Type) -> (std::ops::Range<usize>, i64) {
    let (tmin, tmax) = bounds(ty);
    let mut range = clusters.len()..clusters.len();
    let mut best = 0;
    let mut best_cover = 0;
    for (i, first) in clusters.iter().enumerate() {
        let mut seen = Vec::new();
        let mut own = 0;
        let mut cover = 0;
        for (k, c) in clusters.iter().enumerate().skip(i) {
            if i128::from(c.hi) - i128::from(first.lo) >= 64 {
                break;
            }
            if !seen.contains(&c.target) {
                if seen.len() == 5 {
                    break;
                }
                seen.push(c.target);
            }
            own += if c.lo == c.hi || c.lo == tmin || c.hi == tmax {
                2
            } else {
                4
            };
            cover += c.hi - c.lo + 1;
            let saved = own - (4 + seen.iter().map(|_| 3).sum::<i32>());
            if k + 1 - i >= 3 && (saved > best || saved == best && saved > 0 && cover > best_cover)
            {
                best = saved;
                best_cover = cover;
                range = i..k + 1;
            }
        }
    }
    (range, best_cover)
}
pub(super) fn tree(
    clusters: &[Test],
    selector: Register,
    ty: Type,
    fallback: BlockId,
    level: u8,
    location: Option<DebugLocation>,
) -> Option<Tree> {
    let (window, cover) = window(clusters, ty);
    if clusters.iter().any(|c| {
        i32::try_from(c.lo).is_err()
            || i32::try_from(c.hi).is_err()
            || i32::try_from(i128::from(c.hi) - i128::from(c.lo)).is_err()
    }) {
        return None;
    }
    let mut tests = Vec::new();
    let mut masks: Vec<(u64, BlockId)> = Vec::new();
    for (i, &c) in clusters.iter().enumerate() {
        if window.contains(&i) {
            if i == window.start {
                tests.push(Test {
                    lo: c.lo,
                    hi: clusters[window.end - 1].hi,
                    target: fallback,
                    window: true,
                });
            }
            let width = u32::try_from(c.hi - c.lo + 1).ok()?;
            let shift = u32::try_from(c.lo - clusters[window.start].lo).ok()?;
            let bits = (if width == 64 {
                u64::MAX
            } else {
                (1_u64 << width) - 1
            }) << shift;
            if let Some(mask) = masks.iter_mut().find(|m| m.1 == c.target) {
                mask.0 |= bits;
            } else {
                masks.push((bits, c.target));
            }
        } else {
            tests.push(c);
        }
    }
    let persistent = selector.code() < 16 && selector != Register::R10 && selector != Register::R11;
    let gaps = tests
        .windows(2)
        .filter(|pair| pair[1].lo > pair[0].hi + 1)
        .count();
    let tree = level >= 2 && tests.len() >= 4 && gaps >= 2 && persistent;
    let outside: i64 = tests
        .iter()
        .filter(|c| !c.window)
        .map(|c| c.hi - c.lo + 1)
        .sum();
    Some(Tree {
        selector,
        ty,
        fallback,
        location,
        tests,
        masks,
        sign: signed(ty),
        binary_search: tree,
        window_first: !tree && !window.is_empty() && outside != 0 && cover > outside && persistent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_precedence_and_signed_extremes_survive_clustering() {
        let mut cases = [
            (i64::MAX, BlockId(1)),
            (2, BlockId(2)),
            (1, BlockId(2)),
            (2, BlockId(3)),
            (i64::MIN, BlockId(1)),
        ];
        let (clusters, targets) = clusters(&mut cases);
        assert_eq!(targets, 2);
        assert_eq!(clusters.len(), 3);
        assert_eq!((clusters[1].lo, clusters[1].hi), (1, 2));
        assert_eq!(clusters[1].target.0, 2);
        assert!(tree(&clusters, Register::RDI, Type::I64, BlockId(0), 2, None).is_none());
    }
    #[test]
    fn table_limits_count_original_cases_and_require_more_than_five_targets() {
        let cases = (0..6)
            .map(|i| (i * 10, BlockId(u32::try_from(i).unwrap())))
            .collect::<Vec<_>>();
        assert_eq!(table_size(&cases, 6), Some(51));
        assert_eq!(table_size(&cases, 5), None);
        let sparse = [
            (0, BlockId(0)),
            (60, BlockId(1)),
            (61, BlockId(2)),
            (62, BlockId(3)),
            (63, BlockId(4)),
            (64, BlockId(5)),
        ];
        assert_eq!(table_size(&sparse, 6), None);
    }
    #[test]
    fn window_cover_breaks_equal_cost_ties_and_keeps_mask_order() {
        let clusters = (0..8)
            .map(|i| Test {
                lo: i * 2,
                hi: i * 2,
                target: BlockId(if i % 2 == 0 { 2 } else { 1 }),
                window: false,
            })
            .collect::<Vec<_>>();
        let t = tree(&clusters, Register::RDI, Type::I64, BlockId(0), 2, None).unwrap();
        assert_eq!(t.tests.len(), 1);
        assert!(t.tests[0].window);
        assert_eq!(t.tests[0].hi, 14);
        assert_eq!(
            t.masks
                .iter()
                .map(|&(bits, id)| (bits, id.0))
                .collect::<Vec<_>>(),
            [(0x1111, 2), (0x4444, 1)]
        );
    }
}
