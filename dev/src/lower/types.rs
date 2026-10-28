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
pub(super) fn abi_scalar(scalar: Scalar) -> crate::abi::vocabulary::Builtin {
    use crate::abi::vocabulary::Builtin as B;
    match scalar {
        Scalar::Void => B::Void,
        Scalar::Bool => B::Bool,
        Scalar::SignedChar => B::SignedChar,
        Scalar::UnsignedChar => B::UnsignedChar,
        Scalar::Short => B::Short,
        Scalar::UnsignedShort => B::UnsignedShort,
        Scalar::Nullptr => B::Nullptr,
        Scalar::Basic(f) => match f {
            F::Char => B::Char,
            F::Char16T => B::Char16,
            F::Char32T => B::Char32,
            F::WcharT => B::Wchar,
            F::Int => B::Int,
            F::UnsignedInt => B::UnsignedInt,
            F::LongInt => B::Long,
            F::UnsignedLongInt => B::UnsignedLong,
            F::LongLongInt => B::LongLong,
            F::UnsignedLongLongInt => B::UnsignedLongLong,
            F::Int128 => B::Int128,
            F::UnsignedInt128 => B::Uint128,
            F::Float16 => B::Float16,
            F::BFloat16 => B::Bfloat16,
            F::Float32 => B::Float32,
            F::Float64 => B::Float64,
            F::Float32x => B::Float32X,
            F::Float64x => B::Float64X,
            F::Float128 => B::StdFloat128,
            F::GnuFloat128 => B::Float128,
            F::Float => B::Float,
            F::Double => B::Double,
            F::LongDouble => B::LongDouble,
        },
    }
}
#[derive(Clone, Copy)]
pub(super) struct ClassStorage {
    pub changeable: bool,
    pub destruction: bool,
    pub value_zero: bool,
}
// Source layout and encoded LowIR storage have distinct widths. Keep raw
// sizes here so pointer strides, ABI extents and copy counts remain u64.
enum Storage {
    Scalar(Type),
    Object { bytes: u64, align: u8 },
}
pub(super) fn storage(graph: &Graph, ty: TypeId) -> Result<Type, Error> {
    Ok(match storage_layout(graph, ty)? {
        Storage::Scalar(ty) => ty,
        Storage::Object { bytes, align } => Type::object(bytes, align),
    })
}
fn storage_layout(graph: &Graph, ty: TypeId) -> Result<Storage, Error> {
    if let Some((_, element)) = graph.lower_member_pointer_type(ty) {
        return Ok(Storage::Scalar(
            if matches!(graph.lower_type(element)?, TypeView::Function { .. }) {
                Type::I128
            } else {
                Type::I64
            },
        ));
    }
    Ok(match graph.lower_type(ty)? {
        TypeView::Vector { element, bound } => {
            if bound == 0 {
                return Err(Error::Missing("dependent vector storage layout"));
            }
            let bytes = if matches!(graph.lower_type(element)?, TypeView::Scalar(Scalar::Bool)) {
                bound.div_ceil(8)
            } else {
                layout(graph, element)?
                    .0
                    .checked_mul(bound)
                    .ok_or(crate::lowir::Error::Limit)?
            }
            .checked_next_power_of_two()
            .ok_or(crate::lowir::Error::Limit)?;
            Storage::Object {
                bytes,
                align: u8::try_from(bytes.trailing_zeros())
                    .map_err(|_| crate::lowir::Error::Limit)?,
            }
        }
        TypeView::Complex(part) => {
            let (bytes, align) = layout(graph, part)?;
            Storage::Object {
                bytes: bytes.checked_mul(2).ok_or(crate::lowir::Error::Limit)?,
                align,
            }
        }
        TypeView::Scalar(scalar) => Storage::Scalar(match scalar {
            Scalar::Void => Type::Void,
            Scalar::Bool | Scalar::UnsignedChar => Type::U8,
            Scalar::SignedChar | Scalar::Basic(F::Char) => Type::I8,
            Scalar::Short => Type::I16,
            Scalar::UnsignedShort | Scalar::Basic(F::Char16T) => Type::U16,
            Scalar::Basic(F::Int | F::WcharT) => Type::I32,
            Scalar::Basic(F::UnsignedInt | F::Char32T) => Type::U32,
            Scalar::Basic(
                F::LongInt | F::UnsignedLongInt | F::LongLongInt | F::UnsignedLongLongInt,
            )
            | Scalar::Nullptr => Type::I64,
            Scalar::Basic(F::Int128 | F::UnsignedInt128) => Type::I128,
            Scalar::Basic(F::Float | F::Float32) => Type::F32,
            Scalar::Basic(F::Double | F::Float64 | F::Float32x) => Type::F64,
            Scalar::Basic(F::LongDouble | F::Float64x) => Type::F80,
            Scalar::Basic(F::Float16 | F::BFloat16) => {
                return Ok(Storage::Object { bytes: 2, align: 1 });
            }
            Scalar::Basic(F::Float128 | F::GnuFloat128) => {
                return Ok(Storage::Object {
                    bytes: 16,
                    align: 4,
                });
            }
        }),
        TypeView::Pointer(_)
        | TypeView::BlockPointer(_)
        | TypeView::Reference { .. }
        | TypeView::Function { .. } => Storage::Scalar(Type::Ptr),
        TypeView::Array { element, bound } => {
            if bound == 0 {
                return Err(Error::Missing("unknown-bound array storage layout"));
            }
            let mut element = element;
            let mut count = bound;
            while let TypeView::Array {
                element: inner,
                bound,
            } = graph.lower_type(element)?
            {
                if bound == 0 {
                    return Err(Error::Missing("unknown-bound array storage layout"));
                }
                count = count.checked_mul(bound).ok_or(crate::lowir::Error::Limit)?;
                element = inner;
            }
            // object_layout checks a direct class before type_size, but an
            // array's element goes through type_size at the unlocated layout
            // boundary and preserves its incomplete-class diagnostic.
            if matches!(graph.lower_type(element)?, TypeView::Class(_))
                && !graph.lower_complete_object(element)?
            {
                return Err(crate::sema::Error::at(
                    crate::support::source::SourceOffset(0),
                    "size of an incomplete class",
                )
                .into());
            }
            let (bytes, align) = layout(graph, element)?;
            Storage::Object {
                bytes: bytes.checked_mul(count).ok_or(crate::lowir::Error::Limit)?,
                align,
            }
        }
        TypeView::Enum(underlying) => return storage_layout(graph, underlying),
        TypeView::Class(entity) => {
            if !graph.lower_complete_object(ty)? {
                return Err(crate::sema::Error::at(
                    crate::support::source::SourceOffset(0),
                    "a type outside the procedural lowering slice",
                )
                .into());
            }
            let (mut bytes, mut align) = graph.lower_class_layout(entity)?;
            if graph.lower_cv(ty) & 4 != 0 && bytes <= 16 {
                bytes = bytes.max(1).next_power_of_two();
                align =
                    u8::try_from(bytes.trailing_zeros()).map_err(|_| crate::lowir::Error::Limit)?;
            }
            Storage::Object { bytes, align }
        }
        TypeView::Value { .. } | TypeView::Address { .. } => {
            return Err(Error::Missing("template argument used as storage type"));
        }
        TypeView::Unsupported => return Err(Error::Missing("aggregate storage lowering")),
    })
}
pub(super) fn layout(graph: &Graph, ty: TypeId) -> Result<(u64, u8), Error> {
    if let Some((_, element)) = graph.lower_member_pointer_type(ty) {
        return Ok(
            if matches!(graph.lower_type(element)?, TypeView::Function { .. }) {
                (16, 3)
            } else {
                (8, 3)
            },
        );
    }
    let ty = storage_layout(graph, ty)?;
    Ok(match ty {
        Storage::Object { bytes, align } => (bytes, align),
        Storage::Scalar(ty) => match ty {
            Type::F80 | Type::I128 => (16, 4),
            Type::F64 | Type::Ptr | Type::I64 => (8, 3),
            Type::F32 | Type::I32 | Type::U32 => (4, 2),
            Type::I16 | Type::U16 => (2, 1),
            Type::I8 | Type::U8 => (1, 0),
            _ => return Err(Error::Missing("aggregate target layout")),
        },
    })
}
// ProgramLowering::size_of is a located size demand, distinct from the
// storage-type view's unlocated unsupported-type diagnostic.
pub(super) fn size_of(
    graph: &Graph,
    ty: TypeId,
    location: crate::support::source::SourceOffset,
) -> Result<u64, Error> {
    if graph.lower_hosted_unknown_array(ty) {
        return Ok(0);
    }
    if !graph.lower_complete_object(ty)? {
        return Err(crate::sema::Error::at(location, "the size of an incomplete type").into());
    }
    Ok(layout(graph, ty)?.0)
}
pub(super) fn local_storage(
    graph: &Graph,
    entity: crate::sema::lowering::EntityId,
    ty: TypeId,
) -> Result<Type, Error> {
    let storage = storage(graph, ty)?;
    let requested = u64::from(graph.lower_requested_alignment(entity));
    if requested != 0 && !matches!(graph.lower_type(ty)?, TypeView::Reference { .. }) {
        let (size, natural) = layout(graph, ty)?;
        if requested > 1u64 << natural {
            return Ok(Type::object(
                size.wrapping_add(requested - 1) / requested * requested,
                u8::try_from(requested.trailing_zeros()).map_err(|_| crate::lowir::Error::Limit)?,
            ));
        }
    }
    Ok(storage)
}
pub(super) fn storage_alignment(ty: Type) -> u8 {
    match ty {
        Type::Object { align, .. } => align,
        Type::F80 | Type::I128 => 4,
        Type::F64 | Type::Ptr | Type::I64 => 3,
        Type::F32 | Type::I32 | Type::U32 => 2,
        Type::I16 | Type::U16 => 1,
        _ => 0,
    }
}
pub(super) fn signed(graph: &Graph, ty: TypeId) -> Result<bool, Error> {
    Ok(match graph.lower_type(ty)? {
        TypeView::Enum(underlying) => return signed(graph, underlying),
        TypeView::Scalar(
            Scalar::SignedChar
            | Scalar::Short
            | Scalar::Basic(F::Char | F::WcharT | F::Int | F::LongInt | F::LongLongInt | F::Int128),
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
pub(super) fn character(graph: &Graph, ty: TypeId) -> Result<bool, Error> {
    Ok(matches!(
        graph.lower_type(ty)?,
        TypeView::Scalar(
            Scalar::SignedChar
                | Scalar::UnsignedChar
                | Scalar::Basic(F::Char | F::Char16T | F::Char32T | F::WcharT)
        )
    ))
}
pub(super) fn memory(graph: &Graph, ty: TypeId) -> Result<crate::lowir::model::Facts, Error> {
    let ty_ir = storage(graph, ty)?;
    let volatile = graph.lower_cv(ty) & 2 != 0;
    Ok(crate::lowir::model::Facts::memory(
        volatile,
        !volatile && !matches!(ty_ir, Type::I8 | Type::U8 | Type::Object { .. }),
    ))
}
