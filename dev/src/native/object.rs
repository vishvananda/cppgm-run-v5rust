//! Ordered course-object sections, symbol identities and relocations. See NOTICE.
use crate::lowir::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SectionId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct SymbolId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GroupId(u32);
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct Name(pub(super) u32);
impl SectionId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}
impl SymbolId {
    pub(super) fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SectionKind {
    #[default]
    Null,
    Text,
    Rodata,
    Data,
    Bss,
    InitArray,
    FiniArray,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Binding {
    #[default]
    Local,
    Global,
    Weak,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum SymbolKind {
    #[default]
    None,
    Object,
    Function,
    Section,
    File,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Definition {
    Absolute,
    Common,
    #[default]
    Undefined,
    Section(SectionId),
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum RelocationKind {
    Absolute,
    Relative,
    Branch,
    Other(u32),
}
impl RelocationKind {
    pub(super) fn code(self) -> u32 {
        match self {
            Self::Absolute => 1,
            Self::Relative => 2,
            Self::Branch => 4,
            Self::Other(n) => n,
        }
    }
    pub(super) fn got(self) -> bool {
        matches!(self, Self::Other(9 | 41 | 42))
    }
}
#[derive(Clone, Copy)]
pub(super) struct Relocation {
    pub(super) offset: u64,
    pub(super) kind: RelocationKind,
    pub(super) symbol: SymbolId,
    pub(super) addend: i64,
}
#[derive(Default)]
pub(super) struct Section {
    pub(super) name: Name,
    pub(super) kind: SectionKind,
    pub(super) align: u64,
    pub(super) size: u64,
    pub(super) bytes: Vec<u8>,
    pub(super) relocations: Vec<Relocation>,
    pub(super) group: Option<GroupId>,
}
#[derive(Clone, Copy, Default)]
pub(super) struct Symbol {
    pub(super) name: Name,
    pub(super) binding: Binding,
    pub(super) kind: SymbolKind,
    pub(super) definition: Definition,
    pub(super) value: u64,
    pub(super) size: u64,
}
pub(super) struct Group {
    pub(super) signature: SymbolId,
    pub(super) sections: Vec<SectionId>,
}
pub(crate) struct Object {
    pub(super) path: Vec<u8>,
    names: Vec<u8>,
    sections: Vec<Section>,
    symbols: Vec<Symbol>,
    groups: Vec<Group>,
}
impl Object {
    pub(super) fn strings(&self) -> &[u8] {
        &self.names
    }
    pub(super) fn set_strings(&mut self, mut bytes: Vec<u8>) {
        bytes.push(0);
        self.names = bytes;
    }
    pub(super) fn new(path: Vec<u8>) -> Self {
        Self {
            path,
            names: vec![0],
            sections: vec![Section {
                align: 1,
                ..Section::default()
            }],
            symbols: vec![Symbol::default()],
            groups: Vec::new(),
        }
    }
    pub(super) fn add_name(&mut self, name: &[u8]) -> Result<Name, Error> {
        if name.is_empty() {
            return Ok(Name(0));
        }
        let at = u32::try_from(self.names.len()).map_err(|_| Error::Limit)?;
        let bytes = name.len().checked_add(1).ok_or(Error::Limit)?;
        u32::try_from(self.names.len().checked_add(bytes).ok_or(Error::Limit)?)
            .map_err(|_| Error::Limit)?;
        self.names.try_reserve(bytes).map_err(|_| Error::Limit)?;
        self.names.extend_from_slice(name);
        self.names.push(0);
        Ok(Name(at))
    }
    pub(super) fn name(&self, name: Name) -> &[u8] {
        let bytes = &self.names[name.0 as usize..];
        &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]
    }
    pub(super) fn symbol_name(&self, id: SymbolId) -> &[u8] {
        self.name(self.symbol(id).name)
    }
    pub(super) fn add_section(
        &mut self,
        name: &[u8],
        kind: SectionKind,
        align: u64,
    ) -> Result<SectionId, Error> {
        let name = self.add_name(name)?;
        self.add_named_section(name, kind, align)
    }
    pub(super) fn add_named_section(
        &mut self,
        name: Name,
        kind: SectionKind,
        align: u64,
    ) -> Result<SectionId, Error> {
        let id = SectionId(u32::try_from(self.sections.len()).map_err(|_| Error::Limit)?);
        self.sections.push(Section {
            name,
            kind,
            align,
            ..Section::default()
        });
        Ok(id)
    }
    pub(super) fn add_symbol(&mut self, symbol: Symbol) -> Result<SymbolId, Error> {
        let id = SymbolId(u32::try_from(self.symbols.len()).map_err(|_| Error::Limit)?);
        self.symbols.push(symbol);
        Ok(id)
    }
    pub(super) fn add_group(&mut self, group: Group) -> Result<GroupId, Error> {
        let id = GroupId(u32::try_from(self.groups.len()).map_err(|_| Error::Limit)?);
        for &section in &group.sections {
            self.section_mut(section).group = Some(id);
        }
        self.groups.push(group);
        Ok(id)
    }
    pub(super) fn section_id(&self, index: usize) -> Result<SectionId, Error> {
        if index >= self.sections.len() {
            return Err(Error::Construction("invalid section index".into()));
        }
        Ok(SectionId(u32::try_from(index).map_err(|_| Error::Limit)?))
    }
    pub(super) fn symbol_id(&self, index: usize) -> Result<SymbolId, Error> {
        if index >= self.symbols.len() {
            return Err(Error::Construction("invalid symbol index".into()));
        }
        Ok(SymbolId(u32::try_from(index).map_err(|_| Error::Limit)?))
    }
    pub(super) fn section(&self, id: SectionId) -> &Section {
        &self.sections[id.index()]
    }
    pub(super) fn section_mut(&mut self, id: SectionId) -> &mut Section {
        &mut self.sections[id.index()]
    }
    pub(super) fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.index()]
    }
    pub(super) fn symbol_mut(&mut self, id: SymbolId) -> &mut Symbol {
        &mut self.symbols[id.index()]
    }
    pub(super) fn sections(&self) -> impl Iterator<Item = (SectionId, &Section)> {
        (0..).zip(&self.sections).map(|(i, s)| (SectionId(i), s))
    }
    pub(super) fn symbols_mut(&mut self) -> &mut [Symbol] {
        &mut self.symbols
    }
    pub(super) fn groups(&self) -> &[Group] {
        &self.groups
    }
    pub(super) fn section_count(&self) -> usize {
        self.sections.len()
    }
    pub(super) fn symbol_count(&self) -> usize {
        self.symbols.len()
    }
}
