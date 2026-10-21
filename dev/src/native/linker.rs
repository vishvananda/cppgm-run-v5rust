//! Course object symbol resolution, segment placement and relocation. See NOTICE.
use super::{
    executable,
    image::{Image, align_up},
    object::{Binding, Definition, Object, RelocationKind, SectionId, SectionKind, SymbolId},
};
use crate::lowir::Error;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};
const MAX_ALIGN: u64 = 1 << 24;
const MAX_SEGMENT: u64 = 1 << 46;
#[derive(Clone, Copy)]
struct ObjectId(u32);
impl ObjectId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy)]
struct GlobalId(u32);
impl GlobalId {
    fn index(self) -> usize {
        self.0 as usize
    }
}
#[derive(Clone, Copy)]
struct Reference {
    object: ObjectId,
    symbol: SymbolId,
}
#[derive(Clone, Copy)]
enum Bound {
    InitStart,
    InitEnd,
    FiniStart,
    FiniEnd,
    Got,
}
#[derive(Clone, Copy)]
enum State {
    Undefined,
    Weak(Reference),
    Strong(Reference),
    Linker(Bound),
}
struct Global {
    name: Arc<[u8]>,
    state: State,
    strong_reference: bool,
    referenced_from: Option<ObjectId>,
    address: u64,
}
#[derive(Clone, Copy)]
enum Segment {
    Text,
    Rodata,
    Data,
    Bss,
}
impl Segment {
    fn index(self) -> usize {
        match self {
            Self::Text => 0,
            Self::Rodata => 1,
            Self::Data => 2,
            Self::Bss => 3,
        }
    }
}
#[derive(Clone, Copy)]
struct Location {
    segment: Segment,
    offset: u64,
}
struct Linker {
    objects: Vec<Object>,
    names: HashMap<Arc<[u8]>, GlobalId>,
    groups: HashSet<Vec<u8>>,
    globals: Vec<Global>,
    global_of: Vec<Vec<Option<GlobalId>>>,
    kept: Vec<Vec<bool>>,
    locations: Vec<Vec<Option<Location>>>,
    image: Image,
    array_bounds: [u64; 4],
    got: u64,
    align: [u64; 4],
}
fn error(parts: &[&[u8]]) -> Error {
    Error::ByteDiagnostic(parts.concat())
}
fn object_error(object: &Object, message: &[u8]) -> Error {
    error(&[&object.path, b": ", message])
}
fn startup_order(name: &[u8]) -> u32 {
    if name == b".preinit_array" {
        return 0;
    }
    let Some(at) = name
        .iter()
        .rposition(|&b| b == b'.')
        .filter(|&at| at != 0 && at + 1 < name.len())
    else {
        return 0x20000;
    };
    let mut priority = 0_u32;
    for &c in &name[at + 1..] {
        if !c.is_ascii_digit() || priority > 0xffff {
            return 0x20000;
        }
        priority = priority * 10 + u32::from(c - b'0');
    }
    1 + priority.min(0xffff)
}
fn pad(buffer: &mut Vec<u8>, align: u64, fill: u8) -> Result<u64, Error> {
    let offset = align_up(
        u64::try_from(buffer.len()).map_err(|_| Error::Limit)?,
        align,
    )?;
    let count = usize::try_from(offset).map_err(|_| Error::Limit)?;
    buffer
        .try_reserve(count.saturating_sub(buffer.len()))
        .map_err(|_| Error::Limit)?;
    buffer.resize(count, fill);
    Ok(offset)
}
pub(super) fn link(
    objects: Vec<Object>,
    path: &Path,
    permissions: Option<&std::fs::Permissions>,
) -> Result<(), Error> {
    let mut linker = Linker {
        objects,
        names: HashMap::new(),
        groups: HashSet::new(),
        globals: Vec::new(),
        global_of: Vec::new(),
        kept: Vec::new(),
        locations: Vec::new(),
        image: Image::new(0),
        array_bounds: [0; 4],
        got: 0,
        align: [1; 4],
    };
    for index in 0..linker.objects.len() {
        linker.add_object(ObjectId(u32::try_from(index).map_err(|_| Error::Limit)?))?;
    }
    linker.add_runtime()?;
    linker.check_undefined()?;
    linker.layout()?;
    let layout = executable::Layout::with_alignment(&linker.image, linker.align)?;
    linker.assign_addresses(&layout)?;
    linker.relocate(&layout)?;
    let entry = linker
        .names
        .get(&b"_start"[..])
        .and_then(|id| {
            let global = &linker.globals[id.index()];
            (!matches!(global.state, State::Undefined)).then_some(global.address)
        })
        .ok_or_else(|| Error::Diagnostic("no process entry".into()))?;
    executable::write(path, &layout, &linker.image, entry, permissions)
}
impl Linker {
    fn add_object(&mut self, object: ObjectId) -> Result<(), Error> {
        let at = object.index();
        self.global_of.resize_with(self.objects.len(), Vec::new);
        self.kept.resize_with(self.objects.len(), Vec::new);
        self.locations.resize_with(self.objects.len(), Vec::new);
        let file = &self.objects[at];
        self.locations[at] = vec![None; file.section_count()];
        self.kept[at] = file
            .sections()
            .map(|(_, s)| s.kind != SectionKind::Null)
            .collect();
        for index in 0..file.groups().len() {
            let signature = self.objects[at].groups()[index].signature;
            let name = self.objects[at].symbol_name(signature);
            if self.groups.insert(name.to_vec()) {
                continue;
            }
            for member in 0..self.objects[at].groups()[index].sections.len() {
                let section = self.objects[at].groups()[index].sections[member];
                self.kept[at][section.index()] = false;
                let section = self.objects[at].section_mut(section);
                section.bytes = Vec::new();
                section.relocations = Vec::new();
            }
        }
        self.global_of[at] = vec![None; self.objects[at].symbol_count()];
        for index in 1..self.objects[at].symbol_count() {
            let id = self.objects[at].symbol_id(index)?;
            let symbol = *self.objects[at].symbol(id);
            if symbol.binding == Binding::Local {
                continue;
            }
            let name = self.objects[at].symbol_name(id);
            let global = if let Some(&id) = self.names.get(name) {
                id
            } else {
                let id = GlobalId(u32::try_from(self.globals.len()).map_err(|_| Error::Limit)?);
                let name: Arc<[u8]> = Arc::from(name);
                self.names.insert(Arc::clone(&name), id);
                self.globals.push(Global {
                    name,
                    state: State::Undefined,
                    strong_reference: false,
                    referenced_from: None,
                    address: 0,
                });
                id
            };
            self.global_of[at][index] = Some(global);
            if let Definition::Section(section) = symbol.definition
                && self.kept[at][section.index()]
            {
                let reference = Reference { object, symbol: id };
                let state = if symbol.binding == Binding::Weak {
                    State::Weak(reference)
                } else {
                    State::Strong(reference)
                };
                self.define(global, state)?;
            }
        }
        for section in 1..self.objects[at].section_count() {
            if !self.kept[at][section] {
                continue;
            }
            let id = self.objects[at].section_id(section)?;
            for relocation in &self.objects[at].section(id).relocations {
                let Some(global) = self.global_of[at][relocation.symbol.index()] else {
                    continue;
                };
                if self.objects[at].symbol(relocation.symbol).binding != Binding::Global {
                    continue;
                }
                let global = &mut self.globals[global.index()];
                global.strong_reference = true;
                if global.referenced_from.is_none() {
                    global.referenced_from = Some(object);
                }
            }
        }
        Ok(())
    }
    fn define(&mut self, id: GlobalId, state: State) -> Result<(), Error> {
        let global = &mut self.globals[id.index()];
        if let (State::Strong(old), State::Strong(new)) = (global.state, state) {
            return Err(error(&[
                b"duplicate symbol: ",
                &global.name,
                b" (defined in ",
                &self.objects[old.object.index()].path,
                b" and ",
                &self.objects[new.object.index()].path,
                b")",
            ]));
        }
        if matches!(state, State::Strong(_)) || matches!(global.state, State::Undefined) {
            global.state = state;
        }
        Ok(())
    }
    fn add_runtime(&mut self) -> Result<(), Error> {
        let wanted: Vec<_> = self
            .globals
            .iter()
            .filter(|g| {
                matches!(g.state, State::Undefined) && super::runtime_link::provides(&g.name)
            })
            .map(|g| g.name.as_ref())
            .collect();
        let objects = super::runtime_link::build(&wanted)?;
        let first = self.objects.len();
        self.objects.extend(objects);
        for index in first..self.objects.len() {
            self.add_object(ObjectId(u32::try_from(index).map_err(|_| Error::Limit)?))?;
        }
        for (name, bound) in [
            (&b"__cppgm_init_array_start"[..], Bound::InitStart),
            (&b"__cppgm_init_array_end"[..], Bound::InitEnd),
            (&b"__cppgm_fini_array_start"[..], Bound::FiniStart),
            (&b"__cppgm_fini_array_end"[..], Bound::FiniEnd),
            (&b"_GLOBAL_OFFSET_TABLE_"[..], Bound::Got),
        ] {
            if let Some(id) = self.names.get(name)
                && matches!(self.globals[id.index()].state, State::Undefined)
            {
                self.globals[id.index()].state = State::Linker(bound);
            }
        }
        Ok(())
    }
    fn check_undefined(&self) -> Result<(), Error> {
        let mut missing = Vec::new();
        let mut count = 0;
        let mut host = false;
        for global in &self.globals {
            if !matches!(global.state, State::Undefined) || !global.strong_reference {
                continue;
            }
            host |= global.name.as_ref() == b"__gxx_personality_v0"
                || global.name.as_ref() == b"_Unwind_Resume";
            count += 1;
            if count > 8 {
                continue;
            }
            if !missing.is_empty() {
                missing.extend_from_slice(b", ");
            }
            missing.extend_from_slice(&global.name);
            if let Some(object) = global.referenced_from {
                missing.extend_from_slice(b" (referenced from ");
                missing.extend_from_slice(&self.objects[object.index()].path);
                missing.push(b')');
            }
        }
        if count == 0 {
            return Ok(());
        }
        if count > 8 {
            missing.extend_from_slice(b", ...");
        }
        if host {
            missing.extend_from_slice(b"; an object with host EH tables needs the host C++ toolchain's link (for link mode, compile the source with cppgm++ -c to a *.obj course object)");
        }
        Err(error(&[
            if count > 1 {
                b"undefined symbols: "
            } else {
                b"undefined symbol: "
            },
            &missing,
        ]))
    }
    fn buffer(&mut self, segment: Segment) -> Option<&mut Vec<u8>> {
        match segment {
            Segment::Text => Some(&mut self.image.text),
            Segment::Rodata => Some(&mut self.image.rodata),
            Segment::Data => Some(&mut self.image.data),
            Segment::Bss => None,
        }
    }
    fn place(
        &mut self,
        object: ObjectId,
        section: SectionId,
        segment: Segment,
    ) -> Result<(), Error> {
        let at = object.index();
        let from = self.objects[at].section(section);
        let alignment = from.align;
        if alignment > MAX_ALIGN {
            return Err(object_error(
                &self.objects[at],
                format!("section alignment beyond {MAX_ALIGN}").as_bytes(),
            ));
        }
        self.align[segment.index()] = self.align[segment.index()].max(alignment);
        let offset = if matches!(segment, Segment::Bss) {
            self.image.bss = align_up(self.image.bss, alignment)?;
            if from.size > MAX_SEGMENT.wrapping_sub(self.image.bss) {
                return Err(object_error(
                    &self.objects[at],
                    b"zero-filled storage beyond the address space",
                ));
            }
            let offset = self.image.bss;
            self.image.bss = self.image.bss.wrapping_add(from.size);
            offset
        } else {
            let bytes = std::mem::take(&mut self.objects[at].section_mut(section).bytes);
            let buffer = self.buffer(segment).ok_or(Error::Limit)?;
            let offset = pad(
                buffer,
                alignment,
                if matches!(segment, Segment::Text) {
                    0xcc
                } else {
                    0
                },
            )?;
            buffer.extend_from_slice(&bytes);
            offset
        };
        self.locations[at][section.index()] = Some(Location { segment, offset });
        Ok(())
    }
    fn layout(&mut self) -> Result<(), Error> {
        let mut sizes = [0_u64; 3];
        for (object, file) in self.objects.iter().enumerate() {
            for (id, section) in file.sections().skip(1) {
                if !self.kept[object][id.index()] || section.align > MAX_ALIGN {
                    continue;
                }
                let slot = match section.kind {
                    SectionKind::Text => 0,
                    SectionKind::Rodata | SectionKind::InitArray | SectionKind::FiniArray => 1,
                    SectionKind::Data => 2,
                    _ => continue,
                };
                sizes[slot] = align_up(sizes[slot], section.align)?.wrapping_add(section.size);
            }
        }
        for (segment, size) in [
            (Segment::Text, sizes[0]),
            (Segment::Rodata, sizes[1].wrapping_add(16)),
            (Segment::Data, sizes[2]),
        ] {
            self.buffer(segment)
                .ok_or(Error::Limit)?
                .try_reserve(usize::try_from(size).map_err(|_| Error::Limit)?)
                .map_err(|_| Error::Limit)?;
        }
        for object in 0..self.objects.len() {
            let oid = ObjectId(u32::try_from(object).map_err(|_| Error::Limit)?);
            for index in 1..self.objects[object].section_count() {
                if !self.kept[object][index] {
                    continue;
                }
                let id = self.objects[object].section_id(index)?;
                let segment = match self.objects[object].section(id).kind {
                    SectionKind::Text => Segment::Text,
                    SectionKind::Rodata => Segment::Rodata,
                    SectionKind::Data => Segment::Data,
                    SectionKind::Bss => Segment::Bss,
                    _ => continue,
                };
                self.place(oid, id, segment)?;
            }
        }
        let mut order = Vec::new();
        for (array, kind) in [SectionKind::InitArray, SectionKind::FiniArray]
            .into_iter()
            .enumerate()
        {
            order.clear();
            for (object, file) in self.objects.iter().enumerate() {
                for (id, section) in file.sections().skip(1) {
                    if section.kind == kind && self.kept[object][id.index()] {
                        order.push((
                            startup_order(file.name(section.name)),
                            u32::try_from(object).map_err(|_| Error::Limit)?,
                            id,
                        ));
                    }
                }
            }
            order.sort_unstable_by_key(|(priority, object, section)| {
                (*priority, *object, section.index())
            });
            let _ = pad(&mut self.image.rodata, 8, 0)?;
            self.array_bounds[2 * array] =
                u64::try_from(self.image.rodata.len()).map_err(|_| Error::Limit)?;
            for &(_, object, section) in &order {
                self.place(ObjectId(object), section, Segment::Rodata)?;
            }
            self.array_bounds[2 * array + 1] =
                u64::try_from(self.image.rodata.len()).map_err(|_| Error::Limit)?;
        }
        self.got = pad(&mut self.image.data, 8, 0)?;
        Ok(())
    }
    fn segment_address(layout: &executable::Layout, segment: Segment) -> u64 {
        match segment {
            Segment::Text => layout.text_address,
            Segment::Rodata => layout.rodata_address,
            Segment::Data => layout.data_address,
            Segment::Bss => layout.bss_address,
        }
    }
    fn defined_address(
        &self,
        reference: Reference,
        layout: &executable::Layout,
    ) -> Result<u64, Error> {
        let file = &self.objects[reference.object.index()];
        let symbol = file.symbol(reference.symbol);
        let Definition::Section(section) = symbol.definition else {
            return Err(object_error(
                file,
                b"relocation against an undefined local symbol",
            ));
        };
        let location =
            self.locations[reference.object.index()][section.index()].ok_or_else(|| {
                object_error(file, b"relocation against a symbol in a discarded section")
            })?;
        Ok(Self::segment_address(layout, location.segment)
            .wrapping_add(location.offset)
            .wrapping_add(symbol.value))
    }
    fn target_address(
        &self,
        object: ObjectId,
        symbol: SymbolId,
        layout: &executable::Layout,
    ) -> Result<u64, Error> {
        if symbol.index() == 0 {
            return Ok(0);
        }
        if let Some(global) = self.global_of[object.index()][symbol.index()] {
            return Ok(self.globals[global.index()].address);
        }
        self.defined_address(Reference { object, symbol }, layout)
    }
    fn assign_addresses(&mut self, layout: &executable::Layout) -> Result<(), Error> {
        for index in 0..self.globals.len() {
            let address = match self.globals[index].state {
                State::Undefined => 0,
                State::Weak(reference) | State::Strong(reference) => {
                    self.defined_address(reference, layout)?
                }
                State::Linker(bound) => match bound {
                    Bound::Got => layout.data_address.wrapping_add(self.got),
                    Bound::InitStart => layout.rodata_address.wrapping_add(self.array_bounds[0]),
                    Bound::InitEnd => layout.rodata_address.wrapping_add(self.array_bounds[1]),
                    Bound::FiniStart => layout.rodata_address.wrapping_add(self.array_bounds[2]),
                    Bound::FiniEnd => layout.rodata_address.wrapping_add(self.array_bounds[3]),
                },
            };
            self.globals[index].address = address;
        }
        Ok(())
    }
    fn relocate(&mut self, layout: &executable::Layout) -> Result<(), Error> {
        for object in 0..self.objects.len() {
            let oid = ObjectId(u32::try_from(object).map_err(|_| Error::Limit)?);
            for section in 1..self.objects[object].section_count() {
                let Some(location) = self.locations[object][section] else {
                    continue;
                };
                let sid = self.objects[object].section_id(section)?;
                if self.objects[object].section(sid).relocations.is_empty() {
                    continue;
                }
                if matches!(location.segment, Segment::Bss) {
                    return Err(object_error(
                        &self.objects[object],
                        b"relocations in zero-filled storage",
                    ));
                }
                let size = self.objects[object].section(sid).size;
                for index in 0..self.objects[object].section(sid).relocations.len() {
                    let relocation = self.objects[object].section(sid).relocations[index];
                    if relocation.offset >= size {
                        return Err(object_error(
                            &self.objects[object],
                            b"relocation outside its section",
                        ));
                    }
                    let symbol = self.target_address(oid, relocation.symbol, layout)?;
                    let value = symbol.wrapping_add_signed(relocation.addend);
                    let place = Self::segment_address(layout, location.segment)
                        .wrapping_add(location.offset)
                        .wrapping_add(relocation.offset);
                    let (stored, bytes, fits) =
                        if matches!(relocation.kind, RelocationKind::Absolute) {
                            (value, 8, true)
                        } else {
                            let relative = value.wrapping_sub(place).cast_signed();
                            (relative.cast_unsigned(), 4, i32::try_from(relative).is_ok())
                        };
                    if bytes > size - relocation.offset {
                        return Err(object_error(
                            &self.objects[object],
                            b"relocation outside its section",
                        ));
                    }
                    if !fits {
                        return Err(error(&[
                            &self.objects[object].path,
                            b": relocation value out of range for ",
                            self.objects[object].symbol_name(relocation.symbol),
                        ]));
                    }
                    let offset = usize::try_from(
                        location
                            .offset
                            .checked_add(relocation.offset)
                            .ok_or(Error::Limit)?,
                    )
                    .map_err(|_| Error::Limit)?;
                    let bytes = usize::try_from(bytes).map_err(|_| Error::Limit)?;
                    self.buffer(location.segment)
                        .ok_or(Error::Limit)?
                        .get_mut(offset..offset.checked_add(bytes).ok_or(Error::Limit)?)
                        .ok_or(Error::Limit)?
                        .copy_from_slice(&stored.to_le_bytes()[..bytes]);
                }
            }
        }
        Ok(())
    }
}
