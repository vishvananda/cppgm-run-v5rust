//! Course object compilation, reachability, ordered placement and references. See NOTICE.
use super::{
    global_data,
    image::{Image, ImageSymbol, Section as ImageSection, align_up},
    object::{
        self, Binding, Definition, Group, Name, Object, Relocation, RelocationKind, SectionId,
        SectionKind, Symbol, SymbolKind,
    },
    runtime::{self, Symbols},
};
use crate::{
    lowir::{
        Error,
        model::{DataItem, GlobalInit, NameId, Program, SymbolDetail, SymbolId},
        vocabulary::{Binding as SourceBinding, Role, Storage},
    },
    types::floating::FloatingScanner,
};
use std::collections::HashMap;
#[derive(Clone, Copy, Default)]
struct Placement {
    section: Option<SectionId>,
    offset: u64,
    size: u64,
}
#[derive(Clone, Copy)]
struct RangeId(std::num::NonZeroU32);
impl RangeId {
    fn new(index: usize) -> Result<Self, Error> {
        u32::try_from(index)
            .ok()
            .and_then(|i| i.checked_add(1))
            .and_then(std::num::NonZeroU32::new)
            .map(Self)
            .ok_or(Error::Limit)
    }
    fn index(self) -> usize {
        (self.0.get() - 1) as usize
    }
}
struct CodeRange {
    symbol: SymbolId,
    bytes: std::ops::Range<usize>,
    section: Option<SectionId>,
    base: u64,
    dropped: bool,
}
struct Pending {
    section: SectionId,
    offset: u64,
    target: ImageSymbol,
    addend: i64,
}
pub(crate) struct ObjectBuilder {
    level: u8,
    out: Object,
    image: Image,
    runtime: Symbols,
    ranges: Vec<CodeRange>,
    pending: Vec<Pending>,
    placements: Vec<Placement>,
    object_symbols: Vec<Option<object::SymbolId>>,
    by_name: HashMap<NameId, object::SymbolId>,
    shared: [Option<SectionId>; 4],
    own_names: [Option<Name>; 4],
    named: HashMap<NameId, SectionId>,
    grouped: Vec<SymbolId>,
    addresses: Vec<global_data::Address>,
}
fn kind_slot(kind: SectionKind) -> Result<usize, Error> {
    match kind {
        SectionKind::Text => Ok(0),
        SectionKind::Rodata => Ok(1),
        SectionKind::Data => Ok(2),
        SectionKind::Bss => Ok(3),
        _ => Err(Error::Construction("invalid shared course section".into())),
    }
}
fn section_name(kind: SectionKind) -> &'static [u8] {
    match kind {
        SectionKind::Text => b".text",
        SectionKind::Rodata => b".rodata",
        SectionKind::Data => b".data",
        SectionKind::Bss => b".bss",
        _ => b"",
    }
}
fn target_align(n: u64, align: u64) -> u64 {
    n.wrapping_add(align - 1) & !(align - 1)
}
fn resize(bytes: &mut Vec<u8>, length: u64, fill: u8) -> Result<(), Error> {
    let length = usize::try_from(length).map_err(|_| Error::Limit)?;
    bytes
        .try_reserve(length.saturating_sub(bytes.len()))
        .map_err(|_| Error::Limit)?;
    bytes.resize(length, fill);
    Ok(())
}
pub(crate) fn compile(program: &mut Program, path: Vec<u8>, level: u8) -> Result<Object, Error> {
    let mut compiler = if let Some(sink) = program.take_definition_sink() {
        *sink
            .into_any()
            .downcast::<ObjectBuilder>()
            .map_err(|_| Error::Construction("wrong object definition sink".into()))?
    } else {
        ObjectBuilder::new(level)?
    };
    compiler.out.path = path;
    compiler.encode(program, level)?;
    compiler.prune(program)?;
    compiler.place_text(program)?;
    compiler.place_globals(program)?;
    compiler.place_startup(program)?;
    compiler.add_symbols(program)?;
    compiler.add_relocations(program)?;
    for index in 0..compiler.grouped.len() {
        let symbol = compiler.grouped[index];
        let signature = compiler.object_symbol(program, ImageSymbol::Program(symbol))?;
        let section = compiler.placements[symbol.index()]
            .section
            .ok_or(Error::Limit)?;
        let _ = compiler.out.add_group(Group {
            signature,
            sections: vec![section],
        })?;
    }
    Ok(compiler.out)
}
impl ObjectBuilder {
    pub(crate) fn new(level: u8) -> Result<Self, Error> {
        let mut image = Image::new(0);
        let runtime = Symbols::imports(&mut image)?;
        Ok(Self {
            level,
            out: Object::new(Vec::new()),
            image,
            runtime,
            ranges: Vec::new(),
            pending: Vec::new(),
            placements: Vec::new(),
            object_symbols: Vec::new(),
            by_name: HashMap::new(),
            shared: [None; 4],
            own_names: [None; 4],
            named: HashMap::new(),
            grouped: Vec::new(),
            addresses: Vec::new(),
        })
    }
    fn append(&mut self, program: &Program, symbol: SymbolId, level: u8) -> Result<(), Error> {
        self.image.reserve_symbols(program.symbols().len());
        let function = program
            .function(program.symbol(symbol))
            .ok_or(Error::Limit)?;
        if !super::jump_thunk::emit(program, symbol, function, &mut self.image)? {
            let mir = super::value_selection::select(program, symbol, function, level)?;
            let _ = mir.emit(&mut self.image, Some(self.runtime))?;
        }
        let start = usize::try_from(self.image.definition(ImageSymbol::Program(symbol)).offset)
            .map_err(|_| Error::Limit)?;
        self.ranges.push(CodeRange {
            symbol,
            bytes: start..self.image.text.len(),
            section: None,
            base: 0,
            dropped: false,
        });
        Ok(())
    }

