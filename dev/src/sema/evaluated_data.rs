// Translated from lower/constant_lowering.cpp::constant_data/constant_placements.
// See NOTICE.
//! Project completed semantic cells into ordered static items during analysis.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Root, TemporaryValue},
    constexpr_frames::PlacementValue,
    graph::EntityId,
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::support::hash::HashMap;
use crate::support::source::SourceOffset;

// A shared or straddling field contributes only the bytes its value covers.
// Ordinary units keep their original width and stable declaration order.
pub(super) fn bit_units(
    storage: super::bit_fields::Storage,
    width: u64,
) -> Result<impl Iterator<Item = (u64, u8)>, Error> {
    let split = storage.shared || storage.split != 0 || storage.bytes > 8;
    let start = if split {
        storage.bit / 8
    } else {
        storage.offset
    };
    let end = if split {
        storage
            .bit
            .checked_add(width.min(128))
            .and_then(|end| end.checked_add(7))
            .map(|end| end / 8)
    } else {
        start.checked_add(1)
    }
    .ok_or(Error::Limit("constant field span"))?;
    Ok((start..end).map(move |at| (at, if split { 1 } else { storage.bytes })))
}

#[derive(Default)]
struct BitImage {
    bytes: HashMap<u64, u8>,
    units: Vec<(u64, u8)>,
}
impl BitImage {
    fn add(
        &mut self,
        storage: super::bit_fields::Storage,
        width: u64,
        origin: u64,
        bits: u128,
    ) -> Result<(), Error> {
        let first = origin
            .checked_mul(8)
            .and_then(|at| at.checked_add(storage.bit))
            .ok_or(Error::Limit("constant field offset"))?;
        for k in 0..width.min(128) {
            let bit = first
                .checked_add(k)
                .ok_or(Error::Limit("constant field bit"))?;
            let value = bits
                .checked_shr(u32::try_from(k).map_err(|_| Error::Limit("field shift"))?)
                .unwrap_or(0)
                & 1;
            *self.bytes.entry(bit / 8).or_default() |=
                u8::try_from(value).map_err(|_| Error::Invariant("field bit value"))? << (bit % 8);
        }
        for (at, size) in bit_units(storage, width)? {
            self.units.push((
                origin
                    .checked_add(at)
                    .ok_or(Error::Limit("constant field unit"))?,
                size,
            ));
        }
        Ok(())
    }
    fn place(self, placements: &mut Vec<(u64, u64, Item)>) -> Result<(), Error> {
        for (offset, size) in self.units {
            let mut bits = 0_u64;
            for i in 0..size {
                let at = offset
                    .checked_add(u64::from(i))
                    .ok_or(Error::Limit("constant unit byte"))?;
                bits |= u64::from(self.bytes.get(&at).copied().unwrap_or(0)) << (u32::from(i) * 8);
            }
            placements.push((offset, u64::from(size), Item::BitUnit(size, bits)));
        }
        Ok(())
    }
}

