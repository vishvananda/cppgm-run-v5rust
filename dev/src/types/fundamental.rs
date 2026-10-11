//! Distinct numeric types and the frozen x86-64 literal representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FundamentalType {
    Int,
    UnsignedInt,
    LongInt,
    UnsignedLongInt,
    LongLongInt,
    UnsignedLongLongInt,
    Float,
    Double,
    LongDouble,
}

impl FundamentalType {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::UnsignedInt => "unsigned int",
            Self::LongInt => "long int",
            Self::UnsignedLongInt => "unsigned long int",
            Self::LongLongInt => "long long int",
            Self::UnsignedLongLongInt => "unsigned long long int",
            Self::Float => "float",
            Self::Double => "double",
            Self::LongDouble => "long double",
        }
    }

    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::Int | Self::UnsignedInt | Self::Float => 4,
            Self::LongInt
            | Self::UnsignedLongInt
            | Self::LongLongInt
            | Self::UnsignedLongLongInt
            | Self::Double => 8,
            Self::LongDouble => 16,
        }
    }

    pub(crate) fn integer_max(self) -> u64 {
        match self {
            Self::Int => 0x7FFF_FFFF,
            Self::UnsignedInt => 0xFFFF_FFFF,
            Self::LongInt | Self::LongLongInt => 0x7FFF_FFFF_FFFF_FFFF,
            _ => u64::MAX,
        }
    }
}
