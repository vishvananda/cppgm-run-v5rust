// Translated from lower/definition_order.cpp's constructor entry families. See NOTICE.
//! Serialized spans or retained IDs carry completion and special-member order.
use super::Error;
use crate::lowir::model::{NameId, Program, SymbolId};
use crate::support::hash::HashMap;
use std::io;
use std::io::Write;
use std::ops::Range;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct FamilyId(u32);
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum FamilyKind {
    Entries,
    Constructors,
    Assignments,
}
struct Definition {
    bytes: Range<usize>,
    header: Option<SymbolId>,
    entry: Option<(FamilyId, u8)>,
    form: Option<(FamilyId, u8)>,
}
#[derive(Default)]
pub(crate) struct Definitions {
    retained: bool,
    bytes: crate::lowir::text_storage::DefinitionBytes,
    spans: Vec<Definition>,
    held: Vec<Definition>,
    external: HashMap<(FamilyKind, NameId), FamilyId>,
    next_family: u32,
}
impl Definitions {
    pub(crate) fn set_spill_factory(&mut self, factory: fn() -> io::Result<std::fs::File>) {
        self.bytes.set_spill_factory(factory);
    }
    pub(super) fn retain(&mut self) {
        self.retained = true;
    }
    pub(super) fn finish_retained(
        &mut self,
        program: &mut Program,
        startup: [Option<SymbolId>; 2],
    ) {
        if self.retained {
            program.finish_retained_definitions(startup);
            let spans: Vec<_> = self.spans.iter().chain(&self.held).collect();
            let mut order: Vec<_> = (0..spans.len()).collect();
            Self::order_families(&spans, &mut order, false);
            Self::order_families(&spans, &mut order, true);
            let mut completed: Vec<_> = order
                .into_iter()
                .filter_map(|index| spans[index].header)
                .collect();
            completed.extend(startup.into_iter().flatten());
            program.order_retained_definitions(&completed);
        }
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub(super) fn family(&mut self, object: Option<NameId>) -> Result<FamilyId, Error> {
        self.make_family(FamilyKind::Entries, object)
    }
    pub(super) fn form_family(
        &mut self,
        object: Option<NameId>,
        assignment: bool,
    ) -> Result<FamilyId, Error> {
        self.make_family(
            if assignment {
                FamilyKind::Assignments
            } else {
                FamilyKind::Constructors
            },
            object,
        )
    }
    fn make_family(&mut self, kind: FamilyKind, object: Option<NameId>) -> Result<FamilyId, Error> {
        if let Some(object) = object
            && let Some(&id) = self.external.get(&(kind, object))
        {
            return Ok(id);
        }
        let id = FamilyId(self.next_family);
        self.next_family = self
            .next_family
            .checked_add(1)
            .ok_or(crate::lowir::Error::Limit)?;
        if let Some(object) = object {
            self.external.insert((kind, object), id);
        }
        Ok(id)
    }
    pub(super) fn generated(&mut self, bytes: &mut Vec<u8>) -> Result<(), Error> {
        if bytes.is_empty() {
            return Ok(());
        }
        if self.retained {
            return Err(crate::lowir::Error::Construction(
                "serialized generated definition in retained mode".into(),
            )
            .into());
        }
        let start = self.bytes.len();
        self.bytes
            .write_all(bytes)
            .map_err(crate::lowir::Error::Output)?;
        bytes.clear();
        self.spans.push(Definition {
            bytes: start..self.bytes.len(),
            header: None,
            entry: None,
            form: None,
        });
        Ok(())
    }
    pub(super) fn stream(
        &mut self,
        program: &mut Program,
        entry: Option<(FamilyId, super::construction::Variant)>,
    ) -> Result<(), Error> {
        if program.has_definition_sink() {
            program.stream_last_definition(&mut io::sink())?;
            return Ok(());
        }
        let start = self.bytes.len();
        let header = if self.retained {
            Some(program.stream_last_body(&mut io::sink())?)
        } else {
            program.stream_last_definition(&mut self.bytes)?;
            None
        };
        self.spans.push(Definition {
            bytes: start..self.bytes.len(),
            header,
            entry: entry.map(|(family, variant)| {
                (
                    family,
                    match variant {
                        super::construction::Variant::Base => 0,
                        super::construction::Variant::Deleting => 1,
                        super::construction::Variant::Complete => 2,
                    },
                )
            }),
            form: None,
        });
        Ok(())
    }
    pub(super) fn note_form(&mut self, family: FamilyId, moving: bool) -> Result<(), Error> {
        self.spans
            .last_mut()
            .ok_or(crate::sema::Error::Invariant(
                "copy form without streamed definition",
            ))?
            .form = Some((family, u8::from(moving)));
        Ok(())
    }
    pub(crate) fn write(&self, program: &Program, output: &mut impl Write) -> io::Result<()> {
        let spans: Vec<_> = self.spans.iter().chain(&self.held).collect();
        let mut order: Vec<_> = (0..spans.len()).collect();
        // Copy before move, then base before complete within each form. A
        // family permutes only its occupied positions; other definitions stay.
        Self::order_families(&spans, &mut order, false);
        Self::order_families(&spans, &mut order, true);
        for index in order {
            let span = spans[index];
            if let Some(symbol) = span.header {
                let function = program
                    .function(program.symbol(symbol))
                    .ok_or_else(|| io::Error::other("deferred function header without identity"))?;
                crate::lowir::writer::write_definition_header(program, function, output)?;
            }
            self.bytes.copy_range(span.bytes.clone(), output)?;
        }
        Ok(())
    }
    fn order_families(spans: &[&Definition], order: &mut [usize], entries: bool) {
        let mut placed: Vec<_> = order
            .iter()
            .enumerate()
            .filter_map(|(position, &index)| {
                let family = if entries {
                    spans[index].entry
                } else {
                    spans[index].form
                };
                family.map(|(family, rank)| (family, position, rank, index))
            })
            .collect();
        placed.sort_unstable_by_key(|&(family, position, _, _)| (family, position));
        let mut first = 0;
        let mut slots = Vec::new();
        while first < placed.len() {
            let mut last = first + 1;
            while last < placed.len() && placed[last].0 == placed[first].0 {
                last += 1;
            }
            let family = &mut placed[first..last];
            if family.windows(2).any(|pair| pair[0].2 > pair[1].2) {
                slots.clear();
                slots.extend(family.iter().map(|&(_, position, _, _)| position));
                family.sort_unstable_by_key(|&(_, position, rank, _)| (rank, position));
                for (&position, &(_, _, _, index)) in slots.iter().zip(family.iter()) {
                    order[position] = index;
                }
            }
            first = last;
        }
    }
    pub(super) fn defer_definition(&mut self, program: &mut Program) -> Result<(), Error> {
        if program.has_definition_sink() {
            program.defer_last_definition()?;
            return Ok(());
        }
        let start = self.bytes.len();
        let symbol = if self.retained {
            program.defer_last_definition()?
        } else {
            program.stream_last_body(&mut self.bytes)?
        };
        self.held.push(Definition {
            bytes: start..self.bytes.len(),
            header: Some(symbol),
            entry: None,
            form: None,
        });
        Ok(())
    }
    pub(super) fn release_held(&mut self, program: &mut Program) {
        program.release_held_definitions();
        self.spans.append(&mut self.held);
    }
}