impl Analyzer<'_> {
    pub(super) fn record_completed_image(&mut self, entity: EntityId) -> Result<bool, Error> {
        let Some(image) = self.completed_variable_image(entity)? else {
            return Ok(false);
        };
        self.graph.array_images.set_constant(entity, image);
        Ok(true)
    }
    pub(super) fn completed_variable_image(
        &mut self,
        entity: EntityId,
    ) -> Result<Option<Vec<Item>>, Error> {
        let Some(value) = self.graph.aggregate_constants.get(entity) else {
            return Ok(None);
        };
        let values = value.placement_values().collect();
        let ty = self.graph.lower_entity_type(entity)?;
        self.completed_cells(ty, values)
    }
    pub(super) fn completed_image(
        &mut self,
        ty: TypeId,
        value: &super::constexpr_frames::Frozen,
    ) -> Result<Option<Vec<Item>>, Error> {
        self.completed_cells(ty, value.placement_values().collect())
    }
    fn completed_cells(
        &mut self,
        ty: TypeId,
        values: Vec<PlacementValue>,
    ) -> Result<Option<Vec<Item>>, Error> {
        let mut element = ty;
        while let TypeKind::Array { element: child, .. } = self.graph.types.get(element).kind {
            element = child;
        }
        let scalar_array = element != ty && self.graph.class_of(element).is_none();
        let mut placements = Vec::new();
        let mut tables = crate::support::hash::HashSet::default();
        let mut bits = BitImage::default();
        for PlacementValue {
            depth,
            ty,
            offset,
            field,
            value,
        } in values
        {
            // constant_placements bounds every visited subobject, whereas
            // scalar_array_data bounds only arrays and writes scalar leaves
            // directly. Direct bit-fields bypass recursive placement.
            if depth > 256
                && field.is_none()
                && (!scalar_array
                    || matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }))
            {
                return Ok(None);
            }
            if let TypeKind::Class(class) = self.graph.types.get(ty).kind
                && self.graph.lower_polymorphic(class)
                && !tables.contains(&offset)
            {
                let offset = u64::try_from(offset)
                    .map_err(|_| Error::Invariant("negative constant table placement"))?;
                let first = placements.len();
                self.graph
                    .static_table_placements(class, offset, &mut placements)?;
                for &(offset, _, _) in &placements[first..] {
                    tables.insert(
                        i64::try_from(offset).map_err(|_| Error::Limit("constant table offset"))?,
                    );
                }
            }
            if self.vector_shape(ty).is_some() && self.constant_lane_vector(ty).is_none() {
                return Ok(None);
            }
            if self.constant_lane_vector(ty).is_some()
                || matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Class(_) | TypeKind::Complex(_) | TypeKind::Array { .. }
                )
            {
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
                let Some(TemporaryValue::Integer(value)) = value else {
                    return Ok(None);
                };
                let origin = offset
                    .checked_sub(storage.bit / 8)
                    .ok_or(Error::Invariant("constant field origin"))?;
                bits.add(
                    storage,
                    self.graph.bit_fields.width(field),
                    origin,
                    value.wide(),
                )?;
                continue;
            }
            let size = self.type_size(ty, SourceOffset(0))?.0;
            let item = match value {
                Some(TemporaryValue::MemberPointer(value)) => {
                    Item::Scalar(ty, Value::MemberPointer(value))
                }
                Some(TemporaryValue::Integer(value)) => {
                    Item::Scalar(ty, self.static_integer_value(value))
                }
                Some(TemporaryValue::Floating(value)) => {
                    let TypeKind::Scalar(Scalar::Basic(kind)) = self.graph.types.get(ty).kind
                    else {
                        return Err(Error::Invariant("constant floating placement type"));
                    };
                    let soft = kind.floating_representation();
                    if value.state == super::constant_addresses::ScalarState::Zeroed
                        && matches!(
                            soft,
                            crate::types::FundamentalType::Float16
                                | crate::types::FundamentalType::BFloat16
                                | crate::types::FundamentalType::Float128
                        )
                    {
                        placements.push((offset, size, Item::Zero(size)));
                        continue;
                    }
                    if matches!(
                        soft,
                        crate::types::FundamentalType::Float16
                            | crate::types::FundamentalType::Float128
                    ) {
                        return Ok(None);
                    }
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
                    Root::Variable(root) | Root::Function(root) => {
                        let value = Value::Address(root, pointer.offset);
                        if !self.static_storage_value(value) {
                            return Ok(None);
                        }
                        Item::Scalar(ty, value)
                    }
                    Root::Published(root) => {
                        let Some(value) = self.graph.static_temporary_value(root, pointer.offset)
                        else {
                            return Ok(None);
                        };
                        Item::Scalar(ty, value)
                    }
                    Root::Literal(root) => Item::Scalar(ty, Value::Literal(root, pointer.offset)),
                    Root::TypeInformation(target) => {
                        Item::Scalar(ty, Value::TypeInformation(target, pointer.offset))
                    }
                    Root::Object(_) => return Ok(None),
                },
                None => Item::Zero(size),
            };
            placements.push((offset, size, item));
        }
        bits.place(&mut placements)?;
        placements.sort_by_key(|&(offset, _, _)| offset);
        let size = self.type_size(ty, SourceOffset(0))?.0;
        let mut cursor = 0;
        let mut image = Vec::new();
        for (offset, size, item) in placements {
            if offset < cursor {
                if matches!(item, Item::BitUnit(..)) {
                    continue;
                }
                return Ok(None);
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
            return Ok(None);
        };
        append_zero(&mut image, tail)?;
        Ok(Some(image))
    }
}
