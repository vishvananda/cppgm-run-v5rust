//! Function-owned range pressure and assigned-register interference. See NOTICE.
use crate::lowir::Error;

pub(super) struct Pressure {
    size: usize,
    maximum: Vec<i32>,
    pending: Vec<i32>,
    cover: Vec<u32>,
    below: Vec<u32>,
}
impl Pressure {
    pub(super) fn new(base: &[i32]) -> Result<Self, Error> {
        let size = base.len().checked_next_power_of_two().ok_or(Error::Limit)?;
        let count = size.checked_mul(2).ok_or(Error::Limit)?;
        let mut out = Self {
            size,
            maximum: vec![0; count],
            pending: vec![0; count],
            cover: vec![0; count],
            below: vec![0; count],
        };
        out.maximum[size..size + base.len()].copy_from_slice(base);
        for i in (1..size).rev() {
            out.maximum[i] = out.maximum[2 * i].max(out.maximum[2 * i + 1]);
        }
        Ok(out)
    }
    pub(super) fn add(&mut self, first: usize, last: usize, delta: i32, registers: u32) {
        let mut left = first + self.size;
        let mut right = last + self.size;
        let (l0, r0) = (left, right);
        while left <= right {
            if left & 1 != 0 {
                self.cover_node(left, delta, registers);
                left += 1;
            }
            if right & 1 == 0 {
                self.cover_node(right, delta, registers);
                right -= 1;
            }
            left >>= 1;
            right >>= 1;
        }
        for start in [l0, r0] {
            let mut node = start >> 1;
            while node != 0 {
                self.maximum[node] =
                    self.maximum[2 * node].max(self.maximum[2 * node + 1]) + self.pending[node];
                self.below[node] =
                    self.below[2 * node] | self.below[2 * node + 1] | self.cover[node];
                node >>= 1;
            }
        }
    }
    fn cover_node(&mut self, node: usize, delta: i32, registers: u32) {
        self.maximum[node] += delta;
        self.pending[node] += delta;
        self.cover[node] |= registers;
        self.below[node] |= registers;
    }
    pub(super) fn query(&self, first: usize, last: usize) -> (i32, u32) {
        self.query_node(1, 0, self.size - 1, first, last)
    }
    fn query_node(
        &self,
        node: usize,
        lo: usize,
        hi: usize,
        first: usize,
        last: usize,
    ) -> (i32, u32) {
        if last < lo || hi < first {
            return (i32::MIN / 2, 0);
        }
        if first <= lo && hi <= last {
            return (self.maximum[node], self.below[node]);
        }
        let mid = lo.midpoint(hi);
        let a = self.query_node(2 * node, lo, mid, first, last);
        let b = self.query_node(2 * node + 1, mid + 1, hi, first, last);
        (
            a.0.max(b.0) + self.pending[node],
            a.1 | b.1 | self.cover[node],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_range_adds_preserve_maximum_and_register_unions() -> Result<(), Error> {
        for length in [1, 7, 32, 65] {
            let base: Vec<_> = (0..length).map(|i| i32::try_from(i % 4).unwrap()).collect();
            let mut tree = Pressure::new(&base)?;
            let mut counts = base;
            let mut masks = vec![0; length];
            for step in 0..24 {
                let first = step * 7 % length;
                let last = (first + step % 11).min(length - 1);
                let delta = if step % 3 == 0 { -1 } else { 1 };
                let register = 1 << (step % 12);
                tree.add(first, last, delta, register);
                for i in first..=last {
                    counts[i] += delta;
                    masks[i] |= register;
                }
                for lo in 0..length {
                    for hi in lo..length {
                        assert_eq!(
                            tree.query(lo, hi),
                            (
                                counts[lo..=hi].iter().copied().max().unwrap(),
                                masks[lo..=hi].iter().fold(0, |a, b| a | b)
                            )
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
