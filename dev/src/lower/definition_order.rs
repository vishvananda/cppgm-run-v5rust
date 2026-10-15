// Translated from lower/definition_order.cpp's constructor entry families. See NOTICE.
//! Serialized spans retain completion order while function bodies are released.
use super::Error;
use crate::lowir::model::{NameId, Program, SymbolId};
use std::{
    collections::HashMap,
    io::{self, Write},
    ops::Range,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FamilyId(u32);
struct Definition {
    bytes: Range<usize>,
    header: Option<SymbolId>,
}
#[derive(Default)]
pub(crate) struct Definitions {
    bytes: Vec<u8>,
    spans: Vec<Definition>,
    held: Vec<Definition>,
    external: HashMap<NameId, FamilyId>,
    families: Vec<[Option<usize>; 2]>,
}
impl Definitions {
    pub(crate) fn is_empty(&self) -> bool {
        self.spans.is_empty() && self.held.is_empty()
    }
    pub(super) fn family(&mut self, object: Option<NameId>) -> Result<FamilyId, Error> {
        if let Some(object) = object
            && let Some(&id) = self.external.get(&object)
        {
            return Ok(id);
        }
        let id =
            FamilyId(u32::try_from(self.families.len()).map_err(|_| crate::lowir::Error::Limit)?);
        self.families.push([None, None]);
        if let Some(object) = object {
            self.external.insert(object, id);
        }
        Ok(id)
    }
    pub(super) fn stream(
        &mut self,
        program: &mut Program,
        entry: Option<(FamilyId, super::construction::Variant)>,
    ) -> Result<(), Error> {
        let start = self.bytes.len();
        program.stream_last_definition(&mut self.bytes)?;
        let position = self.spans.len();
        self.spans.push(Definition {
            bytes: start..self.bytes.len(),
            header: None,
        });
        if let Some((family, variant)) = entry {
            let rank = usize::from(variant == super::construction::Variant::Complete);
            let positions = &mut self.families[family.0 as usize];
            positions[rank] = Some(position);
            if let [Some(base), Some(complete)] = *positions
                && base > complete
            {
                self.spans.swap(base, complete);
                *positions = [Some(complete), Some(base)];
            }
        }
        Ok(())
    }
    pub(crate) fn write(&self, program: &Program, output: &mut impl Write) -> io::Result<()> {
        for span in self.spans.iter().chain(&self.held) {
            if let Some(symbol) = span.header {
                let function = program
                    .function(program.symbol(symbol))
                    .ok_or_else(|| io::Error::other("deferred function header without identity"))?;
                crate::lowir::writer::write_definition_header(program, function, output)?;
            }
            output.write_all(&self.bytes[span.bytes.clone()])?;
        }
        Ok(())
    }
    pub(super) fn defer_definition(&mut self, program: &mut Program) -> Result<(), Error> {
        let start = self.bytes.len();
        let symbol = program.stream_last_body(&mut self.bytes)?;
        self.held.push(Definition {
            bytes: start..self.bytes.len(),
            header: Some(symbol),
        });
        Ok(())
    }
    pub(super) fn release_held(&mut self) {
        self.spans.append(&mut self.held);
    }
}
