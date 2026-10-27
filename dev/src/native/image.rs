//! Native section ownership, compact symbol spaces and address fixups. See NOTICE.
use super::{
    executable,
    x86::{Assembler, CodeFixup, invariant},
};
use crate::lowir::{Error, model::SymbolId};
use std::{collections::HashMap, path::Path};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct LocalSymbol(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ImageSymbol {
    Program(SymbolId),
    Local(LocalSymbol),
}
impl ImageSymbol {
    pub(super) fn slot(self, programs: usize) -> usize {
        match self {
            Self::Program(id) => id.index(),
            Self::Local(id) => programs + id.0 as usize,
        }
    }
    pub(super) fn local_index(self) -> Option<usize> {
        match self {
            Self::Program(_) => None,
            Self::Local(id) => Some(id.0 as usize),
        }
    }
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Section {
    #[default]
    Undefined,
    Text,
    Rodata,
    Data,
    Bss,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Definition {
    pub(super) section: Section,
    pub(super) offset: u64,
}
pub(super) struct DataFixup {
    pub(super) section: Section,
    pub(super) offset: usize,
    pub(super) target: ImageSymbol,
    pub(super) addend: i64,
}
#[derive(PartialEq, Eq, Hash)]
struct ConstantKey {
    bytes: [u8; 16],
    length: u8,
    align: u8,
}
pub(super) struct Image {
    pub(super) text: Vec<u8>,
    pub(super) rodata: Vec<u8>,
    pub(super) data: Vec<u8>,
    pub(super) bss: u64,
    symbols: Vec<Definition>,
    locals: Vec<Definition>,
    fixups: Vec<CodeFixup>,
    data_fixups: Vec<DataFixup>,
    constants: HashMap<ConstantKey, ImageSymbol>,
}
pub(super) enum WriteError {
    Undefined,
    Other(Error),
}
impl From<Error> for WriteError {
    fn from(error: Error) -> Self {
        Self::Other(error)
    }
}
pub(super) fn align_up(value: u64, align: u64) -> Result<u64, Error> {
    value
        .checked_add(align - 1)
        .map(|n| n & !(align - 1))
        .ok_or(Error::Limit)
}
impl Image {
    pub(super) fn new(symbols: usize) -> Self {
        Self {
            text: Vec::new(),
            rodata: Vec::new(),
            data: Vec::new(),
            bss: 0,
            symbols: vec![Definition::default(); symbols],
            locals: Vec::new(),
            fixups: Vec::new(),
            data_fixups: Vec::new(),
            constants: HashMap::new(),
        }
    }
    pub(super) fn reserve_symbols(&mut self, count: usize) {
        self.symbols.resize(count, Definition::default());
    }
    pub(super) fn local(&mut self) -> Result<ImageSymbol, Error> {
        let id = LocalSymbol(u32::try_from(self.locals.len()).map_err(|_| Error::Limit)?);
        self.locals.push(Definition::default());
        Ok(ImageSymbol::Local(id))
    }
    pub(super) fn define(&mut self, symbol: ImageSymbol, section: Section, offset: u64) {
        let slot = match symbol {
            ImageSymbol::Program(id) => &mut self.symbols[id.index()],
            ImageSymbol::Local(id) => &mut self.locals[id.0 as usize],
        };
        *slot = Definition { section, offset };
    }
    pub(super) fn bss_symbol(&mut self, bytes: u64, align: u64) -> Result<ImageSymbol, Error> {
        let symbol = self.local()?;
        let offset = align_up(self.bss, align)?;
        self.bss = offset.checked_add(bytes).ok_or(Error::Limit)?;
        self.define(symbol, Section::Bss, offset);
        Ok(symbol)
    }
    pub(super) fn align_text(&mut self) -> Result<u64, Error> {
        let size = align_up(
            u64::try_from(self.text.len()).map_err(|_| Error::Limit)?,
            16,
        )?;
        self.text
            .resize(usize::try_from(size).map_err(|_| Error::Limit)?, 0xcc);
        Ok(size)
    }
    pub(super) fn append(&mut self, assembly: Assembler) -> Result<(), Error> {
        // CodeFixup offsets are target uint32_t values, even when the owned
        // text buffer has a larger host-sized extent.
        let base = u32::try_from(self.text.len() & u32::MAX as usize).map_err(|_| Error::Limit)?;
        for mut fixup in assembly.fixups {
            fixup.offset = fixup.offset.wrapping_add(base);
            self.fixups.push(fixup);
        }
        self.text.extend(assembly.code);
        Ok(())
    }
    pub(super) fn constant(&mut self, bytes: &[u8], align: u8) -> Result<ImageSymbol, Error> {
        let mut key = ConstantKey {
            bytes: [0; 16],
            length: u8::try_from(bytes.len()).map_err(|_| Error::Limit)?,
            align,
        };
        key.bytes
            .get_mut(..bytes.len())
            .ok_or_else(|| invariant("native constant wider than a scalar"))?
            .copy_from_slice(bytes);
        if let Some(&symbol) = self.constants.get(&key) {
            return Ok(symbol);
        }
        let offset = align_up(
            u64::try_from(self.rodata.len()).map_err(|_| Error::Limit)?,
            u64::from(align),
        )?;
        self.rodata
            .resize(usize::try_from(offset).map_err(|_| Error::Limit)?, 0);
        self.rodata.extend_from_slice(bytes);
        let symbol = self.local()?;
        self.define(symbol, Section::Rodata, offset);
        self.constants.insert(key, symbol);
        Ok(symbol)
    }
    pub(super) fn reserve_data(
        &mut self,
        symbol: SymbolId,
        bytes: u64,
        align: u64,
    ) -> Result<usize, Error> {
        let offset = align_up(
            u64::try_from(self.data.len()).map_err(|_| Error::Limit)?,
            align,
        )?;
        let end = usize::try_from(offset.checked_add(bytes).ok_or(Error::Limit)?)
            .map_err(|_| Error::Limit)?;
        self.data
            .try_reserve(end.saturating_sub(self.data.len()))
            .map_err(|_| Error::Limit)?;
        self.data.resize(end, 0);
        self.define(ImageSymbol::Program(symbol), Section::Data, offset);
        usize::try_from(offset).map_err(|_| Error::Limit)
    }
    pub(super) fn address_item(&mut self, offset: usize, target: SymbolId, addend: i64) {
        self.data_fixups.push(DataFixup {
            section: Section::Data,
            offset,
            target: ImageSymbol::Program(target),
            addend,
        });
    }
    pub(super) fn definition(&self, symbol: ImageSymbol) -> Definition {
        match symbol {
            ImageSymbol::Program(id) => self.symbols[id.index()],
            ImageSymbol::Local(id) => self.locals[id.0 as usize],
        }
    }
    pub(super) fn local_definitions(&self) -> impl Iterator<Item = (ImageSymbol, Definition)> + '_ {
        (0..)
            .zip(&self.locals)
            .map(|(i, &d)| (ImageSymbol::Local(LocalSymbol(i)), d))
    }
    pub(super) fn code_fixups(&self) -> &[CodeFixup] {
        &self.fixups
    }
    pub(super) fn data_fixups(&self) -> &[DataFixup] {
        &self.data_fixups
    }
    pub(super) fn add_data_fixup(
        &mut self,
        section: Section,
        offset: u64,
        target: ImageSymbol,
        addend: i64,
    ) -> Result<(), Error> {
        self.data_fixups.push(DataFixup {
            section,
            offset: usize::try_from(offset).map_err(|_| Error::Limit)?,
            target,
            addend,
        });
        Ok(())
    }
    fn bytes_mut(&mut self, section: Section) -> Result<&mut Vec<u8>, Error> {
        match section {
            Section::Text => Ok(&mut self.text),
            Section::Rodata => Ok(&mut self.rodata),
            Section::Data => Ok(&mut self.data),
            _ => Err(invariant("image section has no stored bytes")),
        }
    }
    pub(super) fn reserve(
        &mut self,
        section: Section,
        bytes: u64,
        align: u64,
    ) -> Result<u64, Error> {
        if section == Section::Bss {
            let at = align_up(self.bss, align)?;
            self.bss = at.checked_add(bytes).ok_or(Error::Limit)?;
            return Ok(at);
        }
        let buffer = self.bytes_mut(section)?;
        let at = align_up(
            u64::try_from(buffer.len()).map_err(|_| Error::Limit)?,
            align,
        )?;
        let end = usize::try_from(at.checked_add(bytes).ok_or(Error::Limit)?)
            .map_err(|_| Error::Limit)?;
        buffer
            .try_reserve(end.saturating_sub(buffer.len()))
            .map_err(|_| Error::Limit)?;
        buffer.resize(end, 0);
        Ok(at)
    }
    fn address(&self, symbol: ImageSymbol, layout: &executable::Layout) -> Result<u64, WriteError> {
        let definition = match symbol {
            ImageSymbol::Program(id) => self.symbols[id.index()],
            ImageSymbol::Local(id) => self.locals[id.0 as usize],
        };
        let base = match definition.section {
            Section::Undefined => {
                return Err(WriteError::Undefined);
            }
            Section::Text => layout.text_address,
            Section::Rodata => layout.rodata_address,
            Section::Data => layout.data_address,
            Section::Bss => layout.bss_address,
        };
        Ok(base.wrapping_add(definition.offset))
    }
    pub(super) fn write(
        &mut self,
        path: &Path,
        entry: ImageSymbol,
        permissions: Option<&std::fs::Permissions>,
    ) -> Result<(), WriteError> {
        let layout = executable::Layout::new(self)?;
        for fixup in &self.fixups {
            let target = self
                .address(fixup.target, &layout)?
                .wrapping_add_signed(i64::from(fixup.addend));
            let pc = layout
                .text_address
                .wrapping_add(u64::from(fixup.offset) + u64::from(fixup.end));
            let relative = i64::from_le_bytes(target.wrapping_sub(pc).to_le_bytes());
            let value = i32::try_from(relative)
                .map_err(|_| Error::Diagnostic("PC-relative reference out of range".into()))?;
            let offset = fixup.offset as usize;
            self.text
                .get_mut(offset..offset + 4)
                .ok_or_else(|| invariant("code fixup outside section"))?
                .copy_from_slice(&value.to_le_bytes());
        }
        for index in 0..self.data_fixups.len() {
            let fixup = &self.data_fixups[index];
            let value = self
                .address(fixup.target, &layout)?
                .wrapping_add_signed(fixup.addend);
            let section = fixup.section;
            let begin = fixup.offset;
            let end = begin.checked_add(8).ok_or(Error::Limit)?;
            self.bytes_mut(section)?
                .get_mut(begin..end)
                .ok_or_else(|| invariant("data fixup outside section"))?
                .copy_from_slice(&value.to_le_bytes());
        }
        let entry = self.address(entry, &layout)?;
        executable::write(path, &layout, self, entry, permissions).map_err(WriteError::Other)
    }
}
