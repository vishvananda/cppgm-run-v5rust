//! Typed body-to-body copying for `opt/inliner.cpp`; no textual transport. See NOTICE.
use super::{
    Arguments, BlockId, BlockName, Body, Error, InstId, Instruction, InstructionKind as I,
    LocationId, NameId, OptimizerBlock, SlotId, TempId, Value,
    body_edits::{Remap, rewrite},
};
use crate::lowir::construction_names::SlotName;

pub struct ImportMap {
    pub temps: Vec<Option<Value>>,
    pub slots: Vec<Option<Value>>,
    pub blocks: Vec<Option<BlockId>>,
    locations: Vec<Option<LocationId>>,
}
impl ImportMap {
    #[must_use]
    pub fn new(source: &Body) -> Self {
        Self {
            temps: vec![None; source.temps().len()],
            slots: vec![None; source.slots().len()],
            blocks: vec![None; source.blocks().len()],
            locations: vec![None; source.locations.len()],
        }
    }
}
struct Importer<'a> {
    target: &'a mut Body,
    source: &'a Body,
    map: &'a ImportMap,
}
impl Remap for Importer<'_> {
    fn value(&mut self, value: Value) -> Result<Value, Error> {
        match value {
            Value::Temp(t) => self.map.temps[t.index()].ok_or(Error::Limit),
            Value::Slot(s) => self.map.slots[s.index()].ok_or(Error::Limit),
            Value::Integer(id) => self.target.integer(self.source.literal(id)),
            Value::Float(id, kind) => {
                self.target
                    .constant(crate::lowir::literals::Constant::Float(
                        kind,
                        self.source.literal(id),
                    ))
            }
            Value::Symbol(_) => Ok(value),
        }
    }
    fn block(&mut self, block: BlockId) -> Result<BlockId, Error> {
        self.map.blocks[block.index()].ok_or(Error::Limit)
    }
    fn arguments(&mut self, args: Arguments) -> Result<Arguments, Error> {
        let mut out = Vec::with_capacity(self.source.arguments(args).len());
        for &value in self.source.arguments(args) {
            out.push(self.value(value)?);
        }
        self.target.add_arguments(&out)
    }
}
impl Body {
    /// An intentional owned snapshot for recursive/specialized inlining.
    #[must_use]
    pub fn inline_snapshot(&self) -> Self {
        Self {
            literals: self.literals.clone(),
            operands: self.operands.clone(),
            use_scratch: Vec::new(),
            temps: self.temps.clone(),
            slots: self.slots.clone(),
            slot_names: self.slot_names.clone(),
            blocks: self.blocks.clone(),
            instructions: self.instructions.clone(),
            locations: self.locations.clone(),
            temp_index: self.temp_index.clone(),
            slot_index: self.slot_index.clone(),
            block_index: self.block_index.clone(),
            layout: self.layout.clone(),
            parameter_count: self.parameter_count,
            source: self.source,
        }
    }
    #[inline]
    pub fn import_temp(&mut self, source: &Body, temp: TempId) -> Result<Value, Error> {
        let source = source.temp(temp);
        let id = self.generated_temp(source.ty.ok_or(Error::Limit)?)?;
        self.temps[id.index()].name = source.name;
        Ok(Value::Temp(id))
    }
    pub fn import_slot(&mut self, source: &Body, slot: SlotId) -> Result<Value, Error> {
        if self.slot_names.len() < self.slots.len() {
            self.slot_names
                .extend(self.slots[self.slot_names.len()..].iter().map(|s| {
                    if s.name == NameId(0) {
                        SlotName::Anonymous
                    } else {
                        SlotName::Source(s.name)
                    }
                }));
        }
        let id = SlotId(u32::try_from(self.slots.len()).map_err(|_| Error::Limit)?);
        let copied = source.slot(slot).clone();
        let origin =
            source
                .slot_names
                .get(slot.index())
                .copied()
                .unwrap_or(if copied.name == NameId(0) {
                    SlotName::Anonymous
                } else {
                    SlotName::Source(copied.name)
                });
        let origin = match origin {
            SlotName::Source(name) => SlotName::GeneratedName(name),
            SlotName::Anonymous | SlotName::Parameter(_) => SlotName::Generated("inlined"),
            origin => origin,
        };
        self.slots.push(copied);
        self.slot_names.push(origin);
        Ok(Value::Slot(id))
    }
    pub fn import_block(&mut self, source: &Body, block: BlockId) -> Result<BlockId, Error> {
        let copied = source.block(block);
        let id = self.generated_block()?;
        self.blocks[id.index()].origin = match copied.origin {
            BlockName::Source(_) | BlockName::Anonymous
                if source.layout.first() == Some(&block) =>
            {
                BlockName::Optimizer(OptimizerBlock::InlineEntry)
            }
            BlockName::Source(name) => BlockName::CopiedSource(name),
            BlockName::CopiedSource(_) | BlockName::Anonymous => {
                BlockName::Optimizer(OptimizerBlock::InlineCopy)
            }
            origin => origin,
        };
        Ok(id)
    }
    pub fn import_instruction(
        &mut self,
        source: &Body,
        id: InstId,
        map: &mut ImportMap,
        kept_predecessors: Option<&[bool]>,
    ) -> Result<Instruction, Error> {
        let mut inst = source.instruction(id).clone();
        if let I::Phi(_, pairs) = &mut inst.kind
            && let Some(kept) = kept_predecessors
        {
            pairs.retain(|(block, _)| kept[block.index()]);
        }
        rewrite(
            &mut inst.kind,
            &mut Importer {
                target: self,
                source,
                map,
            },
        )?;
        if let Some(result) = inst.result {
            let Some(Value::Temp(temp)) = map.temps[result.index()] else {
                return Err(Error::Limit);
            };
            inst.result = Some(temp);
        }
        if let Some(location) = inst.location {
            let index = (location.0.get() - 1) as usize;
            if map.locations[index].is_none() {
                let next = self.locations.len().checked_add(1).ok_or(Error::Limit)?;
                let next = u32::try_from(next).map_err(|_| Error::Limit)?;
                self.locations.push(source.locations[index]);
                map.locations[index] = Some(LocationId(
                    std::num::NonZeroU32::new(next).ok_or(Error::Limit)?,
                ));
            }
            inst.location = map.locations[index];
        }
        Ok(inst)
    }
}
