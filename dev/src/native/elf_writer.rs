//! ELF64 relocatable encoding in the frozen section and symbol order. See NOTICE.
use super::{
    image::align_up,
    object::{Binding, Definition, Object, SectionKind, SymbolKind},
};
use crate::lowir::Error;
use std::{collections::HashMap, path::Path};
#[derive(Default)]
struct Section<'a> {
    name: u32,
    kind: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entry: u64,
    bytes: &'a [u8],
    built: Vec<u8>,
}
fn put(out: &mut Vec<u8>, value: u64, width: usize) {
    out.extend_from_slice(&value.to_le_bytes()[..width]);
}
fn number(n: usize) -> Result<u32, Error> {
    u32::try_from(n).map_err(|_| Error::Limit)
}
struct Encoder<'a> {
    object: &'a Object,
    sections: Vec<Section<'a>>,
    names: Vec<u8>,
    contents: Vec<u32>,
    relas: Vec<u32>,
    symbols: Vec<u32>,
    section_names: HashMap<(bool, u32), u32>,
    symtab: u32,
    strtab: u32,
    shstrtab: u32,
    first_global: u32,
    extended: bool,
}
impl<'a> Encoder<'a> {
    fn name(&mut self, prefix: &[u8], text: &[u8]) -> Result<u32, Error> {
        let at = number(self.names.len())?;
        self.names.extend_from_slice(prefix);
        self.names.extend_from_slice(text);
        self.names.push(0);
        Ok(at)
    }
    fn section_name(&mut self, rela: bool, name: super::object::Name) -> Result<u32, Error> {
        if let Some(&known) = self.section_names.get(&(rela, name.0)) {
            return Ok(known);
        }
        let offset = self.name(if rela { b".rela" } else { b"" }, self.object.name(name))?;
        self.section_names.insert((rela, name.0), offset);
        Ok(offset)
    }
    fn new(object: &'a Object) -> Result<Self, Error> {
        let mut contents = vec![0; object.section_count()];
        let mut relas = contents.clone();
        let mut next = number(object.groups().len())?
            .checked_add(1)
            .ok_or(Error::Limit)?;
        for (_, slot) in contents.iter_mut().enumerate().skip(1) {
            *slot = next;
            next = next.checked_add(1).ok_or(Error::Limit)?;
        }
        for (id, section) in object.sections().skip(1) {
            if !section.relocations.is_empty() {
                relas[id.index()] = next;
                next = next.checked_add(1).ok_or(Error::Limit)?;
            }
        }
        let extended = next > 0xff00;
        let symtab = next;
        let strtab = next.checked_add(1).ok_or(Error::Limit)?;
        let shstrtab = next
            .checked_add(if extended { 3 } else { 2 })
            .ok_or(Error::Limit)?;
        let mut symbols = vec![0; object.symbol_count()];
        let mut next = 1_u32;
        for (i, slot) in symbols.iter_mut().enumerate().skip(1) {
            if object.symbol(object.symbol_id(i)?).binding == Binding::Local {
                *slot = next;
                next = next.checked_add(1).ok_or(Error::Limit)?;
            }
        }
        let first_global = next;
        for (i, slot) in symbols.iter_mut().enumerate().skip(1) {
            if object.symbol(object.symbol_id(i)?).binding != Binding::Local {
                *slot = next;
                next = next.checked_add(1).ok_or(Error::Limit)?;
            }
        }
        Ok(Self {
            object,
            sections: vec![Section::default()],
            names: vec![0],
            contents,
            relas,
            symbols,
            section_names: HashMap::new(),
            symtab,
            strtab,
            shstrtab,
            first_global,
            extended,
        })
    }
    fn groups(&mut self) -> Result<(), Error> {
        let mut name = None;
        for group in self.object.groups() {
            let name = if let Some(n) = name {
                n
            } else {
                let n = self.name(b".group", b"")?;
                name = Some(n);
                n
            };
            let mut s = Section {
                name,
                kind: 17,
                link: self.symtab,
                info: self.symbols[group.signature.index()],
                align: 4,
                entry: 4,
                ..Section::default()
            };
            put(&mut s.built, 1, 4);
            for member in &group.sections {
                put(&mut s.built, u64::from(self.contents[member.index()]), 4);
                if self.relas[member.index()] != 0 {
                    put(&mut s.built, u64::from(self.relas[member.index()]), 4);
                }
            }
            self.sections.push(s);
        }
        Ok(())
    }
    fn contents(&mut self) -> Result<(), Error> {
        for (_, from) in self.object.sections().skip(1) {
            let name = self.section_name(false, from.name)?;
            let kind = match from.kind {
                SectionKind::Bss | SectionKind::TlsBss => 8,
                SectionKind::InitArray => 14,
                SectionKind::FiniArray => 15,
                _ => 1,
            };
            let flags = match from.kind {
                SectionKind::Null => 0,
                SectionKind::Text => 6,
                SectionKind::Rodata => 2,
                _ => 3,
            } | if from.group.is_some() { 0x200 } else { 0 }
                | if matches!(from.kind, SectionKind::TlsData | SectionKind::TlsBss) {
                    0x400
                } else {
                    0
                };
            self.sections.push(Section {
                name,
                kind,
                flags,
                size: from.size,
                align: from.align,
                entry: if matches!(from.kind, SectionKind::InitArray | SectionKind::FiniArray) {
                    8
                } else {
                    0
                },
                bytes: if kind == 8 { &[] } else { &from.bytes },
                ..Section::default()
            });
        }
        Ok(())
    }
    fn relocations(&mut self) -> Result<(), Error> {
        for (id, from) in self.object.sections().skip(1) {
            if from.relocations.is_empty() {
                continue;
            }
            let mut s = Section {
                name: self.section_name(true, from.name)?,
                kind: 4,
                flags: 0x40 | if from.group.is_some() { 0x200 } else { 0 },
                link: self.symtab,
                info: self.contents[id.index()],
                align: 8,
                entry: 24,
                ..Section::default()
            };
            for r in &from.relocations {
                let kind = r.kind.code();
                put(&mut s.built, r.offset, 8);
                put(
                    &mut s.built,
                    (u64::from(self.symbols[r.symbol.index()]) << 32) | u64::from(kind),
                    8,
                );
                s.built.extend_from_slice(&r.addend.to_le_bytes());
            }
            self.sections.push(s);
        }
        Ok(())
    }
    fn symbols(&mut self) -> Result<(), Error> {
        let mut s = Section {
            name: self.name(b".symtab", b"")?,
            kind: 2,
            link: self.strtab,
            info: self.first_global,
            align: 8,
            entry: 24,
            built: vec![0; 24],
            ..Section::default()
        };
        let mut extended = Section::default();
        if self.extended {
            put(&mut extended.built, 0, 4);
        }
        let mut order = vec![0; self.symbols.len()];
        for (i, &at) in self.symbols.iter().enumerate().skip(1) {
            order[at as usize] = i;
        }
        for &id in order.iter().skip(1) {
            let symbol = self.object.symbol(self.object.symbol_id(id)?);
            let (index, escaped) = match symbol.definition {
                Definition::Undefined => (0, false),
                Definition::Absolute => (0xfff1, false),
                Definition::Common => (0xfff2, false),
                Definition::Section(id) => {
                    let n = self.contents[id.index()];
                    (n, n >= 0xff00)
                }
            };
            put(&mut s.built, u64::from(symbol.name.0), 4);
            let bind = match symbol.binding {
                Binding::Local => 0,
                Binding::Global => 1,
                Binding::Weak => 2,
            };
            let kind = match symbol.kind {
                SymbolKind::None => 0,
                SymbolKind::Object => 1,
                SymbolKind::Function => 2,
                SymbolKind::Section => 3,
                SymbolKind::File => 4,
                SymbolKind::Tls => 6,
            };
            s.built.push((bind << 4) | kind);
            s.built.push(0);
            put(
                &mut s.built,
                if escaped { 0xffff } else { u64::from(index) },
                2,
            );
            put(&mut s.built, symbol.value, 8);
            put(&mut s.built, symbol.size, 8);
            if self.extended {
                put(
                    &mut extended.built,
                    if escaped { u64::from(index) } else { 0 },
                    4,
                );
            }
        }
        self.sections.push(s);
        let name = self.name(b".strtab", b"")?;
        self.sections.push(Section {
            name,
            kind: 3,
            align: 1,
            bytes: self.object.strings(),
            ..Section::default()
        });
        if self.extended {
            extended.name = self.name(b".symtab_shndx", b"")?;
            extended.kind = 18;
            extended.link = self.symtab;
            extended.align = 4;
            extended.entry = 4;
            self.sections.push(extended);
        }
        Ok(())
    }
    fn finish(mut self) -> Result<Vec<u8>, Error> {
        self.groups()?;
        self.contents()?;
        self.relocations()?;
        self.symbols()?;
        let name = self.name(b".shstrtab", b"")?;
        let stack = self.name(b".note.GNU-stack", b"")?;
        self.sections.push(Section {
            name,
            kind: 3,
            align: 1,
            built: self.names,
            ..Section::default()
        });
        self.sections.push(Section {
            name: stack,
            kind: 1,
            align: 1,
            ..Section::default()
        });
        let mut position = 64;
        for s in self.sections.iter_mut().skip(1) {
            if s.kind != 8 {
                s.size =
                    u64::try_from(s.bytes.len().max(s.built.len())).map_err(|_| Error::Limit)?;
            }
            position = align_up(position, s.align.max(1))?;
            s.offset = position;
            if s.kind != 8 {
                position = position.checked_add(s.size).ok_or(Error::Limit)?;
            }
        }
        let table = align_up(position, 8)?;
        let count = number(self.sections.len())?;
        if count >= 0xff00 {
            self.sections[0].size = u64::from(count);
        }
        if self.shstrtab >= 0xff00 {
            self.sections[0].link = self.shstrtab;
        }
        let mut out = b"\x7fELF\x02\x01\x01\0\0\0\0\0\0\0\0\0".to_vec();
        for (n, width) in [
            (1, 2),
            (62, 2),
            (1, 4),
            (0, 8),
            (0, 8),
            (table, 8),
            (0, 4),
            (64, 2),
            (0, 2),
            (0, 2),
            (64, 2),
            (if count >= 0xff00 { 0 } else { u64::from(count) }, 2),
            (
                if self.shstrtab >= 0xff00 {
                    0xffff
                } else {
                    u64::from(self.shstrtab)
                },
                2,
            ),
        ] {
            put(&mut out, n, width);
        }
        for s in self.sections.iter().skip(1) {
            if s.kind == 8 {
                continue;
            }
            out.resize(usize::try_from(s.offset).map_err(|_| Error::Limit)?, 0);
            out.extend_from_slice(s.bytes);
            out.extend_from_slice(&s.built);
        }
        out.resize(usize::try_from(table).map_err(|_| Error::Limit)?, 0);
        for s in &self.sections {
            for (n, width) in [
                (u64::from(s.name), 4),
                (u64::from(s.kind), 4),
                (s.flags, 8),
                (0, 8),
                (s.offset, 8),
                (s.size, 8),
                (u64::from(s.link), 4),
                (u64::from(s.info), 4),
                (s.align, 8),
                (s.entry, 8),
            ] {
                put(&mut out, n, width);
            }
        }
        Ok(out)
    }
}
pub(crate) fn write(object: &Object, path: &Path) -> Result<(), Error> {
    let image = bytes(object)?;
    std::fs::write(path, image).map_err(|_| Error::argument("cannot write ", path.as_os_str()))
}
pub(crate) fn bytes(object: &Object) -> Result<Vec<u8>, Error> {
    Encoder::new(object)?.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lowir::{
            builder::FunctionBuilder,
            model::{InstructionKind as I, Metadata, Program},
            vocabulary::{Binding as SourceBinding, Type},
        },
        native::object_compile::{ObjectBuilder, ObjectModel, compile},
    };
    // Compare whole objects with independent physical encoding orders. Besides
    // FDE bytes this checks their section-symbol identities and relocations;
    // equal-sized FDEs alone would conceal the wrong function association.
    #[test]
    fn host_unwind_follows_released_definitions_after_pruning() -> Result<(), Error> {
        fn object(order: &[(&str, bool)]) -> Result<Vec<u8>, Error> {
            let mut program = Program::default();
            program
                .set_definition_sink(Box::new(ObjectBuilder::with_model(0, ObjectModel::Host)?))?;
            for &(name, held) in order {
                let mut builder = FunctionBuilder::new(&mut program, name, &[], Type::I64)?;
                builder.metadata(Metadata {
                    binding: Some(if name == "unused" {
                        SourceBinding::Internal
                    } else {
                        SourceBinding::Weak
                    }),
                    ..Metadata::default()
                });
                builder.block("entry")?;
                let value = builder.integer(if name == "held" { 0x1234_5678_9abc } else { 7 })?;
                builder.effect(I::Return(Type::I64, Some(value)))?;
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
            Encoder::new(&object)?.finish()
        }
        let delayed = object(&[("first", false), ("held", false), ("last", false)])?;
        for order in [
            vec![("held", true), ("first", false), ("last", false)],
            vec![
                ("unused", true),
                ("held", true),
                ("first", false),
                ("last", false),
            ],
            vec![
                ("held", true),
                ("unused", false),
                ("first", false),
                ("last", false),
            ],
        ] {
            assert_eq!(object(&order)?, delayed);
        }
        Ok(())
    }
}
