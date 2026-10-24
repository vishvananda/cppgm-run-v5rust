//! Distinct numeric and text types and the frozen x86-64 literal representation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum FundamentalType {
    Char,
    Char16T,
    Char32T,
    WcharT,
    Int,
    UnsignedInt,
    LongInt,
    UnsignedLongInt,
    LongLongInt,
    UnsignedLongLongInt,
    Int128,
    UnsignedInt128,
    Float,
    Double,
    LongDouble,
    Float16,
    Float32,
    Float64,
    Float128,
    Float32x,
    Float64x,
    GnuFloat128,
    BFloat16,
}

impl FundamentalType {
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Char => "char",
            Self::Char16T => "char16_t",
            Self::Char32T => "char32_t",
            Self::WcharT => "wchar_t",
            Self::Int => "int",
            Self::UnsignedInt => "unsigned int",
            Self::LongInt => "long int",
            Self::UnsignedLongInt => "unsigned long int",
            Self::LongLongInt => "long long int",
            Self::UnsignedLongLongInt => "unsigned long long int",
            Self::Int128 => "__int128",
            Self::UnsignedInt128 => "unsigned __int128",
            Self::Float => "float",
            Self::Double => "double",
            Self::LongDouble => "long double",
            Self::Float16 => "_Float16",
            Self::Float32 => "_Float32",
            Self::Float64 => "_Float64",
            Self::Float128 => "_Float128",
            Self::Float32x => "_Float32x",
            Self::Float64x => "_Float64x",
            Self::GnuFloat128 => "__float128",
            Self::BFloat16 => "__bf16",
        }
    }

    #[must_use]
    pub fn size(self) -> usize {
        match self {
            Self::Char => 1,
            Self::Char16T | Self::Float16 | Self::BFloat16 => 2,
            Self::Char32T
            | Self::WcharT
            | Self::Int
            | Self::UnsignedInt
            | Self::Float
            | Self::Float32 => 4,
            Self::LongInt
            | Self::UnsignedLongInt
            | Self::LongLongInt
            | Self::UnsignedLongLongInt
            | Self::Double
            | Self::Float64
            | Self::Float32x => 8,
            Self::LongDouble
            | Self::Int128
            | Self::UnsignedInt128
            | Self::Float128
            | Self::Float64x
            | Self::GnuFloat128 => 16,
        }
    }

    /// Canonical type identity stays distinct from its target representation.
    #[must_use]
    pub(crate) fn floating_representation(self) -> Self {
        match self {
            Self::Float32 => Self::Float,
            Self::Float64 | Self::Float32x => Self::Double,
            Self::Float64x => Self::LongDouble,
            Self::GnuFloat128 => Self::Float128,
            _ => self,
        }
    }
    #[must_use]
    pub(crate) fn floating_rank(self) -> u8 {
        match self {
            Self::BFloat16 => 1,
            Self::Float16 => 2,
            Self::Float | Self::Float32 => 3,
            Self::Double | Self::Float64 | Self::Float32x => 4,
            Self::LongDouble | Self::Float64x => 5,
            Self::Float128 | Self::GnuFloat128 => 6,
            _ => 0,
        }
    }
    pub(crate) fn floating_precision(self) -> Option<u32> {
        Some(match self.floating_rank() {
            1 => 8,
            2 => 11,
            3 => 24,
            4 => 53,
            5 => 64,
            6 => 113,
            _ => return None,
        })
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
