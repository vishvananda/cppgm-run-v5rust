// Translated from lower/constant_lowering.cpp::constant_data/constant_placements.
// See NOTICE.
//! Project completed semantic cells into ordered static items during analysis.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Root, TemporaryValue},
    graph::EntityId,
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{Scalar, TypeKind},
};
use crate::support::source::SourceOffset;

impl Analyzer<'_> {
    pub(super) fn record_completed_image(&mut self, entity: EntityId) -> Result<bool, Error> {
        let Some(value) = self.graph.aggregate_constants.get(entity) else {
            return Ok(false);
        };
        let values: Vec<_> = value.values().collect();
        let mut placements = Vec::new();
        let mut tables = std::collections::HashSet::new();
        let mut units = std::collections::HashMap::<u64, (u8, u64)>::new();
        for (ty, offset, field, value) in values {
            if let TypeKind::Class(class) = self.graph.types.get(ty).kind
                && self.graph.lower_polymorphic(class)
                && tables.insert(offset)
            {
                let offset = u64::try_from(offset)
                    .map_err(|_| Error::Invariant("negative constant table placement"))?;
                placements.push((offset, 8, Item::Table(class)));
            }
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Class(_) | TypeKind::Array { .. }
            ) {
                continue;
            }
            let offset = u64::try_from(offset)
                .map_err(|_| Error::Invariant("negative constant placement"))?;
            if let Some(field) = field {
                let field = self
                    .graph
                    .entity(field)
                    .bit_field
                    .ok_or(Error::Invariant("constant field without layout"))?;
                let storage = self.graph.bit_fields.storage(field)?;
                if storage.shared || storage.split != 0 || storage.bytes > 8 {
                    return Ok(false);
                }
                let Some(TemporaryValue::Integer(value)) = value else {
                    return Ok(false);
                };
                let origin = offset
                    .wrapping_sub(storage.bit / 8)
                    .wrapping_add(storage.offset);
                let unit = units.entry(origin).or_insert((storage.bytes, 0));
                let bit = storage.bit.wrapping_sub(storage.offset.wrapping_mul(8));
                let width = u32::try_from(self.graph.bit_fields.width(field).min(64))
                    .map_err(|_| Error::Invariant("bounded constant field width"))?;
                let mask = u64::MAX.checked_shr(64 - width).unwrap_or(0);
                unit.1 |= (value.bits & mask)
                    .checked_shl(
                        u32::try_from(bit).map_err(|_| Error::Limit("constant field shift"))?,
                    )
                    .unwrap_or(0);
                continue;
            }
            let size = self.type_size(ty, SourceOffset(0))?.0;
            let item = match value {
                Some(TemporaryValue::Integer(value)) => {
                    Item::Scalar(ty, Value::Integer(value.bits))
                }
                Some(TemporaryValue::Floating(value)) => {
                    let TypeKind::Scalar(Scalar::Basic(kind)) = self.graph.types.get(ty).kind
                    else {
                        return Err(Error::Invariant("constant floating placement type"));
                    };
                    let format = self
                        .float_format(ty)
                        .ok_or(Error::Invariant("constant floating format"))?;
                    let bits = value
                        .value
                        .bits(format)
                        .ok_or(Error::Invariant("constant floating encoding"))?;
                    Item::Scalar(ty, Value::Floating(kind, bits))
                }
                Some(TemporaryValue::Pointer(pointer)) => match pointer.root {
                    Root::Null => Item::Zero(size),
                    Root::Variable(root) | Root::Temporary(root) | Root::Function(root) => {
                        Item::Scalar(ty, Value::Address(root, pointer.offset))
                    }
                    Root::Literal(root) => Item::Scalar(ty, Value::Literal(root, pointer.offset)),
                    _ => return Ok(false),
                },
                None => Item::Zero(size),
            };
            placements.push((offset, size, item));
        }
        for (offset, (size, bits)) in units {
            placements.push((offset, u64::from(size), Item::BitUnit(size, bits)));
        }
        placements.sort_by_key(|&(offset, _, _)| offset);
        let size = self
            .type_size(self.graph.lower_entity_type(entity)?, SourceOffset(0))?
            .0;
        let mut cursor = 0;
        let mut image = Vec::new();
        for (offset, size, item) in placements {
            if offset < cursor {
                return Ok(false);
            }
            append_zero(&mut image, offset - cursor)?;
            if let Item::Zero(size) = item {
                append_zero(&mut image, size)?;
            } else {
                image.push(item);
            }
            cursor = offset
                .checked_add(size)
                .ok_or(Error::Limit("constant placement size"))?;
        }
        let Some(tail) = size.checked_sub(cursor) else {
            return Ok(false);
        };
        append_zero(&mut image, tail)?;
        self.graph.array_images.set_constant(entity, image);
        Ok(true)
    }
}
