//! Typed field access and copy expansion from `opt/aggregate_slots.cpp`. See NOTICE.
use super::{
    aggregate_slots::{Address, Aggregate, unsigned},
    alias_facts::{literal_integer, signed_size, signed_word},
    memory_contents::bytes,
};
use crate::lowir::{
    Error,
    literals::{Constant, FloatKind},
    model::{
        BlockId, Call, Facts, InstId, Instruction, InstructionKind as I, LinkEditor, OptimizerSlot,
        Value,
    },
    vocabulary::{Binary as B, Convert, Pass, Projection, Type},
};
struct Edit {
    block: BlockId,
    at: Option<InstId>,
    origin: InstId,
}
impl Aggregate<'_, '_> {
    fn append(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        kind: I,
        ty: Option<Type>,
    ) -> Result<InstId, Error> {
        let result = ty
            .map(|ty| self.context.body.generated_temp(ty))
            .transpose()?;
        let id = self
            .context
            .body
            .append(edit.block, Instruction::new(result, kind, 0))?;
        self.context.body.inherit_location(id, edit.origin);
        links.insert_after(edit.block, edit.at, id)?;
        edit.at = Some(id);
        Ok(id)
    }
    fn emit(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        kind: I,
        ty: Type,
    ) -> Result<Value, Error> {
        let id = self.append(links, edit, kind, Some(ty))?;
        Ok(Value::Temp(
            self.context
                .body
                .instruction(id)
                .result
                .ok_or(Error::Limit)?,
        ))
    }
    fn word(&mut self, bits: u64) -> Result<Value, Error> {
        self.context
            .body
            .constant(Constant::Integer(literal_integer(signed_size(bits))))
    }
    fn binary(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        op: B,
        ty: Type,
        left: Value,
        right: Value,
    ) -> Result<Value, Error> {
        self.emit(
            links,
            edit,
            I::Binary(op, Facts::default(), ty, left, right),
            ty,
        )
    }
    fn packed_field(
        &self,
        slot: crate::lowir::model::SlotId,
        offset: u64,
    ) -> Result<(Value, super::aggregate_slots::Field), Error> {
        let index = self.facts[slot.index()]
            .fields
            .iter()
            .position(|f| f.offset <= offset && offset < f.offset.wrapping_add(bytes(f.ty)))
            .ok_or(Error::Limit)?;
        let field = self.facts[slot.index()].fields[index];
        let slot = self.fields[slot.index()]
            .as_ref()
            .and_then(|fields| fields.get(index))
            .copied()
            .ok_or(Error::Limit)?;
        Ok((Value::Slot(slot), field))
    }
    fn load_packed(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        id: InstId,
        address: Address,
        ty: Type,
    ) -> Result<(), Error> {
        let offset = unsigned(address.offset);
        let (slot, field) = self.packed_field(address.slot, offset)?;
        let mut value = self.emit(
            links,
            edit,
            I::Load(Facts::default(), field.ty, slot),
            field.ty,
        )?;
        if offset > field.offset {
            let shift = self.word(8 * (offset - field.offset))?;
            value = self.binary(links, edit, B::UShr, field.ty, value, shift)?;
        }
        self.context
            .body
            .replace_kind(id, I::Convert(Convert::Truncate, ty, field.ty, value));
        Ok(())
    }
    fn store_packed(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        id: InstId,
        address: Address,
    ) -> Result<(), Error> {
        let I::Store(flags, ty, value, _) = self.context.body.instruction(id).kind else {
            return Err(Error::Limit);
        };
        let offset = unsigned(address.offset);
        let (slot, field) = self.packed_field(address.slot, offset)?;
        let shift = u32::try_from(8 * (offset - field.offset)).map_err(|_| Error::Limit)?;
        let bits = 8 * bytes(ty);
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1_u64 << bits) - 1
        }
        .wrapping_shl(shift);
        let end = field.offset.wrapping_add(bytes(field.ty));
        let mut written = 0_u64;
        for &(from, to) in &self.facts[address.slot.index()].writes {
            for at in from.max(field.offset)..to.min(end) {
                let shift = u32::try_from(8 * (at - field.offset)).map_err(|_| Error::Limit)?;
                written |= 0xff_u64.wrapping_shl(shift);
            }
        }
        let keep = !mask & written;
        let mut merged = None;
        if keep != 0 {
            let loaded = self.emit(
                links,
                edit,
                I::Load(Facts::default(), field.ty, slot),
                field.ty,
            )?;
            let mask = self.word(keep)?;
            merged = Some(self.binary(links, edit, B::And, field.ty, loaded, mask)?);
        }
        if let Value::Integer(literal) = value {
            let put = unsigned(signed_word(self.context.body.literal(literal))).wrapping_shl(shift)
                & mask;
            if let Some(previous) = merged {
                if put != 0 {
                    let constant = self.word(put)?;
                    merged = Some(self.binary(links, edit, B::Or, field.ty, previous, constant)?);
                }
            } else {
                merged = Some(self.word(put)?);
            }
        } else {
            let mut placed = self.emit(
                links,
                edit,
                I::Convert(Convert::ZeroExtend, field.ty, ty, value),
                field.ty,
            )?;
            if shift != 0 {
                let constant = self.word(u64::from(shift))?;
                placed = self.binary(links, edit, B::Shl, field.ty, placed, constant)?;
            }
            merged = Some(if let Some(previous) = merged {
                self.binary(links, edit, B::Or, field.ty, previous, placed)?
            } else {
                placed
            });
        }
        self.context.body.replace_kind(
            id,
            I::Store(
                Facts::memory(flags.volatile(), false),
                field.ty,
                merged.ok_or(Error::Limit)?,
                slot,
            ),
        );
        Ok(())
    }
    fn pointer(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        mut base: Value,
        offset: i64,
    ) -> Result<Value, Error> {
        if matches!(base, Value::Slot(_)) {
            base = self.emit(links, edit, I::Addr(base), Type::Ptr)?;
        }
        if offset == 0 {
            return Ok(base);
        }
        let constant = self.word(unsigned(offset))?;
        self.emit(
            links,
            edit,
            I::Index(Type::I8, Some(Projection::Field), base, constant),
            Type::Ptr,
        )
    }
    fn expand(&mut self, links: &mut LinkEditor, edit: &mut Edit, kind: &I) -> Result<(), Error> {
        let (size, align, mut source, dest) = match *kind {
            I::CopyObject(size, align, source, dest) => (size, align, Some(source), dest),
            I::ZeroInit(size, align, dest) => (size, align, None, dest),
            _ => return Err(Error::Limit),
        };
        if let Some(value @ Value::Temp(temp)) = source
            && self
                .context
                .body
                .temp(temp)
                .ty
                .is_some_and(|ty| matches!(ty, Type::Object { .. }))
        {
            let ty = self.context.body.temp(temp).ty.ok_or(Error::Limit)?;
            let slot = self
                .context
                .body
                .optimizer_slot(ty, OptimizerSlot::Aggregate)?;
            let address = self.emit(links, edit, I::Addr(slot), Type::Ptr)?;
            self.append(
                links,
                edit,
                I::CopyObject(size, align, value, address),
                None,
            )?;
            source = Some(address);
        }
        let from = source.and_then(|value| self.split_storage(value));
        let to = self.split_storage(dest);
        let lead = to.or(from).ok_or(Error::Limit)?;
        for field in self.facts[lead.slot.index()].fields.clone() {
            if field.offset < unsigned(lead.offset)
                || field.offset.wrapping_add(bytes(field.ty))
                    > unsigned(lead.offset).wrapping_add(size)
            {
                continue;
            }
            let relative = signed_size(field.offset).wrapping_sub(lead.offset);
            let value = if let Some(source) = source {
                let storage = from
                    .and_then(|at| self.field(at.slot, unsigned(at.offset.wrapping_add(relative))));
                let storage = if let Some(storage) = storage {
                    storage
                } else {
                    self.pointer(links, edit, source, relative)?
                };
                self.emit(
                    links,
                    edit,
                    I::Load(Facts::default(), field.ty, storage),
                    field.ty,
                )?
            } else {
                self.context.body.constant(
                    FloatKind::from_type(field.ty)
                        .map_or(Constant::Integer(0), |kind| Constant::Float(kind, 0)),
                )?
            };
            let storage =
                to.and_then(|at| self.field(at.slot, unsigned(at.offset.wrapping_add(relative))));
            let storage = if let Some(storage) = storage {
                storage
            } else {
                self.pointer(links, edit, dest, relative)?
            };
            self.append(
                links,
                edit,
                I::Store(Facts::default(), field.ty, value, storage),
                None,
            )?;
        }
        Ok(())
    }
    fn rewrite_call(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        id: InstId,
        call: &Call,
    ) -> Result<Option<InstId>, Error> {
        let passes = self
            .context
            .program
            .call_boundary(call)
            .map(|s| s.parameters.iter().map(|p| p.pass).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut after = Edit {
            block: edit.block,
            at: Some(id),
            origin: id,
        };
        for (index, pass) in passes
            .into_iter()
            .take(self.context.body.arguments(call.arguments).len())
            .enumerate()
        {
            let value = self.context.body.arguments(call.arguments)[index];
            let Some(at) = self.address(value).filter(|at| {
                pass == Some(Pass::IndirectResult) && self.fields[at.slot.index()].is_some()
            }) else {
                continue;
            };
            let ty = self.context.body.slot(at.slot).ty;
            let slot = self
                .context
                .body
                .optimizer_slot(ty, OptimizerSlot::Aggregate)?;
            let address = self.emit(links, edit, I::Addr(slot), Type::Ptr)?;
            self.context
                .body
                .replace_argument(call.arguments, index, address)?;
            self.expand(
                links,
                &mut after,
                &I::CopyObject(bytes(ty), 0, address, value),
            )?;
        }
        Ok(after.at)
    }
    fn rewrite_access(
        &mut self,
        links: &mut LinkEditor,
        edit: &mut Edit,
        id: InstId,
    ) -> Result<(), Error> {
        let kind = self.context.body.instruction(id).kind.clone();
        let (ty, value, storage, flags) = match kind {
            I::Load(flags, ty, storage) => (ty, None, storage, flags),
            I::Store(flags, ty, value, storage) => (ty, Some(value), storage, flags),
            _ => return Err(Error::Limit),
        };
        let Some(at) = self.split_storage(storage) else {
            return Ok(());
        };
        let offset = unsigned(at.offset);
        let packed = self.facts[at.slot.index()].fields.iter().any(|f| {
            f.packed
                && f.offset <= offset
                && offset < f.offset.wrapping_add(bytes(f.ty))
                && !(f.offset == offset && f.ty == ty)
        });
        if packed {
            if value.is_some() {
                self.store_packed(links, edit, id, at)?;
            } else {
                self.load_packed(links, edit, id, at, ty)?;
            }
        } else {
            let storage = self
                .field(at.slot, offset)
                .ok_or_else(|| Error::Construction("missing split field".into()))?;
            self.context.body.replace_kind(
                id,
                if let Some(value) = value {
                    I::Store(flags, ty, value, storage)
                } else {
                    I::Load(flags, ty, storage)
                },
            );
        }
        Ok(())
    }
    pub(super) fn rewrite(&mut self) -> Result<(), Error> {
        let mut links = LinkEditor::new(self.context.body)?;
        for block in self.context.body.layout.clone() {
            let mut previous = None;
            let mut cursor = links.first(block);
            while let Some(id) = cursor {
                let next = links.next(id);
                let kind = self.context.body.instruction(id).kind.clone();
                let mut edit = Edit {
                    block,
                    at: previous,
                    origin: id,
                };
                match kind {
                    I::Load(..) | I::Store(..) => self.rewrite_access(&mut links, &mut edit, id)?,
                    I::Call(ref call) => {
                        if let Some(after) = self.rewrite_call(&mut links, &mut edit, id, call)?
                            && after != id
                        {
                            previous = Some(after);
                            cursor = links.next(after);
                            continue;
                        }
                    }
                    I::CopyObject(_, _, source, dest)
                        if self.split_storage(source).is_some()
                            || self.split_storage(dest).is_some() =>
                    {
                        self.expand(&mut links, &mut edit, &kind)?;
                        links.unlink(block, id)?;
                        previous = edit.at;
                        cursor = next;
                        continue;
                    }
                    I::ZeroInit(_, _, dest) if self.split_storage(dest).is_some() => {
                        self.expand(&mut links, &mut edit, &kind)?;
                        links.unlink(block, id)?;
                        previous = edit.at;
                        cursor = next;
                        continue;
                    }
                    _ => (),
                }
                previous = Some(id);
                cursor = next;
            }
        }
        links.commit(self.context.body)
    }
}