    fn encode(&mut self, program: &mut Program, level: u8) -> Result<(), Error> {
        self.image.reserve_symbols(program.symbols().len());
        for index in 0..program.publications().len() {
            let symbol = program.publications()[index];
            if program
                .function(program.symbol(symbol))
                .is_none_or(|f| f.body.is_none())
            {
                continue;
            }
            self.append(program, symbol, level)?;
            program.release_body(symbol)?;
        }
        // A declared TLS wrapper is a definition of the object's address in course mode.
        for index in 0..program.symbols().len() {
            let symbol = program
                .find_symbol(program.symbols()[index].name)
                .ok_or(Error::Limit)?;
            let Some(function) = program.function(program.symbol(symbol)) else {
                continue;
            };
            if !program.function_defined(symbol)
                && let Some(object) = function.metadata.tls_for
            {
                super::runtime::emit_tls_wrapper(
                    &mut self.image,
                    ImageSymbol::Program(symbol),
                    ImageSymbol::Program(object),
                )?;
                let start =
                    usize::try_from(self.image.definition(ImageSymbol::Program(symbol)).offset)
                        .map_err(|_| Error::Limit)?;
                self.ranges.push(CodeRange {
                    symbol,
                    bytes: start..self.image.text.len(),
                    section: None,
                    base: 0,
                    dropped: false,
                });
            }
        }
        Ok(())
    }
    fn local_function(program: &Program, symbol: SymbolId) -> bool {
        let metadata = program
            .symbol_metadata(program.symbol(symbol))
            .copied()
            .unwrap_or_default();
        metadata.role.is_none()
            && metadata.flags & 4 == 0
            && (metadata.binding == Some(SourceBinding::Internal)
                || !program.function_defined(symbol))
    }
    fn prune(&mut self, program: &Program) -> Result<(), Error> {
        let mut range_of = vec![None; program.symbols().len()];
        let mut first = Vec::with_capacity(self.ranges.len() + 1);
        let mut next = 0;
        for (index, range) in self.ranges.iter_mut().enumerate() {
            range_of[range.symbol.index()] = Some(RangeId::new(index)?);
            while next < self.image.code_fixups().len()
                && (self.image.code_fixups()[next].offset as usize) < range.bytes.start
            {
                next += 1;
            }
            first.push(next);
            range.dropped = true;
        }
        first.push(self.image.code_fixups().len());
        let mut work = Vec::new();
        let reach = |symbol: SymbolId, ranges: &mut [CodeRange], work: &mut Vec<RangeId>| {
            if let Some(range) = range_of[symbol.index()]
                && ranges[range.index()].dropped
            {
                ranges[range.index()].dropped = false;
                work.push(range);
            }
        };
        for index in 0..self.ranges.len() {
            let symbol = self.ranges[index].symbol;
            if !Self::local_function(program, symbol) {
                reach(symbol, &mut self.ranges, &mut work);
            }
        }
        for global in program.globals_in_publication_order() {
            match &global.init {
                GlobalInit::Address(symbol, _) => reach(*symbol, &mut self.ranges, &mut work),
                GlobalInit::Data(items) => {
                    for item in items {
                        if let DataItem::Address(_, symbol, _) = *item {
                            reach(symbol, &mut self.ranges, &mut work);
                        }
                    }
                }
                _ => {}
            }
        }
        for alias in program.aliases() {
            reach(alias.target, &mut self.ranges, &mut work);
        }
        while let Some(range) = work.pop() {
            let index = range.index();
            for fixup in &self.image.code_fixups()[first[index]..first[index + 1]] {
                if (fixup.offset as usize) >= self.ranges[index].bytes.end {
                    break;
                }
                if let ImageSymbol::Program(symbol) = fixup.target {
                    reach(symbol, &mut self.ranges, &mut work);
                }
            }
        }
        Ok(())
    }
    fn comdat(program: &Program, symbol: SymbolId) -> bool {
        program
            .symbol_metadata(program.symbol(symbol))
            .is_some_and(|m| m.binding == Some(SourceBinding::Weak) && m.flags & 64 == 0)
    }
    fn section(
        &mut self,
        program: &Program,
        kind: SectionKind,
        symbol: SymbolId,
    ) -> Result<SectionId, Error> {
        let index = kind_slot(kind)?;
        if Self::comdat(program, symbol) {
            let name = if let Some(name) = self.own_names[index] {
                name
            } else {
                let name = self.out.add_name(section_name(kind))?;
                self.own_names[index] = Some(name);
                name
            };
            self.grouped.push(symbol);
            self.out.add_named_section(name, kind, 1)
        } else {
            self.shared_section(kind)
        }
    }
    fn shared_section(&mut self, kind: SectionKind) -> Result<SectionId, Error> {
        let index = kind_slot(kind)?;
        if let Some(section) = self.shared[index] {
            return Ok(section);
        }
        let section = self.out.add_section(section_name(kind), kind, 1)?;
        self.shared[index] = Some(section);
        Ok(section)
    }
    fn place_text(&mut self, program: &Program) -> Result<(), Error> {
        let locals = self.image.local_definitions().count();
        let count = program
            .symbols()
            .len()
            .checked_add(locals)
            .ok_or(Error::Limit)?;
        self.placements.resize(count, Placement::default());
        for index in 0..self.ranges.len() {
            if self.ranges[index].dropped {
                continue;
            }
            let symbol = self.ranges[index].symbol;
            let section = self.section(program, SectionKind::Text, symbol)?;
            let output = self.out.section_mut(section);
            output.align = 16;
            let base = align_up(
                u64::try_from(output.bytes.len()).map_err(|_| Error::Limit)?,
                16,
            )?;
            resize(&mut output.bytes, base, 0xcc)?;
            output
                .bytes
                .extend_from_slice(&self.image.text[self.ranges[index].bytes.clone()]);
            output.size = u64::try_from(output.bytes.len()).map_err(|_| Error::Limit)?;
            self.ranges[index].section = Some(section);
            self.ranges[index].base = base;
            self.placements[symbol.index()] = Placement {
                section: Some(section),
                offset: base,
                size: u64::try_from(self.ranges[index].bytes.len()).map_err(|_| Error::Limit)?,
            };
        }
        if !self.image.rodata.is_empty() {
            let section = self.shared_section(SectionKind::Rodata)?;
            let output = self.out.section_mut(section);
            output.bytes.extend_from_slice(&self.image.rodata);
            output.size = u64::try_from(output.bytes.len()).map_err(|_| Error::Limit)?;
            output.align = 16;
            for (symbol, definition) in self.image.local_definitions() {
                if definition.section == ImageSection::Rodata {
                    self.placements[symbol.slot(program.symbols().len())] = Placement {
                        section: Some(section),
                        offset: definition.offset,
                        size: 0,
                    };
                }
            }
        }
        Ok(())
    }
    fn place_globals(&mut self, program: &Program) -> Result<(), Error> {
        let mut scanner = FloatingScanner::default();
        for index in 0..program.publications().len() {
            let symbol = program.publications()[index];
            let Some(global) = program
                .global(program.symbol(symbol))
                .filter(|g| !matches!(g.init, GlobalInit::Declaration))
            else {
                continue;
            };
            let metadata = global.metadata;
            let (size, align) = global_data::layout(global)?;
            let mut kind = if metadata.storage == Some(Storage::ReadOnly) {
                SectionKind::Rodata
            } else if global_data::is_zero(global, &mut scanner)? {
                SectionKind::Bss
            } else {
                SectionKind::Data
            };
            let section = if let Some(name) = metadata.section {
                kind = if metadata.storage == Some(Storage::ReadOnly) {
                    SectionKind::Rodata
                } else {
                    SectionKind::Data
                };
                if let Some(&section) = self.named.get(&name) {
                    if kind == SectionKind::Data {
                        self.out.section_mut(section).kind = kind;
                    }
                    section
                } else {
                    let section = self.out.add_section(program.names.bytes(name), kind, 1)?;
                    self.named.insert(name, section);
                    section
                }
            } else {
                self.section(program, kind, symbol)?
            };
            kind = self.out.section(section).kind;
            let output = self.out.section_mut(section);
            output.align = output.align.max(u64::from(align));
            let at = target_align(output.size, u64::from(align));
            output.size = at.wrapping_add(size);
            self.placements[symbol.index()] = Placement {
                section: Some(section),
                offset: at,
                size,
            };
            if kind == SectionKind::Bss {
                continue;
            }
            resize(&mut output.bytes, output.size, 0)?;
            self.addresses.clear();
            let global = program.global(program.symbol(symbol)).ok_or(Error::Limit)?;
            global_data::write_global(
                global,
                &mut output.bytes,
                usize::try_from(at).map_err(|_| Error::Limit)?,
                &mut self.addresses,
                &mut scanner,
            )?;
            for address in &self.addresses {
                self.pending.push(Pending {
                    section,
                    offset: at.wrapping_add(address.offset),
                    target: ImageSymbol::Program(address.target),
                    addend: address.addend,
                });
            }
        }
        Ok(())
    }
    fn place_startup(&mut self, program: &Program) -> Result<(), Error> {
        for index in 0..program.symbols().len() {
            let symbol = program
                .find_symbol(program.symbols()[index].name)
                .ok_or(Error::Limit)?;
            if !program.function_defined(symbol) {
                continue;
            }
            let role = program
                .symbol_metadata(program.symbol(symbol))
                .and_then(|m| m.role);
            let (name, kind) = match role {
                Some(Role::Init) => (&b".init_array"[..], SectionKind::InitArray),
                Some(Role::Fini) => (&b".fini_array"[..], SectionKind::FiniArray),
                _ => continue,
            };
            let section = self.out.add_section(name, kind, 8)?;
            self.out.section_mut(section).bytes = vec![0; 8];
            self.out.section_mut(section).size = 8;
            self.pending.push(Pending {
                section,
                offset: 0,
                target: ImageSymbol::Program(symbol),
                addend: 0,
            });
        }
        Ok(())
    }
    fn object_symbol(
        &mut self,
        program: &Program,
        image_symbol: ImageSymbol,
    ) -> Result<object::SymbolId, Error> {
        let slot = image_symbol.slot(program.symbols().len());
        if let Some(symbol) = self.object_symbols[slot] {
            return Ok(symbol);
        }
        let place = self.placements[slot];
        let mut symbol = Symbol {
            definition: place
                .section
                .map_or(Definition::Undefined, Definition::Section),
            value: place.offset,
            size: place.size,
            ..Symbol::default()
        };
        if let Some(local) = image_symbol.local_index() {
            if place.section.is_none() {
                let name = runtime::IMPORT_NAMES
                    .get(local)
                    .ok_or_else(|| Error::Construction("unnamed undefined object import".into()))?;
                symbol.name = self.out.add_name(name)?;
                symbol.binding = Binding::Global;
            } else {
                symbol.kind = SymbolKind::Object;
            }
            let added = self.out.add_symbol(symbol)?;
            self.object_symbols[slot] = Some(added);
            return Ok(added);
        }
        let ImageSymbol::Program(source) = image_symbol else {
            return Err(Error::Limit);
        };
        let from = program.symbol(source);
        let metadata = program.symbol_metadata(from).copied().unwrap_or_default();
        let defined = place.section.is_some();
        symbol.kind = if matches!(from.detail, SymbolDetail::Function(_)) {
            SymbolKind::Function
        } else {
            SymbolKind::Object
        };
        symbol.binding = if metadata.binding == Some(SourceBinding::Weak) {
            Binding::Weak
        } else {
            Binding::Global
        };
        if defined
            && (metadata.binding == Some(SourceBinding::Internal)
                || matches!(from.detail, SymbolDetail::Function(_))
                    && !program.function_defined(source))
        {
            symbol.binding = Binding::Local;
        }
        let name = metadata.object.unwrap_or(from.name);
        if symbol.binding != Binding::Local
            && let Some(&existing) = self.by_name.get(&name)
        {
            if defined && self.out.symbol(existing).definition == Definition::Undefined {
                symbol.name = self.out.symbol(existing).name;
                *self.out.symbol_mut(existing) = symbol;
            }
            self.object_symbols[slot] = Some(existing);
            return Ok(existing);
        }
        symbol.name = if symbol.binding == Binding::Local && metadata.object.is_none() {
            let mut spelling = vec![b'@'];
            spelling.extend_from_slice(program.names.bytes(name));
            self.out.add_name(&spelling)?
        } else {
            self.out.add_name(program.names.bytes(name))?
        };
        let added = self.out.add_symbol(symbol)?;
        if symbol.binding != Binding::Local {
            self.by_name.insert(name, added);
        }
        self.object_symbols[slot] = Some(added);
        Ok(added)
    }
    fn define_alias(
        &mut self,
        program: &Program,
        name: NameId,
        target: object::SymbolId,
    ) -> Result<(), Error> {
        let mut alias = *self.out.symbol(target);
        if let Some(&existing) = self.by_name.get(&name) {
            if self.out.symbol(existing).definition == Definition::Undefined {
                alias.name = self.out.symbol(existing).name;
                *self.out.symbol_mut(existing) = alias;
            }
            return Ok(());
        }
        alias.name = self.out.add_name(program.names.bytes(name))?;
        let id = self.out.add_symbol(alias)?;
        if alias.binding != Binding::Local {
            self.by_name.insert(name, id);
        }
        Ok(())
    }
    fn add_symbols(&mut self, program: &Program) -> Result<(), Error> {
        self.object_symbols.resize(self.placements.len(), None);
        for index in 0..self.ranges.len() {
            if !self.ranges[index].dropped {
                let _ =
                    self.object_symbol(program, ImageSymbol::Program(self.ranges[index].symbol))?;
            }
        }
        for index in 0..program.publications().len() {
            let symbol = program.publications()[index];
            if self.placements[symbol.index()].section.is_some() {
                let _ = self.object_symbol(program, ImageSymbol::Program(symbol))?;
            }
        }
        for index in 0..program.aliases().len() {
            let name = program.aliases()[index].object;
            let target = program.aliases()[index].target;
            let target = self.object_symbol(program, ImageSymbol::Program(target))?;
            self.define_alias(program, name, target)?;
        }
        for index in 0..program.symbols().len() {
            if self.placements[index].section.is_none() {
                continue;
            }
            let source = program
                .find_symbol(program.symbols()[index].name)
                .ok_or(Error::Limit)?;
            let metadata = program
                .symbol_metadata(program.symbol(source))
                .copied()
                .unwrap_or_default();
            let name = program.symbol(source).name;
            if metadata.flags & 1 != 0 && metadata.object.is_some_and(|object| object != name) {
                let target = self.object_symbol(program, ImageSymbol::Program(source))?;
                self.define_alias(program, name, target)?;
            }
            if metadata.role == Some(Role::Entry) {
                let target = self.object_symbol(program, ImageSymbol::Program(source))?;
                if self.out.symbol_name(target) != b"main" {
                    let mut entry = *self.out.symbol(target);
                    entry.name = self.out.add_name(b"main")?;
                    entry.binding = Binding::Global;
                    let _ = self.out.add_symbol(entry)?;
                }
            }
        }
        Ok(())
    }
    fn add_relocations(&mut self, program: &Program) -> Result<(), Error> {
        let mut range = 0;
        for index in 0..self.image.code_fixups().len() {
            let fixup = self.image.code_fixups()[index];
            while range < self.ranges.len() && fixup.offset as usize >= self.ranges[range].bytes.end
            {
                range += 1;
            }
            let code = self.ranges.get(range).ok_or_else(|| {
                Error::Construction("code reference outside every function".into())
            })?;
            if (fixup.offset as usize) < code.bytes.start {
                return Err(Error::Construction(
                    "code reference outside every function".into(),
                ));
            }
            if code.dropped {
                continue;
            }
            let section = code.section.ok_or(Error::Limit)?;
            let offset = code
                .base
                .checked_add(
                    u64::try_from(fixup.offset as usize - code.bytes.start)
                        .map_err(|_| Error::Limit)?,
                )
                .ok_or(Error::Limit)?;
            let symbol = self.object_symbol(program, fixup.target)?;
            self.out.section_mut(section).relocations.push(Relocation {
                offset,
                kind: if fixup.branch {
                    RelocationKind::Branch
                } else {
                    RelocationKind::Relative
                },
                symbol,
                addend: i64::from(fixup.addend) - i64::from(fixup.end),
            });
        }
        for index in 0..self.pending.len() {
            let target = self.pending[index].target;
            let symbol = self.object_symbol(program, target)?;
            let pending = &self.pending[index];
            self.out
                .section_mut(pending.section)
                .relocations
                .push(Relocation {
                    offset: pending.offset,
                    kind: RelocationKind::Absolute,
                    symbol,
                    addend: pending.addend,
                });
        }
        Ok(())
    }
}

impl crate::lowir::model::DefinitionSink for ObjectBuilder {
    fn append(&mut self, program: &Program, symbol: SymbolId) -> Result<(), Error> {
        self.append(program, symbol, self.level)
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
