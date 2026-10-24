// Translated from lex/gnu_builtins.cpp and sema/function_records.h. See NOTICE.
//! Reserved atomic spellings and compact immutable call semantics.
use super::Error;
use crate::{lex::identifiers::NameId, syntax::cursor::Cursor};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicArithmetic {
    Add,
    Sub,
    And,
    Or,
    Xor,
    Nand,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AtomicOperation {
    Load(bool),
    Store(bool),
    Exchange(bool),
    CompareExchange(bool),
    Update(AtomicArithmetic, bool),
    TestAndSet,
    Clear,
    ThreadFence,
    SignalFence,
    BoolCompareSwap,
    ValCompareSwap,
    LockFree,
}
impl AtomicOperation {
    pub(super) fn generic(self) -> bool {
        matches!(
            self,
            Self::Load(true)
                | Self::Store(true)
                | Self::Exchange(true)
                | Self::CompareExchange(true)
        )
    }
    pub(super) fn loads(self) -> bool {
        matches!(self, Self::Load(_))
    }
    pub(super) fn byte(self) -> bool {
        matches!(self, Self::TestAndSet | Self::Clear)
    }
}
#[derive(Clone, Copy, Debug)]
pub(crate) enum AtomicOrder {
    Relaxed,
    Acquire,
    Release,
    SequentiallyConsistent,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct AtomicIntrinsic {
    pub operation: AtomicOperation,
    pub implied: Option<AtomicOrder>,
    pub c11: bool,
    pub weak: Option<bool>,
}
#[derive(Clone, Copy)]
pub(super) enum AtomicOperand {
    Pointer,
    Reference,
    Value,
    Bool,
    Order,
    Size,
}
#[derive(Clone, Copy)]
pub(super) enum AtomicResult {
    Object,
    Void,
    Bool,
    Constant,
}
#[derive(Clone, Copy)]
pub(super) struct AtomicForm {
    pub intrinsic: AtomicIntrinsic,
    pub roles: &'static [AtomicOperand],
    pub result: AtomicResult,
    pub trailing: bool,
}

// These IDs address immutable compiler vocabulary, so entity facts may retain
// them after the analyzer's unit-specific NameId index has been released.
#[derive(Clone, Copy)]
pub(crate) struct AtomicBuiltinId(u8);
#[derive(Clone)]
pub(super) struct AtomicBuiltins(Vec<(NameId, AtomicBuiltinId)>);
impl AtomicBuiltins {
    pub(super) fn find(&self, name: NameId) -> Option<AtomicBuiltinId> {
        self.0
            .iter()
            .find_map(|&(key, id)| (key == name).then_some(id))
    }
}
impl AtomicBuiltinId {
    pub(super) fn form(self) -> Result<AtomicForm, Error> {
        let mut index = usize::from(self.0);
        for &(prefix, entries, implied, trailing) in ATOMIC_FAMILIES {
            if let Some(&(suffix, operation, roles, result)) = entries.get(index) {
                let implied = match suffix {
                    "init" => Some(AtomicOrder::Relaxed),
                    "lock_test_and_set" => Some(AtomicOrder::Acquire),
                    "lock_release" => Some(AtomicOrder::Release),
                    _ => implied,
                };
                return Ok(AtomicForm {
                    intrinsic: AtomicIntrinsic {
                        operation,
                        implied,
                        c11: prefix == "__c11_atomic_",
                        weak: match suffix {
                            "compare_exchange_strong" => Some(false),
                            "compare_exchange_weak" => Some(true),
                            _ => None,
                        },
                    },
                    roles,
                    result,
                    trailing,
                });
            }
            index -= entries.len();
        }
        Err(Error::Invariant("atomic builtin identity outside registry"))
    }
}

use AtomicArithmetic as A;
use AtomicOperand::{Bool as B, Order as O, Pointer as P, Reference as R, Size as S, Value as V};
use AtomicOperation as K;
use AtomicResult::{Bool as Z, Constant as C, Object as T, Void as N};
type AtomicEntry = (
    &'static str,
    AtomicOperation,
    &'static [AtomicOperand],
    AtomicResult,
);
type AtomicFamily = (
    &'static str,
    &'static [AtomicEntry],
    Option<AtomicOrder>,
    bool,
);
const ATOMIC_FAMILIES: &[AtomicFamily] = &[
    (
        "__c11_atomic_",
        &[
            ("init", K::Store(false), &[P, V], N),
            ("load", K::Load(false), &[P, O], T),
            ("store", K::Store(false), &[P, V, O], N),
            ("exchange", K::Exchange(false), &[P, V, O], T),
            (
                "compare_exchange_strong",
                K::CompareExchange(false),
                &[P, R, V, O, O],
                Z,
            ),
            (
                "compare_exchange_weak",
                K::CompareExchange(false),
                &[P, R, V, O, O],
                Z,
            ),
            ("fetch_add", K::Update(A::Add, true), &[P, V, O], T),
            ("fetch_sub", K::Update(A::Sub, true), &[P, V, O], T),
            ("fetch_and", K::Update(A::And, true), &[P, V, O], T),
            ("fetch_or", K::Update(A::Or, true), &[P, V, O], T),
            ("fetch_xor", K::Update(A::Xor, true), &[P, V, O], T),
            ("fetch_nand", K::Update(A::Nand, true), &[P, V, O], T),
            ("thread_fence", K::ThreadFence, &[O], N),
            ("signal_fence", K::SignalFence, &[O], N),
            ("is_lock_free", K::LockFree, &[S], C),
        ],
        None,
        false,
    ),
    (
        "__atomic_",
        &[
            ("load_n", K::Load(false), &[P, O], T),
            ("load", K::Load(true), &[P, R, O], N),
            ("store_n", K::Store(false), &[P, V, O], N),
            ("store", K::Store(true), &[P, R, O], N),
            ("exchange_n", K::Exchange(false), &[P, V, O], T),
            ("exchange", K::Exchange(true), &[P, R, R, O], N),
            (
                "compare_exchange_n",
                K::CompareExchange(false),
                &[P, R, V, B, O, O],
                Z,
            ),
            (
                "compare_exchange",
                K::CompareExchange(true),
                &[P, R, R, B, O, O],
                Z,
            ),
            ("add_fetch", K::Update(A::Add, false), &[P, V, O], T),
            ("sub_fetch", K::Update(A::Sub, false), &[P, V, O], T),
            ("and_fetch", K::Update(A::And, false), &[P, V, O], T),
            ("or_fetch", K::Update(A::Or, false), &[P, V, O], T),
            ("xor_fetch", K::Update(A::Xor, false), &[P, V, O], T),
            ("nand_fetch", K::Update(A::Nand, false), &[P, V, O], T),
            ("fetch_add", K::Update(A::Add, true), &[P, V, O], T),
            ("fetch_sub", K::Update(A::Sub, true), &[P, V, O], T),
            ("fetch_and", K::Update(A::And, true), &[P, V, O], T),
            ("fetch_or", K::Update(A::Or, true), &[P, V, O], T),
            ("fetch_xor", K::Update(A::Xor, true), &[P, V, O], T),
            ("fetch_nand", K::Update(A::Nand, true), &[P, V, O], T),
            ("test_and_set", K::TestAndSet, &[P, O], Z),
            ("clear", K::Clear, &[P, O], N),
            ("thread_fence", K::ThreadFence, &[O], N),
            ("signal_fence", K::SignalFence, &[O], N),
            ("always_lock_free", K::LockFree, &[S, R], C),
            ("is_lock_free", K::LockFree, &[S, R], C),
        ],
        None,
        false,
    ),
    (
        "__sync_",
        &[
            ("fetch_and_add", K::Update(A::Add, true), &[P, V], T),
            ("fetch_and_sub", K::Update(A::Sub, true), &[P, V], T),
            ("fetch_and_and", K::Update(A::And, true), &[P, V], T),
            ("fetch_and_or", K::Update(A::Or, true), &[P, V], T),
            ("fetch_and_xor", K::Update(A::Xor, true), &[P, V], T),
            ("fetch_and_nand", K::Update(A::Nand, true), &[P, V], T),
            ("add_and_fetch", K::Update(A::Add, false), &[P, V], T),
            ("sub_and_fetch", K::Update(A::Sub, false), &[P, V], T),
            ("and_and_fetch", K::Update(A::And, false), &[P, V], T),
            ("or_and_fetch", K::Update(A::Or, false), &[P, V], T),
            ("xor_and_fetch", K::Update(A::Xor, false), &[P, V], T),
            ("nand_and_fetch", K::Update(A::Nand, false), &[P, V], T),
            ("bool_compare_and_swap", K::BoolCompareSwap, &[P, V, V], Z),
            ("val_compare_and_swap", K::ValCompareSwap, &[P, V, V], T),
            ("lock_test_and_set", K::Exchange(false), &[P, V], T),
            ("lock_release", K::Store(false), &[P], N),
            ("synchronize", K::ThreadFence, &[], N),
        ],
        Some(AtomicOrder::SequentiallyConsistent),
        true,
    ),
];

pub(super) fn names(cursor: &mut Cursor<'_>) -> Result<AtomicBuiltins, Error> {
    let mut result = Vec::new();
    for &(prefix, entries, _, _) in ATOMIC_FAMILIES {
        for &(suffix, _, _, _) in entries {
            let id = AtomicBuiltinId(
                u8::try_from(result.len()).map_err(|_| Error::Limit("atomic registry limit"))?,
            );
            let name = format!("{prefix}{suffix}");
            let name = cursor
                .intern_name(name.as_bytes())
                .map_err(|_| Error::Limit("semantic identifier limit"))?;
            result.push((name, id));
        }
    }
    Ok(AtomicBuiltins(result))
}
