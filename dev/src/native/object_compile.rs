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
/// Exception runtime and object ABI chosen by the output dialect.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObjectModel {
    Course,
    Host,
}
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
    cfi: std::ops::Range<usize>,
    lsda: std::ops::Range<usize>,
    types: std::ops::Range<usize>,
}
struct Pending {
    section: SectionId,
    offset: u64,
    target: ImageSymbol,
    addend: i64,
}
#[derive(Clone, Copy)]
enum UnwindTarget {
    Section(SectionId),
    Cell(usize),
}
struct Cell {
    target: ImageSymbol,
    offset: u64,
    symbol: Option<object::SymbolId>,
}
pub(crate) struct ObjectBuilder {
    level: u8,
    observation_options: super::ObservationOptions,
    observations: super::Observations,
    model: ObjectModel,
    cfi: Vec<u8>,
    unwind: Vec<(SectionId, u64, UnwindTarget, i64)>,
    lsda: Vec<u8>,
    type_fields: Vec<(u32, SymbolId)>,
    cells: Vec<Cell>,
    cell_index: HashMap<ImageSymbol, usize>,
    group_members: Vec<(SymbolId, SectionId)>,
    out: Object,
    image: Image,
    runtime: Symbols,
    ranges: Vec<CodeRange>,
    // Encoding and relocation ranges stay in physical order. Only completed
    // definition placement waits for the unit's inert-destructor release.
    order: Vec<RangeId>,
    held: Vec<RangeId>,
    pending: Vec<Pending>,
    placements: Vec<Placement>,
    object_symbols: Vec<Option<object::SymbolId>>,
    by_name: HashMap<NameId, object::SymbolId>,
    host_names: HashMap<NameId, NameId>,
    shared: [Option<SectionId>; 6],
    own_names: [Option<Name>; 6],
    named: HashMap<NameId, SectionId>,
    grouped: Vec<SymbolId>,
    relro: Option<SectionId>,
    relro_name: Option<Name>,
    addresses: Vec<global_data::Address>,
}
fn kind_slot(kind: SectionKind) -> Result<usize, Error> {
    match kind {
        SectionKind::Text => Ok(0),
        SectionKind::Rodata => Ok(1),
        SectionKind::Data => Ok(2),
        SectionKind::Bss => Ok(3),
        SectionKind::TlsData => Ok(4),
        SectionKind::TlsBss => Ok(5),
        _ => Err(Error::Construction("invalid shared course section".into())),
    }
}
fn section_name(kind: SectionKind) -> &'static [u8] {
    match kind {
        SectionKind::Text => b".text",
        SectionKind::Rodata => b".rodata",
        SectionKind::Data => b".data",
        SectionKind::Bss => b".bss",
        SectionKind::TlsData => b".tdata",
        SectionKind::TlsBss => b".tbss",
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
    compile_observed(program, path, level).map(|(object, _)| object)
}
pub(crate) fn compile_observed(
    program: &mut Program,
    path: Vec<u8>,
    level: u8,
) -> Result<(Object, super::Observations), Error> {
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
    compiler.observations.statistics.pruned_functions = crate::support::telemetry::observe(|| {
        compiler.ranges.iter().filter(|range| range.dropped).count()
    });
    compiler.place_text(program)?;
    compiler.place_globals(program)?;
    compiler.place_startup(program)?;
    compiler.place_unwind(program)?;
    compiler.add_symbols(program)?;
    compiler.add_relocations(program)?;
    compiler.add_unwind_relocations(program)?;
    let mut member = 0;
    for index in 0..compiler.grouped.len() {
        let symbol = compiler.grouped[index];
        let signature = compiler.object_symbol(program, ImageSymbol::Program(symbol))?;
        let section = compiler.placements[symbol.index()]
            .section
            .ok_or(Error::Limit)?;
        let mut sections = vec![section];
        while let Some(&(owner, section)) = compiler.group_members.get(member) {
            if owner != symbol {
                break;
            }
            sections.push(section);
            member += 1;
        }
        let _ = compiler.out.add_group(Group {
            signature,
            sections,
        })?;
    }
    if member != compiler.group_members.len() {
        return Err(Error::Construction(
            "an LSDA of a function without its group".into(),
        ));
    }

    Ok((compiler.out, compiler.observations))
}
impl ObjectBuilder {
    pub(crate) fn new(level: u8) -> Result<Self, Error> {
        Self::with_model(level, ObjectModel::Course)
    }
    pub(crate) fn with_model(level: u8, model: ObjectModel) -> Result<Self, Error> {
        Self::with_census(level, model, false)
    }
    pub(crate) fn with_census(level: u8, model: ObjectModel, census: bool) -> Result<Self, Error> {
        Self::with_observations(
            level,
            model,
            super::ObservationOptions {
                census,
                clocks: false,
            },
        )
    }
    pub(crate) fn with_observations(
        level: u8,
        model: ObjectModel,
        observation_options: super::ObservationOptions,
    ) -> Result<Self, Error> {
        let mut image = Image::new(0);
        let runtime = Symbols::imports(&mut image)?;
        Ok(Self {
            level,
            observation_options,
            observations: super::Observations::default(),
            model,
            cfi: Vec::new(),
            unwind: Vec::new(),
            lsda: Vec::new(),
            type_fields: Vec::new(),
            cells: Vec::new(),
            cell_index: HashMap::new(),
            group_members: Vec::new(),
            out: Object::new(Vec::new()),
            image,
            runtime,
            ranges: Vec::new(),
            order: Vec::new(),
            held: Vec::new(),
            pending: Vec::new(),
            placements: Vec::new(),
            object_symbols: Vec::new(),
            by_name: HashMap::new(),
            host_names: HashMap::new(),
            shared: [None; 6],
            own_names: [None; 6],
            named: HashMap::new(),
            grouped: Vec::new(),
            relro: None,
            relro_name: None,
            addresses: Vec::new(),
        })
    }
    fn append(&mut self, program: &Program, symbol: SymbolId, level: u8) -> Result<(), Error> {
        self.image.reserve_symbols(program.symbols().len());
        let function = program
            .function(program.symbol(symbol))
            .ok_or(Error::Limit)?;
        let mut facts = super::host_unwind::Frame::default();
        let lsda_start = self.lsda.len();
        let types_start = self.type_fields.len();
        if let Some(thunk) = program.jump_thunk(symbol) {
            super::jump_thunk::emit(symbol, thunk, &mut self.image)?;
        } else {
            let clock = (self.observation_options.clocks || self.observation_options.census)
                .then(crate::support::telemetry::Clock::now);
            let mir = super::value_selection::select_for_object(
                program, symbol, function, level, self.model,
            )?;
            self.observations.statistics.selected(
                &mir,
                clock.map_or(0.0, |start| start.elapsed().as_secs_f64() * 1000.0),
            )?;
            #[cfg(feature = "telemetry")]
            if self.observation_options.census {
                super::census::append(program, &mir, &mut self.observations.census);
            }
            let clock = clock.map(|_| crate::support::telemetry::Clock::now());
            let emission = super::emission::emit_with_frame(
                &mir,
                &mut self.image,
                Some(self.runtime),
                (self.model == ObjectModel::Host).then_some(&mut facts),
            )?;
            self.observations.statistics.encoded(
                emission,
                clock.map_or(0.0, |start| start.elapsed().as_secs_f64() * 1000.0),
            )?;
            if self.model == ObjectModel::Host {
                let lsda = super::lsda::encode(&facts, &mir.eh)?;
                self.observations.statistics.landings += mir.eh.landings.len();
                self.observations.statistics.lsdas += usize::from(!lsda.bytes.is_empty());
                self.lsda.extend(lsda.bytes);
                self.type_fields.extend(lsda.types);
            }
        }
        let start = usize::try_from(self.image.definition(ImageSymbol::Program(symbol)).offset)
            .map_err(|_| Error::Limit)?;
        let cfi_start = self.cfi.len();
        if self.model == ObjectModel::Host {
            self.cfi
                .extend(facts.encode(self.lsda.len() != lsda_start)?);
        }
        self.ranges.push(CodeRange {
            symbol,
            bytes: start..self.image.text.len(),
            section: None,
            base: 0,
            dropped: false,
            cfi: cfi_start..self.cfi.len(),
            lsda: lsda_start..self.lsda.len(),
            types: types_start..self.type_fields.len(),
        });
        self.order.push(RangeId::new(self.ranges.len() - 1)?);
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
                    cfi: 0..0,
                    lsda: 0..0,
                    types: 0..0,
                });
                self.order.push(RangeId::new(self.ranges.len() - 1)?);
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
        let defined = program.function_defined(symbol)
            || program
                .global(program.symbol(symbol))
                .is_some_and(|global| !matches!(global.init, GlobalInit::Declaration));
        defined
            && program
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
        for position in 0..self.order.len() {
            let index = self.order[position].index();
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
    fn relro_section(&mut self, program: &Program, symbol: SymbolId) -> Result<SectionId, Error> {
        if Self::comdat(program, symbol) {
            let name = if let Some(name) = self.relro_name {
                name
            } else {
                let name = self.out.add_name(b".data.rel.ro")?;
                self.relro_name = Some(name);
                name
            };
            self.grouped.push(symbol);
            self.out.add_named_section(name, SectionKind::Data, 1)
        } else if let Some(section) = self.relro {
            Ok(section)
        } else {
            let section = self
                .out
                .add_section(b".data.rel.ro", SectionKind::Data, 1)?;
            self.relro = Some(section);
            Ok(section)
        }
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
            let relro = self.model == ObjectModel::Host
                && metadata.storage == Some(Storage::ReadOnly)
                && match &global.init {
                    GlobalInit::Address(..) => true,
                    GlobalInit::Data(items) => items
                        .iter()
                        .any(|item| matches!(item, DataItem::Address(..))),
                    _ => false,
                };
            let mut kind = if metadata.storage == Some(Storage::ReadOnly) && !relro {
                SectionKind::Rodata
            } else if !relro && global_data::is_zero(global, &mut scanner)? {
                SectionKind::Bss
            } else {
                SectionKind::Data
            };
            if self.model == ObjectModel::Host && metadata.storage == Some(Storage::ThreadLocal) {
                kind = if kind == SectionKind::Bss {
                    SectionKind::TlsBss
                } else {
                    SectionKind::TlsData
                };
            }
            let section = if let Some(name) = metadata.section {
                kind = if metadata.storage == Some(Storage::ReadOnly) && !relro {
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
            } else if relro {
                self.relro_section(program, symbol)?
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
            if matches!(kind, SectionKind::Bss | SectionKind::TlsBss) {
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
    fn cell(&mut self, target: ImageSymbol) -> Result<usize, Error> {
        if let Some(&index) = self.cell_index.get(&target) {
            return Ok(index);
        }
        let data = self.shared_section(SectionKind::Data)?;
        let section = self.out.section_mut(data);
        section.align = section.align.max(8);
        let offset = align_up(section.size, 8)?;
        section.size = offset.checked_add(8).ok_or(Error::Limit)?;
        resize(&mut section.bytes, section.size, 0)?;
        self.pending.push(Pending {
            section: data,
            offset,
            target,
            addend: 0,
        });
        let index = self.cells.len();
        self.cells.push(Cell {
            target,
            offset,
            symbol: None,
        });
        self.cell_index.insert(target, index);
        Ok(index)
    }
    fn place_unwind(&mut self, program: &Program) -> Result<(), Error> {
        if self.model != ObjectModel::Host || self.ranges.is_empty() {
            return Ok(());
        }
        let any_lsda = self.ranges.iter().any(|r| !r.dropped && !r.lsda.is_empty());
        let plain = self.ranges.iter().any(|r| !r.dropped && r.lsda.is_empty());
        let personality_cell = if any_lsda {
            Some(self.cell(self.runtime.personality)?)
        } else {
            None
        };
        let mut placed_lsda = vec![None; self.ranges.len()];
        let mut shared = None;
        let mut name = None;
        for position in 0..self.order.len() {
            let index = self.order[position].index();
            let range = &self.ranges[index];
            if range.dropped || range.lsda.is_empty() {
                continue;
            }
            let symbol = range.symbol;
            let lsda = range.lsda.clone();
            self.observations.statistics.unwind_bytes += lsda.len();
            let types = range.types.clone();
            let name = if let Some(name) = name {
                name
            } else {
                let added = self.out.add_name(b".gcc_except_table")?;
                name = Some(added);
                added
            };
            let section = if Self::comdat(program, symbol) {
                let id = self.out.add_named_section(name, SectionKind::Rodata, 4)?;
                self.group_members.push((symbol, id));
                id
            } else if let Some(shared) = shared {
                shared
            } else {
                let id = self.out.add_named_section(name, SectionKind::Rodata, 4)?;
                shared = Some(id);
                id
            };
            let data = self.out.section_mut(section);
            let offset = align_up(
                u64::try_from(data.bytes.len()).map_err(|_| Error::Limit)?,
                4,
            )?;
            resize(&mut data.bytes, offset, 0)?;
            data.bytes.extend_from_slice(&self.lsda[lsda]);
            data.size = u64::try_from(data.bytes.len()).map_err(|_| Error::Limit)?;
            placed_lsda[index] = Some((section, offset));
            for t in types {
                let (field, ty) = self.type_fields[t];
                let cell = self.cell(ImageSymbol::Program(ty))?;
                self.unwind.push((
                    section,
                    offset + u64::from(field),
                    UnwindTarget::Cell(cell),
                    0,
                ));
            }
        }
        let frame = self.out.add_section(b".eh_frame", SectionKind::Rodata, 8)?;
        let bytes = &mut self.out.section_mut(frame).bytes;
        if plain {
            let _ = super::host_unwind::cie(bytes, false)?;
        }
        let personality_cie = u32::try_from(bytes.len()).map_err(|_| Error::Limit)?;
        if let Some(cell) = personality_cell {
            let field = super::host_unwind::cie(bytes, true)?.ok_or(Error::Limit)?;
            self.unwind
                .push((frame, u64::from(field), UnwindTarget::Cell(cell), 0));
        }
        // Follow delivered definition order, retaining only final frame/table spans.
        for &id in &self.order {
            let range = &self.ranges[id.index()];
            if range.dropped {
                continue;
            }
            let lsda = placed_lsda[id.index()];
            let (field, lsda_field) = super::host_unwind::fde(
                bytes,
                u32::try_from(range.bytes.len()).map_err(|_| Error::Limit)?,
                &self.cfi[range.cfi.clone()],
                if lsda.is_some() { personality_cie } else { 0 },
                lsda.is_some(),
            )?;
            self.unwind.push((
                frame,
                u64::from(field),
                UnwindTarget::Section(range.section.ok_or(Error::Limit)?),
                i64::try_from(range.base).map_err(|_| Error::Limit)?,
            ));
            if let Some((section, offset)) = lsda {
                self.unwind.push((
                    frame,
                    u64::from(lsda_field.ok_or(Error::Limit)?),
                    UnwindTarget::Section(section),
                    i64::try_from(offset).map_err(|_| Error::Limit)?,
                ));
            }
        }
        self.observations.statistics.unwind_bytes += bytes.len();
        self.out.section_mut(frame).size = u64::try_from(bytes.len()).map_err(|_| Error::Limit)?;
        self.cfi.clear();
        self.lsda.clear();
        self.type_fields.clear();
        Ok(())
    }
    fn add_unwind_relocations(&mut self, program: &Program) -> Result<(), Error> {
        for index in 0..self.cells.len() {
            let target = self.object_symbol(program, self.cells[index].target)?;
            // The DW namespace and ref-cell marker form an ABI symbol.
            let mut name = b"DW.".to_vec();
            name.extend_from_slice(b"ref.");
            name.extend_from_slice(self.out.symbol_name(target));
            let name = self.out.add_name(&name)?;
            let symbol = self.out.add_symbol(Symbol {
                name,
                kind: SymbolKind::Object,
                definition: Definition::Section(self.shared[2].ok_or(Error::Limit)?),
                value: self.cells[index].offset,
                size: 8,
                ..Symbol::default()
            })?;
            self.cells[index].symbol = Some(symbol);
        }
        let mut symbols = vec![None; self.out.section_count()];
        for &(section, offset, target, addend) in &self.unwind {
            let symbol = match target {
                UnwindTarget::Cell(index) => self.cells[index].symbol.ok_or(Error::Limit)?,
                UnwindTarget::Section(target) => {
                    if let Some(symbol) = symbols[target.index()] {
                        symbol
                    } else {
                        let symbol = self.out.add_symbol(Symbol {
                            kind: SymbolKind::Section,
                            definition: Definition::Section(target),
                            ..Symbol::default()
                        })?;
                        symbols[target.index()] = Some(symbol);
                        symbol
                    }
                }
            };
            self.out.section_mut(section).relocations.push(Relocation {
                offset,
                kind: RelocationKind::Relative,
                symbol,
                addend,
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
        if self.model == ObjectModel::Host && !defined {
            symbol.kind = SymbolKind::None;
        }
        if self.model == ObjectModel::Host && metadata.storage == Some(Storage::ThreadLocal) {
            symbol.kind = SymbolKind::Tls;
        }
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
        let mut name = metadata.object.unwrap_or(from.name);
        if self.model == ObjectModel::Host {
            name = self.host_names.get(&name).copied().unwrap_or(name);
        }
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
    fn add_symbols(&mut self, program: &mut Program) -> Result<(), Error> {
        if self.model == ObjectModel::Host {
            for index in 0..program.symbols().len() {
                let source = &program.symbols()[index];
                let name = program
                    .symbol_metadata(source)
                    .and_then(|m| m.object)
                    .unwrap_or(source.name);
                let host = host_builtin_name(program, name)?;
                if host != name {
                    self.host_names.insert(name, host);
                }
            }
        }
        self.object_symbols.resize(self.placements.len(), None);
        for position in 0..self.order.len() {
            let index = self.order[position].index();
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
            let mut kind = match fixup.kind {
                super::x86::FixupKind::Relative if fixup.branch => RelocationKind::Branch,
                super::x86::FixupKind::Relative => RelocationKind::Relative,
                super::x86::FixupKind::Got => RelocationKind::Other(42),
                super::x86::FixupKind::TpOff => RelocationKind::Other(23),
                super::x86::FixupKind::GotTpOff => RelocationKind::Other(22),
            };
            if matches!(fixup.kind, super::x86::FixupKind::Got)
                && fixup.addend == 0
                && let ImageSymbol::Program(target) = fixup.target
                && self.placements[target.index()].section.is_some()
                && super::symbol_access::host_symbol_access(program, target)
                    == super::symbol_access::SymbolAccess::Direct
            {
                let at = usize::try_from(offset).map_err(|_| Error::Limit)?;
                let bytes = &mut self.out.section_mut(section).bytes;
                if at >= 3
                    && bytes[at - 3] & 0xf8 == 0x48
                    && bytes[at - 2] == 0x8b
                    && bytes[at - 1] & 0xc7 == 0x05
                {
                    bytes[at - 2] = 0x8d;
                    kind = RelocationKind::Relative;
                }
            }
            if self.model == ObjectModel::Host
                && !fixup.branch
                && matches!(fixup.kind, super::x86::FixupKind::Relative)
                && let ImageSymbol::Program(target) = fixup.target
                && self.placements[target.index()].section.is_none()
                && matches!(program.symbol(target).detail, SymbolDetail::Function(_))
            {
                let at = usize::try_from(offset).map_err(|_| Error::Limit)?;
                let bytes = &mut self.out.section_mut(section).bytes;
                if at < 3
                    || bytes[at - 3] & 0xf8 != 0x48
                    || bytes[at - 2] != 0x8d
                    || bytes[at - 1] & 0xc7 != 0x05
                    || fixup.addend != 0
                {
                    return Err(Error::Construction(
                        "an external function's address outside a lea".into(),
                    ));
                }
                bytes[at - 2] = 0x8b;
                kind = RelocationKind::Other(42);
            }
            self.out.section_mut(section).relocations.push(Relocation {
                offset,
                kind,
                symbol,
                addend: i64::from(fixup.addend)
                    - if matches!(fixup.kind, super::x86::FixupKind::TpOff) {
                        0
                    } else {
                        i64::from(fixup.end)
                    },
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
    fn reads_jump_thunks(&self) -> bool {
        true
    }
    fn append(&mut self, program: &Program, symbol: SymbolId, held: bool) -> Result<(), Error> {
        self.append(program, symbol, self.level)?;
        if held {
            let range = self.order.pop().ok_or(Error::Limit)?;
            self.held.push(range);
        }
        Ok(())
    }
    fn release_held(&mut self) {
        self.order.append(&mut self.held);
    }
    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

fn host_builtin_name(program: &mut Program, name: NameId) -> Result<NameId, Error> {
    let host: &[u8] = match program.names.bytes(name) {
        b"cppgm_builtin_strlen" => b"strlen",
        b"cppgm_builtin_memcpy" => b"memcpy",
        b"cppgm_builtin_memmove" => b"memmove",
        b"cppgm_builtin_memset" => b"memset",
        b"cppgm_builtin_memcmp" => b"memcmp",
        b"cppgm_builtin_operator_new" => b"_Znwm",
        b"cppgm_builtin_operator_new_array" => b"_Znam",
        b"cppgm_builtin_operator_delete" => b"_ZdlPv",
        b"cppgm_builtin_operator_delete_array" => b"_ZdaPv",
        _ => return Ok(name),
    };
    let host = std::str::from_utf8(host).map_err(|_| Error::Limit)?;
    program.names.intern(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{InstructionKind as I, Metadata},
        vocabulary::Type,
    };

    #[test]
    fn weak_declared_tls_wrapper_uses_shared_text() -> Result<(), Error> {
        let source = b"global @g : i64 [storage=thread_local] = 9\n\
            declare function @wrapper() -> ptr [binding=weak,tls_for=@g]\n\
            function @main() -> i64 [role=entry] {block ^entry:\n\
            %p = call ptr @wrapper()\n%x = load i64 %p\nreturn i64 %x}\n";
        for model in [ObjectModel::Course, ObjectModel::Host] {
            let mut program = crate::lowir::test_program(source)?;
            program.set_definition_sink(Box::new(ObjectBuilder::with_model(0, model)?))?;
            let object = compile(&mut program, Vec::new(), 0)?;
            assert!(object.groups().is_empty());
            let sections: Vec<_> = object
                .sections()
                .filter(|(_, section)| section.kind == SectionKind::Text)
                .collect();
            assert_eq!(sections.len(), 1);
            let wrapper = (1..object.symbol_count())
                .map(|index| object.symbol_id(index))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .find(|&symbol| object.symbol_name(symbol) == b"@wrapper")
                .ok_or(Error::Limit)?;
            assert_eq!(object.symbol(wrapper).binding, Binding::Local);
            assert_eq!(
                object.symbol(wrapper).definition,
                Definition::Section(sections[0].0)
            );
        }
        Ok(())
    }

    #[test]
    fn streamed_data_addresses_relax_only_for_final_direct_definitions() -> Result<(), Error> {
        use crate::lowir::model::{ForwardGlobal, Global, Value};
        for (defined, replaceable) in [(false, false), (true, false), (true, true)] {
            let mut program = Program::default();
            program
                .set_definition_sink(Box::new(ObjectBuilder::with_model(0, ObjectModel::Host)?))?;
            let name = program.names.intern("data")?;
            let source = program.add_source(std::path::Path::new(""))?;
            let data = program.reference(name, source, 0)?;
            program.set_forward_global(
                data,
                ForwardGlobal {
                    binding: Some(SourceBinding::Strong),
                    storage: None,
                    defined: false,
                },
            );
            let mut builder = FunctionBuilder::new(&mut program, "address", &[], Type::Ptr)?;
            builder.block("entry")?;
            builder.effect(I::Return(Type::Ptr, Some(Value::Symbol(data))))?;
            builder.finish()?;
            program.stream_last_definition(&mut std::io::sink())?;
            assert!(program.functions().iter().all(|f| f.body.is_none()));
            program.add_global(
                data,
                Global {
                    name,
                    ty: Some(Type::I32),
                    metadata: Metadata {
                        binding: Some(if replaceable {
                            SourceBinding::Weak
                        } else {
                            SourceBinding::Strong
                        }),
                        flags: if replaceable { 64 } else { 0 },
                        ..Metadata::default()
                    },
                    init: if defined {
                        GlobalInit::Zero
                    } else {
                        GlobalInit::Declaration
                    },
                },
            )?;
            let object = compile(&mut program, Vec::new(), 0)?;
            let (_, text) = object
                .sections()
                .find(|(_, s)| s.kind == SectionKind::Text)
                .ok_or(Error::Limit)?;
            let [relocation] = text.relocations.as_slice() else {
                return Err(Error::Limit);
            };
            let direct = defined && !replaceable;
            assert_eq!(relocation.kind.code(), if direct { 2 } else { 42 });
            let field = usize::try_from(relocation.offset).map_err(|_| Error::Limit)?;
            assert_eq!(text.bytes[field - 2], if direct { 0x8d } else { 0x8b });
        }
        Ok(())
    }

    #[test]
    fn host_lazy_array_index_observes_the_full_signed_parameter() -> Result<(), Error> {
        use crate::lowir::{
            model::{ForwardGlobal, Global, Value},
            vocabulary::Convert,
        };
        let mut program = Program::default();
        program.set_definition_sink(Box::new(ObjectBuilder::with_model(0, ObjectModel::Host)?))?;
        let name = program.names.intern("data")?;
        let source = program.add_source(std::path::Path::new(""))?;
        let data = program.reference(name, source, 0)?;
        program.set_forward_global(
            data,
            ForwardGlobal {
                binding: Some(SourceBinding::Strong),
                storage: None,
                defined: false,
            },
        );
        let mut builder =
            FunctionBuilder::new(&mut program, "read", &[("i", Type::I32)], Type::I32)?;
        builder.block("entry")?;
        let index = builder.parameter(0)?;
        let index = builder.value(
            I::Convert(Convert::SignExtend, Type::I64, Type::I32, index),
            "extended",
        )?;
        let base = builder.value(I::Addr(Value::Symbol(data)), "base")?;
        let address = builder.value(I::Index(Type::I32, None, base, index), "address")?;
        let value = builder.value(
            I::Load(crate::lowir::model::Facts::default(), Type::I32, address),
            "value",
        )?;
        builder.effect(I::Return(Type::I32, Some(value)))?;
        builder.finish()?;
        program.stream_last_definition(&mut std::io::sink())?;
        program.add_global(
            data,
            Global {
                name,
                ty: None,
                metadata: Metadata {
                    binding: Some(SourceBinding::Strong),
                    ..Metadata::default()
                },
                init: GlobalInit::Declaration,
            },
        )?;
        let object = compile(&mut program, Vec::new(), 0)?;
        let (_, text) = object
            .sections()
            .find(|(_, s)| s.kind == SectionKind::Text)
            .ok_or(Error::Limit)?;
        // Host int indices arrive with only their low 32 bits specified. The
        // lazy address consumes a 64-bit index, so entry must sign-extend it.
        assert!(
            text.bytes
                .starts_with(&[0x55, 0x48, 0x8b, 0xec, 0x48, 0x63, 0xff])
        );
        Ok(())
    }

    #[test]
    fn held_definitions_release_ir_before_ordered_object_placement() -> Result<(), Error> {
        let mut program = Program::default();
        program.set_definition_sink(Box::new(ObjectBuilder::new(0)?))?;
        for (name, held) in [("held", true), ("first", false), ("last", false)] {
            let mut builder = FunctionBuilder::new(&mut program, name, &[], Type::Void)?;
            builder.metadata(Metadata {
                binding: Some(SourceBinding::Weak),
                ..Metadata::default()
            });
            builder.block("entry")?;
            builder.effect(I::Return(Type::Void, None))?;
            builder.finish()?;
            if held {
                program.defer_last_definition()?;
            } else {
                program.stream_last_definition(&mut std::io::sink())?;
            }
            assert!(program.functions().iter().all(|f| f.body.is_none()));
            if name == "first" {
                program.release_held_definitions();
            }
        }
        program.release_held_definitions();
        let object = compile(&mut program, Vec::new(), 0)?;
        let names: Vec<_> = (0..object.symbol_count())
            .map(|index| object.symbol_id(index).map(|id| object.symbol_name(id)))
            .collect::<Result<_, _>>()?;
        assert_eq!(names, [b"".as_slice(), b"first", b"held", b"last"]);
        Ok(())
    }
}
