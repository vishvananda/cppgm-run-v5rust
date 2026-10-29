//! Flat compiler tables with inexpensive identity hashing. See NOTICE.
//!
//! The reference's `PointerMap` and `TypeTable` mix integer identities rather
//! than running a keyed string hash for every ID. Keys remain typed and exact
//! equality resolves collisions; iteration order never defines output order.
use std::hash::{BuildHasherDefault, Hasher as Hash};

pub(crate) type BuildHasher = BuildHasherDefault<Hasher>;
pub(crate) type HashMap<K, V> = std::collections::HashMap<K, V, BuildHasher>;
pub(crate) type HashSet<K> = std::collections::HashSet<K, BuildHasher>;

#[derive(Default)]
pub(crate) struct Hasher(u64);
impl Hasher {
    #[inline]
    fn mix(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}
impl Hash for Hasher {
    #[inline]
    fn finish(&self) -> u64 {
        // Spread both low bucket bits and high hashbrown control bits.
        let mut hash = self.0;
        hash ^= hash >> 33;
        hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
        hash ^ (hash >> 33)
    }
    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        while let Some((word, rest)) = bytes.split_first_chunk::<8>() {
            self.mix(u64::from_le_bytes(*word));
            bytes = rest;
        }
        if let Some((word, rest)) = bytes.split_first_chunk::<4>() {
            self.mix(u64::from(u32::from_le_bytes(*word)));
            bytes = rest;
        }
        if let Some((word, rest)) = bytes.split_first_chunk::<2>() {
            self.mix(u64::from(u16::from_le_bytes(*word)));
            bytes = rest;
        }
        if let Some(&byte) = bytes.first() {
            self.mix(u64::from(byte));
        }
    }
    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.mix(u64::from(value));
    }
    #[inline]
    fn write_u16(&mut self, value: u16) {
        self.mix(u64::from(value));
    }
    #[inline]
    fn write_u32(&mut self, value: u32) {
        self.mix(u64::from(value));
    }
    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }
    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{hash::BuildHasher as _, rc::Rc};

    #[test]
    fn borrowed_spellings_share_identity_across_growth_and_removal() {
        let mut names = HashMap::<Rc<[u8]>, usize>::default();
        for i in 0..1024 {
            names.insert(Rc::from(format!("name{i}").into_bytes()), i);
        }
        for i in 0..1024 {
            assert_eq!(names.get(format!("name{i}").as_bytes()), Some(&i));
        }
        names.remove(b"name17".as_slice());
        assert!(!names.contains_key(b"name17".as_slice()));
        assert_eq!(names.get(b"name18".as_slice()), Some(&18));
    }

    #[test]
    fn structural_boundaries_and_high_id_bits_participate() {
        let state = BuildHasher::default();
        assert_ne!(state.hash_one((1u32, 2u32)), state.hash_one((2u32, 1u32)));
        assert_ne!(state.hash_one(1u64), state.hash_one(1u64 << 32));
        assert_ne!(state.hash_one(b"ab"), state.hash_one(b"ab\0"));
        assert_ne!(state.hash_one([1u32, 2u32]), state.hash_one([1u32, 3u32]));
    }
}
