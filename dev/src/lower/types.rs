// Translated from lower/symbol_lowering.cpp::storage_type. See NOTICE.
use super::Error;
use crate::{
    lowir::vocabulary::Type,
    sema::{
        Graph,
        lowering::{Scalar, TypeId, TypeView},
    },
    types::FundamentalType as F,
};
pub(super) fn storage(graph: &Graph, ty: TypeId) -> Result<Type, Error> {
    Ok(match graph.lower_type(ty)? {
        TypeView::Scalar(scalar) => match scalar {
            Scalar::Void => Type::Void,
            Scalar::Bool | Scalar::UnsignedChar => Type::U8,
            Scalar::SignedChar | Scalar::Basic(F::Char) => Type::I8,
            Scalar::Short => Type::I16,
            Scalar::UnsignedShort | Scalar::Basic(F::Char16T) => Type::U16,
            Scalar::Basic(F::Int | F::WcharT) => Type::I32,
            Scalar::Basic(F::UnsignedInt | F::Char32T) => Type::U32,
            Scalar::Basic(
                F::LongInt | F::UnsignedLongInt | F::LongLongInt | F::UnsignedLongLongInt,
            ) => Type::I64,
            Scalar::Basic(F::Float) => Type::F32,
            Scalar::Basic(F::Double) => Type::F64,
            Scalar::Basic(F::LongDouble) => Type::F80,
            Scalar::Nullptr => Type::Ptr,
        },
        TypeView::Pointer(_) | TypeView::Reference | TypeView::Function { .. } => Type::Ptr,
        TypeView::Enum(underlying) => return storage(graph, underlying),
        TypeView::Unsupported => return Err(Error::Missing("aggregate storage lowering")),
    })
}
pub(super) fn signed(graph: &Graph, ty: TypeId) -> Result<bool, Error> {
    Ok(match graph.lower_type(ty)? {
        TypeView::Enum(underlying) => return signed(graph, underlying),
        TypeView::Scalar(
            Scalar::SignedChar
            | Scalar::Short
            | Scalar::Basic(F::Char | F::WcharT | F::Int | F::LongInt | F::LongLongInt),
        ) => true,
        _ => false,
    })
}
pub(super) fn boolean(graph: &Graph, ty: TypeId) -> Result<bool, Error> {
    Ok(matches!(
        graph.lower_type(ty)?,
        TypeView::Scalar(Scalar::Bool)
    ))
}
pub(super) fn memory(graph: &Graph, ty: TypeId) -> Result<crate::lowir::model::Facts, Error> {
    let ty_ir = storage(graph, ty)?;
    let volatile = graph.lower_cv(ty) & 2 != 0;
    Ok(crate::lowir::model::Facts::memory(
        volatile,
        !volatile && !matches!(ty_ir, Type::I8 | Type::U8),
    ))
}
