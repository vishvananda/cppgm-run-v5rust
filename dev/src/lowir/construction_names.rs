// Translated from lowir/local_names.cpp. See NOTICE.
//! Local origins remain typed; one output view owns ordinals and reusable spelling scratch.
use super::{
    Error,
    model::{BlockId, BlockName, Body, NameId, Names},
};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use std::fmt::Write as _;

#[derive(Clone, Copy)]
pub(super) enum SlotName {
    Source(NameId),
    Shadow(NameId),
    Anonymous,
    RangeFor(NameId),
    Generated(&'static str),
    GeneratedName(NameId),
    Parameter(usize),
}
pub(super) fn record_shadows(labels: &mut [SlotName]) {
    let mut seen = HashSet::default();
    for label in labels {
        if let SlotName::Source(name) = *label
            && !seen.insert(name)
        {
            *label = SlotName::Shadow(name);
        }
    }
}

pub(crate) struct LocalNames<'a> {
    names: &'a Names,
    body: &'a Body,
    values: HashSet<&'a str>,
    blocks: HashSet<&'a str>,
    slots: Vec<u32>,
    temps: Vec<u32>,
    scratch: String,
}
impl<'a> LocalNames<'a> {
    pub(crate) fn new(names: &'a Names, body: &'a Body) -> Result<Self, Error> {
        let values = body
            .explicit_value_names()
            .map(|name| names.get(name))
            .filter(|name| collision_spelling(name))
            .collect();
        let blocks = body
            .blocks()
            .iter()
            .filter_map(super::model::Block::source_name)
            .map(|name| names.get(name))
            .filter(|name| collision_spelling(name))
            .collect();
        let mut view = Self {
            names,
            body,
            values,
            blocks,
            slots: Vec::new(),
            temps: Vec::new(),
            scratch: String::new(),
        };
        view.ordinals()?;
        Ok(view)
    }
    fn ordinals(&mut self) -> Result<(), Error> {
        let mut generated = 0_u32;
        let mut range_for = 0_u32;
        let shadows = (0..self.body.slots().len())
            .any(|index| matches!(self.body.slot_name(index), SlotName::Shadow(_)));
        let mut counts = HashMap::<NameId, u32>::default();
        self.slots.resize(self.body.slots().len(), 0);
        for index in 0..self.slots.len() {
            self.slots[index] = match self.body.slot_name(index) {
                SlotName::Source(name) | SlotName::Shadow(name) if shadows => {
                    let count = counts.entry(name).or_default();
                    *count = count.checked_add(1).ok_or(Error::Limit)?;
                    *count
                }
                SlotName::Generated(stem) => self.generated_ordinal(stem, &mut generated)?,
                SlotName::GeneratedName(name) => {
                    self.generated_ordinal(self.names.get(name), &mut generated)?
                }
                SlotName::RangeFor(_) => {
                    range_for = range_for.checked_add(1).ok_or(Error::Limit)?;
                    range_for
                }
                SlotName::Source(_)
                | SlotName::Shadow(_)
                | SlotName::Anonymous
                | SlotName::Parameter(_) => 0,
            };
        }
        if !self.values.iter().any(|name| temporary_spelling(name)) {
            return Ok(());
        }
        self.temps.resize(self.body.temps().len(), 0);
        let mut ordinal = 0_u32;
        for index in self.body.parameter_count..self.temps.len() {
            loop {
                ordinal = ordinal.checked_add(1).ok_or(Error::Limit)?;
                self.scratch.clear();
                write!(self.scratch, "t{ordinal}").map_err(|_| Error::Limit)?;
                if !self.values.contains(self.scratch.as_str()) {
                    break;
                }
            }
            self.temps[index] = ordinal;
        }
        Ok(())
    }
    fn generated_ordinal(&mut self, stem: &str, next: &mut u32) -> Result<u32, Error> {
        loop {
            *next = next.checked_add(1).ok_or(Error::Limit)?;
            if self.values.is_empty() {
                return Ok(*next);
            }
            self.scratch.clear();
            write!(self.scratch, "{stem}__{next}").map_err(|_| Error::Limit)?;
            if !self.values.contains(self.scratch.as_str()) {
                return Ok(*next);
            }
        }
    }
    pub(crate) fn temp(&mut self, index: usize) -> Result<&str, Error> {
        let temp = self.body.temps().get(index).ok_or(Error::Limit)?;
        if !temp.generated && temp.name != NameId::empty() {
            return Ok(self.names.get(temp.name));
        }
        let (stem, separator, ordinal) = if index < self.body.parameter_count {
            (
                "__param",
                "",
                u32::try_from(index).map_err(|_| Error::Limit)?,
            )
        } else {
            let ordinal = if self.temps.is_empty() {
                u32::try_from(index - self.body.parameter_count + 1).map_err(|_| Error::Limit)?
            } else {
                self.temps[index]
            };
            if temp.name == NameId::empty() {
                ("t", "", ordinal)
            } else {
                (self.names.get(temp.name), "_", ordinal)
            }
        };
        spelling(&mut self.scratch, &self.values, stem, separator, ordinal)
    }
    pub(crate) fn slot(&mut self, index: usize) -> Result<&str, Error> {
        let ordinal = *self.slots.get(index).ok_or(Error::Limit)?;
        let (stem, separator, ordinal) = match self.body.slot_name(index) {
            SlotName::Source(name) if name != NameId::empty() => return Ok(self.names.get(name)),
            SlotName::Source(_) | SlotName::Anonymous => {
                ("", "", u32::try_from(index).map_err(|_| Error::Limit)?)
            }
            SlotName::Shadow(name) => (self.names.get(name), "__shadow", ordinal),
            SlotName::RangeFor(name) => (self.names.get(name), "", ordinal),
            SlotName::Generated(stem) => (stem, "__", ordinal),
            SlotName::GeneratedName(name) => (self.names.get(name), "__", ordinal),
            SlotName::Parameter(index) => (
                "__param",
                "",
                u32::try_from(index).map_err(|_| Error::Limit)?,
            ),
        };
        spelling(&mut self.scratch, &self.values, stem, separator, ordinal)
    }
    pub(crate) fn block(&mut self, id: BlockId) -> Result<&str, Error> {
        let (stem, separator) = match self.body.block_name(id) {
            BlockName::Source(name) if name != NameId::empty() => return Ok(self.names.get(name)),
            BlockName::Source(_) | BlockName::Anonymous => ("", ""),
            BlockName::Generated(stem) => (stem, "_"),
            BlockName::CopiedSource(name) => (self.names.get(name), "_"),
            BlockName::Optimizer(kind) => (kind.spelling(), "_"),
        };
        spelling(
            &mut self.scratch,
            &self.blocks,
            stem,
            separator,
            u32::try_from(id.index()).map_err(|_| Error::Limit)?,
        )
    }
}
fn collision_spelling(name: &str) -> bool {
    name.as_bytes()
        .last()
        .is_some_and(|&byte| byte.is_ascii_digit() || byte == b'_')
}
fn temporary_spelling(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() >= 2
        && bytes[0] == b't'
        && bytes[1] != b'0'
        && bytes[1..].iter().all(u8::is_ascii_digit)
}
fn spelling<'a>(
    scratch: &'a mut String,
    taken: &HashSet<&str>,
    stem: &str,
    separator: &str,
    ordinal: u32,
) -> Result<&'a str, Error> {
    scratch.clear();
    write!(scratch, "{stem}{separator}{ordinal}").map_err(|_| Error::Limit)?;
    while taken.contains(scratch.as_str()) {
        scratch.push('_');
    }
    Ok(scratch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts, InstructionKind as I, Program},
        vocabulary::Type,
        writer,
    };
    #[test]
    fn output_names_share_explicit_collisions_without_entering_the_unit_store() -> Result<(), Error>
    {
        let mut program = Program::default();
        let mut builder = FunctionBuilder::new(&mut program, "f", &[], Type::I64)?;
        builder.block("entry")?;
        let object = builder.generated_slot("argobj", Type::I64)?;
        let source = builder.slot("t1", Type::I64)?;
        let one = builder.integer(1)?;
        let value = builder.value(I::Const(Type::I64, one), "argobj__1")?;
        let copy = builder.generated(I::Copy(Type::I64, value))?;
        builder.effect(I::Store(Facts::default(), Type::I64, copy, object))?;
        builder.effect(I::Store(Facts::default(), Type::I64, value, source))?;
        let generated = builder.generated_block("if_end")?;
        let explicit = builder.block_id("if_end_1")?;
        builder.effect(I::Jump(generated))?;
        builder.place_block(generated)?;
        builder.effect(I::Jump(explicit))?;
        builder.place_block(explicit)?;
        builder.effect(I::Return(Type::I64, Some(copy)))?;
        let bytes = builder.program().names.storage_bytes();
        builder.finish()?;
        assert_eq!(program.names.storage_bytes(), bytes);
        assert!(program.names.find("argobj__2").is_none());
        assert!(program.names.find("if_end").is_none());
        assert!(program.names.find("if_end_1_").is_none());
        let mut text = Vec::new();
        writer::write(&program, &mut text).map_err(Error::Output)?;
        let text = String::from_utf8(text).map_err(|_| Error::Limit)?;
        assert!(text.contains("slot $argobj__2 : i64"), "{text}");
        assert!(text.contains("%t3 = copy i64 %argobj__1"), "{text}");
        assert!(text.contains("block ^if_end_1_:"), "{text}");
        assert_eq!(program.names.storage_bytes(), bytes);
        let symbol = program.publications()[0];
        let mut body = program.take_body(symbol)?.ok_or(Error::Limit)?;
        body.compact()?;
        assert_eq!(program.names.storage_bytes(), bytes);
        program.restore_body(symbol, body)?;
        let mut after = Vec::new();
        writer::write(&program, &mut after).map_err(Error::Output)?;
        assert_eq!(after, text.as_bytes());
        assert_eq!(program.names.storage_bytes(), bytes);
        Ok(())
    }
}
