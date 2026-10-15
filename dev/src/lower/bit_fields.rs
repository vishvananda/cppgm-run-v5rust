// Translated from lower/aggregate_lowering.cpp's bit-field storage operations. See NOTICE.
//! Access descriptors consume completed layout; initialization state belongs to one subobject.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Binary, Convert, Type},
    },
    sema::lowering::{EntityId, TypeId},
};

/// The storage facts shared by synthesized constructors and assignments.
#[derive(Clone, Copy)]
pub(super) struct Transfer {
    pub member: Option<EntityId>,
    pub ty: TypeId,
    pub offset: u64,
    pub trivial: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Field {
    pub source: TypeId,
    pub operation: TypeId,
    pub declared: Type,
    pub unit: Type,
    pub offset: u64,
    pub shift: u32,
    pub width: u32,
    pub signed: bool,
    pub unit_unsigned: bool,
    pub shared: bool,
    pub split: u8,
}
#[derive(Clone, Copy)]
pub(super) struct Unit {
    pub address: Value,
    pub volatile: bool,
}
#[derive(Clone, Copy)]
pub(super) struct Initialization {
    pub unit: u64,
    pub high: u64,
    pub limit: u64,
}
impl Default for Initialization {
    fn default() -> Self {
        Self {
            unit: 0,
            high: 0,
            limit: u64::MAX,
        }
    }
}
fn ones(bits: u32) -> u128 {
    if bits == 128 {
        u128::MAX
    } else {
        (1u128 << bits) - 1
    }
}
fn pieces(bytes: u8) -> impl Iterator<Item = (u8, u8)> {
    let mut at = 0;
    std::iter::from_fn(move || {
        if at == bytes {
            return None;
        }
        let mut width = 8;
        while width > bytes - at {
            width /= 2;
        }
        let offset = at;
        at += width;
        Some((offset, width))
    })
}
fn piece_type(width: u8) -> Type {
    match width {
        1 => Type::U8,
        2 => Type::U16,
        4 => Type::U32,
        _ => Type::I64,
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn transfer_bit_fields(
        &mut self,
        function: EntityId,
        class: EntityId,
        actions: &[Transfer],
        first: usize,
        limit: u64,
    ) -> Result<usize, Error> {
        let member = actions[first]
            .member
            .ok_or(crate::sema::Error::Invariant("assignment field member"))?;
        let field = self.field(member)?;
        let mut last = first;
        for (index, action) in actions.iter().enumerate().skip(first + 1) {
            let Some(member) = action.member else { break };
            let Some(layout) = self.graph.lower_bit_field(member)? else {
                break;
            };
            if layout.width == 0 {
                break;
            }
            if self.graph.lower_local_name(member).is_some() {
                let next = self.field(member)?;
                if next.offset != field.offset || next.unit != field.unit {
                    break;
                }
            }
            last = index;
        }
        let end = field.offset.wrapping_add(u64::from(field.unit.bits() / 8));
        let mut whole = !field.shared && end <= limit;
        for action in &actions[first..=last] {
            whole &= self.graph.lower_cv(action.ty) & 2 == 0;
        }
        // Members are in offset order: inspect adjacent overlaps only. Bases
        // may be laid out separately and are checked once per allocation unit.
        for &action in actions[..first]
            .iter()
            .rev()
            .take_while(|a| a.member.is_some())
        {
            let (_, stop) = self.transfer_bytes(action)?;
            if stop <= field.offset {
                break;
            }
            whole &= self.transfer_bytes_trivial(action);
        }
        for &action in actions[..first].iter().take_while(|a| a.member.is_none()) {
            let (start, stop) = self.transfer_bytes(action)?;
            if start < end && stop > field.offset {
                whole &= self.transfer_bytes_trivial(action);
            }
        }
        for &action in &actions[last + 1..] {
            let (start, stop) = self.transfer_bytes(action)?;
            if start >= end {
                break;
            }
            if start < stop {
                whole &= self.transfer_bytes_trivial(action);
            }
        }
        let ty = self.graph.lower_entity_type(class)?;
        if whole {
            let unit = match field.unit.bits() {
                8 => Type::U8,
                16 => Type::U16,
                32 => Type::U32,
                _ => field.unit,
            };
            let source = self.assignment_source(function)?;
            let offset = self.builder.integer(u128::from(field.offset))?;
            let source = self
                .emit(
                    I::Index(
                        Type::I8,
                        Some(crate::lowir::vocabulary::Projection::Field),
                        source,
                        offset,
                    ),
                    ty,
                )?
                .value;
            let value = self
                .emit(I::Load(Facts::memory(false, false), unit, source), ty)?
                .value;
            let destination = self.assignment_this(ty)?;
            let destination = self
                .emit(
                    I::Index(
                        Type::I8,
                        Some(crate::lowir::vocabulary::Projection::Field),
                        destination,
                        offset,
                    ),
                    ty,
                )?
                .value;
            self.effect(I::Store(
                Facts::memory(false, false),
                unit,
                value,
                destination,
            ))?;
        } else {
            for action in &actions[first..=last] {
                let member = action
                    .member
                    .ok_or(crate::sema::Error::Invariant("assignment field member"))?;
                if self.graph.lower_local_name(member).is_none() {
                    continue;
                }
                let field = self.field(member)?;
                let volatile = self.graph.lower_cv(action.ty) & 2 != 0;
                let source = self.assignment_source(function)?;
                let offset = self.builder.integer(u128::from(field.offset))?;
                let address = self
                    .emit(
                        I::Index(
                            Type::I8,
                            Some(crate::lowir::vocabulary::Projection::Field),
                            source,
                            offset,
                        ),
                        ty,
                    )?
                    .value;
                let value = self.read_field_view(field, Unit { address, volatile }, false)?;
                let destination = self.assignment_this(ty)?;
                let address = self
                    .emit(
                        I::Index(
                            Type::I8,
                            Some(crate::lowir::vocabulary::Projection::Field),
                            destination,
                            offset,
                        ),
                        ty,
                    )?
                    .value;
                let value = self.convert_field_unit(field, value)?;
                self.write_field_view(field, Unit { address, volatile }, value, false)?;
            }
        }
        Ok(last)
    }
    fn transfer_bytes_trivial(&self, action: Transfer) -> bool {
        self.graph.lower_cv(action.ty) & 2 == 0 && action.trivial
    }
    fn transfer_bytes(&self, action: Transfer) -> Result<(u64, u64), Error> {
        let start = action.offset;
        let stop = if let Some(member) = action.member {
            if let Some(layout) = self.graph.lower_bit_field(member)? {
                layout.bit.wrapping_add(layout.width).wrapping_add(7) / 8
            } else {
                start.wrapping_add(types::layout(self.graph, action.ty)?.0)
            }
        } else if let crate::sema::lowering::TypeView::Class(base) =
            self.graph.lower_type(action.ty)?
        {
            start.wrapping_add(self.graph.lower_nonvirtual_bytes(base)?)
        } else {
            return Err(crate::sema::Error::Invariant("assignment base type").into());
        };
        Ok((start, stop))
    }
    pub(super) fn field(&self, entity: EntityId) -> Result<Field, Error> {
        let layout = self.graph.lower_bit_field(entity)?.ok_or(Error::Semantic(
            crate::sema::Error::Invariant("field access without bit-field"),
        ))?;
        let source = self.graph.lower_entity_type(entity)?;
        let declared = types::storage(self.graph, source)?;
        let declared_bits = declared.bits();
        let width = u32::try_from(layout.width.min(u64::from(declared_bits)))
            .map_err(|_| crate::lowir::Error::Limit)?;
        let signed = layout.signed;
        let access = if declared_bits < 32 || !signed && declared_bits == 32 && width < 32 {
            Type::I32
        } else {
            declared
        };
        let bits = u32::from(layout.bytes) * 8;
        let unit = if layout.split != 0 {
            if layout.split > 8 {
                Type::I128
            } else {
                Type::I64
            }
        } else if bits == 0 || bits == declared_bits {
            if declared_bits == access.bits() {
                access
            } else {
                declared
            }
        } else if bits == access.bits() {
            access
        } else {
            match (bits, signed) {
                (8, true) => Type::I8,
                (8, false) => Type::U8,
                (16, true) => Type::I16,
                (16, false) => Type::U16,
                (32, true) => Type::I32,
                (32, false) => Type::U32,
                (64, _) => Type::I64,
                (128, _) => Type::I128,
                _ => return Err(Error::Missing("bit-field storage unit integer width")),
            }
        };
        let shift = u32::try_from(layout.bit.wrapping_sub(layout.offset.wrapping_mul(8)))
            .map_err(|_| crate::lowir::Error::Limit)?;
        Ok(Field {
            source,
            operation: layout.operation,
            declared,
            unit,
            offset: layout.offset,
            shift,
            width,
            signed,
            unit_unsigned: matches!(unit, Type::U8 | Type::U16 | Type::U32)
                || matches!(unit, Type::I64 | Type::I128) && !signed,
            shared: layout.shared,
            split: layout.split,
        })
    }
    pub(super) fn field_integer(
        &mut self,
        field: Field,
        ty: Type,
        bits: u128,
    ) -> Result<RValue, Error> {
        // C++ integer_literal sign-extends the low half of <=64-bit masks.
        let bits = if ty.bits() <= 64 {
            u128::from_le_bytes(
                i128::from(i64::from_le_bytes(
                    u64::try_from(bits & u128::from(u64::MAX))
                        .map_err(|_| crate::lowir::Error::Limit)?
                        .to_le_bytes(),
                ))
                .to_le_bytes(),
            )
        } else {
            bits
        };
        Ok(RValue {
            value: self.builder.integer(bits)?,
            ty,
            source: field.source,
            literal: false,
            bits: u64::try_from(bits).ok(),
        })
    }
    pub(super) fn field_binary(
        &mut self,
        field: Field,
        op: Binary,
        ty: Type,
        lhs: RValue,
        rhs: RValue,
    ) -> Result<RValue, Error> {
        self.emit(
            I::Binary(op, Facts::default(), ty, lhs.value, rhs.value),
            field.source,
        )
    }
    fn field_mask(&mut self, field: Field, ty: Type, clear: bool) -> Result<RValue, Error> {
        let mask = ones(field.width) << if clear { field.shift } else { 0 };
        self.field_integer(
            field,
            ty,
            if clear { !mask & ones(ty.bits()) } else { mask },
        )
    }
    pub(super) fn load_unit(
        &mut self,
        field: Field,
        ty: Type,
        unit: Unit,
    ) -> Result<RValue, Error> {
        if field.split == 0 {
            return self.emit(
                I::Load(Facts::memory(unit.volatile, false), ty, unit.address),
                field.source,
            );
        }
        let mut assembled = None;
        for (at, width) in pieces(field.split) {
            let address = self.piece_address(field, unit.address, at)?;
            let piece_ty = piece_type(width);
            let mut piece = self.emit(
                I::Load(Facts::memory(unit.volatile, false), piece_ty, address),
                field.source,
            )?;
            if piece_ty != ty {
                piece = self.emit(
                    I::Convert(Convert::ZeroExtend, ty, piece_ty, piece.value),
                    field.source,
                )?;
            }
            if at != 0 {
                let shift = self.field_integer(field, ty, u128::from(at) * 8)?;
                piece = self.field_binary(field, Binary::Shl, ty, piece, shift)?;
            }
            assembled = Some(if let Some(old) = assembled {
                self.field_binary(field, Binary::Or, ty, old, piece)?
            } else {
                piece
            });
        }
        assembled.ok_or(Error::Semantic(crate::sema::Error::Invariant(
            "empty split unit",
        )))
    }
    pub(super) fn store_unit(
        &mut self,
        field: Field,
        ty: Type,
        value: RValue,
        unit: Unit,
    ) -> Result<(), Error> {
        if field.split == 0 {
            return self.effect(I::Store(
                Facts::memory(unit.volatile, false),
                ty,
                value.value,
                unit.address,
            ));
        }
        for (at, width) in pieces(field.split) {
            let mut piece = value;
            if at != 0 {
                let shift = self.field_integer(field, ty, u128::from(at) * 8)?;
                piece = self.field_binary(field, Binary::UShr, ty, value, shift)?;
            }
            let piece_ty = piece_type(width);
            if piece_ty != ty {
                piece = self.emit(
                    I::Convert(Convert::Truncate, piece_ty, ty, piece.value),
                    field.source,
                )?;
            }
            let address = self.piece_address(field, unit.address, at)?;
            self.effect(I::Store(
                Facts::memory(unit.volatile, false),
                piece_ty,
                piece.value,
                address,
            ))?;
        }
        Ok(())
    }
    fn piece_address(&mut self, field: Field, address: Value, at: u8) -> Result<Value, Error> {
        if at == 0 {
            return Ok(address);
        }
        let offset = self.builder.integer(u128::from(at))?;
        Ok(self
            .emit(I::Index(Type::I8, None, address, offset), field.source)?
            .value)
    }
    fn declared_width(&mut self, field: Field, value: RValue) -> Result<RValue, Error> {
        if value.ty == field.declared {
            return Ok(value);
        }
        let bits = value.ty.bits();
        if bits == field.declared.bits() {
            return self.emit(I::Copy(field.declared, value.value), field.source);
        }
        let op = if bits > field.declared.bits() {
            Convert::Truncate
        } else if field.signed {
            Convert::SignExtend
        } else {
            Convert::ZeroExtend
        };
        self.emit(
            I::Convert(op, field.declared, value.ty, value.value),
            field.source,
        )
    }
    fn sign_extend_field(&mut self, field: Field, value: RValue) -> Result<RValue, Error> {
        let shift = self.field_integer(
            field,
            field.unit,
            u128::from(field.unit.bits() - field.width),
        )?;
        let up = self.field_binary(field, Binary::Shl, field.unit, value, shift)?;
        self.field_binary(field, Binary::Shr, field.unit, up, shift)
    }
    pub(super) fn read_field(&mut self, field: Field, unit: Unit) -> Result<RValue, Error> {
        self.read_field_view(field, unit, true)
    }
    pub(super) fn read_field_view(
        &mut self,
        field: Field,
        unit: Unit,
        typed: bool,
    ) -> Result<RValue, Error> {
        let mut value = self.load_unit(field, field.unit, unit)?;
        if field.shift != 0 {
            let shift = self.field_integer(field, field.unit, u128::from(field.shift))?;
            value = self.field_binary(
                field,
                if field.unit_unsigned {
                    Binary::UShr
                } else {
                    Binary::Shr
                },
                field.unit,
                value,
                shift,
            )?;
        }
        let mask = self.field_mask(field, field.unit, false)?;
        value = self.field_binary(field, Binary::And, field.unit, value, mask)?;
        if field.signed {
            value = self.sign_extend_field(field, value)?;
        }
        if !typed && value.ty.bits() == field.declared.bits() {
            Ok(value)
        } else {
            self.declared_width(field, value)
        }
    }
    pub(super) fn write_field(
        &mut self,
        field: Field,
        unit: Unit,
        value: RValue,
    ) -> Result<RValue, Error> {
        self.write_field_view(field, unit, value, true)
    }
    pub(super) fn write_field_view(
        &mut self,
        field: Field,
        unit: Unit,
        value: RValue,
        typed: bool,
    ) -> Result<RValue, Error> {
        let mask = self.field_mask(field, field.unit, false)?;
        let masked = self.field_binary(field, Binary::And, field.unit, value, mask)?;
        let result = if field.signed {
            self.sign_extend_field(field, masked)?
        } else {
            masked
        };
        let result = if !typed && result.ty.bits() == field.declared.bits() {
            result
        } else {
            self.declared_width(field, result)?
        };
        self.merge_field(field, field.unit, unit, masked)?;
        Ok(result)
    }
    fn merge_field(
        &mut self,
        field: Field,
        ty: Type,
        unit: Unit,
        masked: RValue,
    ) -> Result<(), Error> {
        let mut positioned = masked;
        if field.shift != 0 {
            let shift = self.field_integer(field, ty, u128::from(field.shift))?;
            positioned = self.field_binary(field, Binary::Shl, ty, masked, shift)?;
        }
        let old = self.load_unit(field, ty, unit)?;
        let mask = self.field_mask(field, ty, true)?;
        let cleared = self.field_binary(field, Binary::And, ty, old, mask)?;
        let merged = self.field_binary(field, Binary::Or, ty, cleared, positioned)?;
        self.store_unit(field, ty, merged, unit)
    }
    pub(super) fn convert_field_unit(
        &mut self,
        field: Field,
        value: RValue,
    ) -> Result<RValue, Error> {
        if value.ty == field.unit {
            return Ok(value);
        }
        if let Some(bits) = value.bits {
            let to = field.unit.bits();
            let mut folded = u128::from(bits) & ones(to);
            if !field.unit_unsigned && to < 64 && folded & (1u128 << (to - 1)) != 0 {
                folded |= !ones(to);
            }
            if to == 128 && !field.unit_unsigned {
                folded = u128::from_le_bytes(
                    i128::from(i64::from_le_bytes(bits.to_le_bytes())).to_le_bytes(),
                );
            }
            return self.field_integer(field, field.unit, folded);
        }
        self.convert_integer(
            value,
            field.source,
            field.unit,
            matches!(field.unit, Type::I8 | Type::I16 | Type::I32 | Type::I64),
            Context::Assignment,
        )
    }
    pub(super) fn initialize_field_value(
        &mut self,
        field: Field,
        unit: Unit,
        mut value: RValue,
    ) -> Result<(), Error> {
        let mut ty = field.declared;
        if field.unit.bits() != field.declared.bits() {
            ty = field.unit;
            value = self.convert_field_unit(field, value)?;
        }
        let whole = self.whole_field_unit(field);
        let mask = self.field_mask(field, ty, false)?;
        let masked = self.field_binary(field, Binary::And, ty, mask, value)?;
        if whole {
            let mut positioned = masked;
            if field.shift != 0 {
                let shift = self.field_integer(field, ty, u128::from(field.shift))?;
                positioned = self.field_binary(field, Binary::Shl, ty, masked, shift)?;
            }
            self.store_unit(field, ty, positioned, unit)?;
        } else {
            self.merge_field(field, ty, unit, masked)?;
        }
        self.bit_initialization.unit = field.offset + 1;
        Ok(())
    }
    pub(super) fn whole_field_unit(&self, field: Field) -> bool {
        !field.shared
            && self.bit_initialization.unit != field.offset + 1
            && field.offset.wrapping_mul(8) >= self.bit_initialization.high
            && field
                .offset
                .wrapping_mul(8)
                .wrapping_add(u64::from(field.unit.bits()))
                <= self.bit_initialization.limit
    }
}
