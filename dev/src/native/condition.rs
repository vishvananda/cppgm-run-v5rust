//! Typed x86 conditions shared by scalar flags, setcc and conditional moves. See NOTICE.
use crate::lowir::vocabulary::Compare;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Condition {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Below,
    BelowEqual,
    Above,
    AboveEqual,
    Parity,
    NotParity,
}
impl Condition {
    pub(super) fn from_predicate(pred: Compare) -> Self {
        match pred {
            Compare::Eq => Self::Equal,
            Compare::Ne => Self::NotEqual,
            Compare::Lt => Self::Less,
            Compare::Le => Self::LessEqual,
            Compare::Gt => Self::Greater,
            Compare::Ge => Self::GreaterEqual,
            Compare::Ult => Self::Below,
            Compare::Ule => Self::BelowEqual,
            Compare::Ugt => Self::Above,
            Compare::Uge => Self::AboveEqual,
        }
    }
    pub(super) fn code(self) -> u8 {
        match self {
            Self::Equal => 4,
            Self::NotEqual => 5,
            Self::Less => 12,
            Self::LessEqual => 14,
            Self::Greater => 15,
            Self::GreaterEqual => 13,
            Self::Below => 2,
            Self::BelowEqual => 6,
            Self::Above => 7,
            Self::AboveEqual => 3,
            Self::Parity => 10,
            Self::NotParity => 11,
        }
    }
    pub(super) fn spelling(self) -> &'static str {
        match self {
            Self::Equal => "e",
            Self::NotEqual => "ne",
            Self::Less => "l",
            Self::LessEqual => "le",
            Self::Greater => "g",
            Self::GreaterEqual => "ge",
            Self::Below => "b",
            Self::BelowEqual => "be",
            Self::Above => "a",
            Self::AboveEqual => "ae",
            Self::Parity => "p",
            Self::NotParity => "np",
        }
    }
    pub(super) fn invert(self) -> Self {
        match self {
            Self::Equal => Self::NotEqual,
            Self::NotEqual => Self::Equal,
            Self::Less => Self::GreaterEqual,
            Self::LessEqual => Self::Greater,
            Self::Greater => Self::LessEqual,
            Self::GreaterEqual => Self::Less,
            Self::Below => Self::AboveEqual,
            Self::BelowEqual => Self::Above,
            Self::Above => Self::BelowEqual,
            Self::AboveEqual => Self::Below,
            Self::Parity => Self::NotParity,
            Self::NotParity => Self::Parity,
        }
    }
    pub(super) fn swapped(self) -> Self {
        match self {
            Self::Equal | Self::NotEqual | Self::Parity | Self::NotParity => self,
            Self::Less => Self::Greater,
            Self::LessEqual => Self::GreaterEqual,
            Self::Greater => Self::Less,
            Self::GreaterEqual => Self::LessEqual,
            Self::Below => Self::Above,
            Self::BelowEqual => Self::AboveEqual,
            Self::Above => Self::Below,
            Self::AboveEqual => Self::BelowEqual,
        }
    }
}
