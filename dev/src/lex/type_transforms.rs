// Translated from lex/gnu_builtins.h and gnu_builtins.cpp. See NOTICE.
//! C++11 type-transform vocabulary shared by parsing, semantics and ABI views.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TypeTransform {
    UnderlyingType,
    AddLvalueReference,
    AddPointer,
    AddRvalueReference,
    Decay,
    MakeSigned,
    MakeUnsigned,
    RemoveAllExtents,
    RemoveConst,
    RemoveCv,
    RemoveCvref,
    RemoveExtent,
    RemovePointer,
    RemoveReference,
    RemoveReferenceGnu,
    RemoveRestrict,
    RemoveVolatile,
    AddAtomic,
}

impl TypeTransform {
    const SPELLINGS: [(Self, &'static str); 18] = [
        (Self::UnderlyingType, "__underlying_type"),
        (Self::AddLvalueReference, "__add_lvalue_reference"),
        (Self::AddPointer, "__add_pointer"),
        (Self::AddRvalueReference, "__add_rvalue_reference"),
        (Self::Decay, "__decay"),
        (Self::MakeSigned, "__make_signed"),
        (Self::MakeUnsigned, "__make_unsigned"),
        (Self::RemoveAllExtents, "__remove_all_extents"),
        (Self::RemoveConst, "__remove_const"),
        (Self::RemoveCv, "__remove_cv"),
        (Self::RemoveCvref, "__remove_cvref"),
        (Self::RemoveExtent, "__remove_extent"),
        (Self::RemovePointer, "__remove_pointer"),
        (Self::RemoveReference, "__remove_reference_t"),
        (Self::RemoveReferenceGnu, "__remove_reference"),
        (Self::RemoveRestrict, "__remove_restrict"),
        (Self::RemoveVolatile, "__remove_volatile"),
        (Self::AddAtomic, "_Atomic"),
    ];

    #[inline]
    #[must_use]
    pub fn find(spelling: &[u8]) -> Option<Self> {
        Self::SPELLINGS
            .iter()
            .find_map(|&(kind, name)| (name.as_bytes() == spelling).then_some(kind))
    }

    #[inline]
    #[must_use]
    pub fn spelling(self) -> &'static str {
        Self::SPELLINGS[self as usize].1
    }
}
