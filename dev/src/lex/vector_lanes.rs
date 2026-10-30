//! GCC vector lane builtin vocabulary from `lex/gnu_builtins.cpp`. See NOTICE.
use crate::types::FundamentalType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Extract,
    Initialize,
    Insert,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lane {
    Basic(FundamentalType),
    SignedChar,
    Short,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Operation {
    pub kind: Kind,
    pub lane: Lane,
    pub lanes: u8,
}

/// Index into the immutable operation vocabulary, checked before publication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationId(u8);
impl OperationId {
    #[inline]
    #[must_use]
    pub fn new(index: usize) -> Option<Self> {
        OPERATIONS.get(index)?;
        Some(Self(u8::try_from(index).ok()?))
    }
    #[inline]
    #[must_use]
    pub fn operation(self) -> Operation {
        OPERATIONS[usize::from(self.0)].1
    }
}
use FundamentalType::{Char, Double, Float, Int, LongLongInt};
use Kind::{Extract, Initialize, Insert};
use Lane::{Basic, Short, SignedChar};

pub const OPERATIONS: &[(&[u8], Operation)] = &[
    (
        b"__builtin_ia32_vec_ext_v16qi",
        Operation {
            kind: Extract,
            lane: Basic(Char),
            lanes: 16,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v2df",
        Operation {
            kind: Extract,
            lane: Basic(Double),
            lanes: 2,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v2di",
        Operation {
            kind: Extract,
            lane: Basic(LongLongInt),
            lanes: 2,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v2si",
        Operation {
            kind: Extract,
            lane: Basic(Int),
            lanes: 2,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v4hi",
        Operation {
            kind: Extract,
            lane: Short,
            lanes: 4,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v4sf",
        Operation {
            kind: Extract,
            lane: Basic(Float),
            lanes: 4,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v4si",
        Operation {
            kind: Extract,
            lane: Basic(Int),
            lanes: 4,
        },
    ),
    (
        b"__builtin_ia32_vec_ext_v8hi",
        Operation {
            kind: Extract,
            lane: Short,
            lanes: 8,
        },
    ),
    (
        b"__builtin_ia32_vec_init_v2si",
        Operation {
            kind: Initialize,
            lane: Basic(Int),
            lanes: 2,
        },
    ),
    (
        b"__builtin_ia32_vec_init_v4hi",
        Operation {
            kind: Initialize,
            lane: Short,
            lanes: 4,
        },
    ),
    (
        b"__builtin_ia32_vec_init_v8qi",
        Operation {
            kind: Initialize,
            lane: SignedChar,
            lanes: 8,
        },
    ),
    (
        b"__builtin_ia32_vec_set_v4hi",
        Operation {
            kind: Insert,
            lane: Short,
            lanes: 4,
        },
    ),
    (
        b"__builtin_ia32_vec_set_v8hi",
        Operation {
            kind: Insert,
            lane: Short,
            lanes: 8,
        },
    ),
];

#[inline]
#[must_use]
pub fn contains(name: &[u8]) -> bool {
    OPERATIONS
        .binary_search_by_key(&name, |entry| entry.0)
        .is_ok()
}
