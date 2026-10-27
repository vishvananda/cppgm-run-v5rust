//! Checked ELF64 object decoding for the course's static linker. See NOTICE.
use super::object::{
    Binding, Definition, Group, Name, Object, Relocation, RelocationKind, SectionId, SectionKind,
    Symbol, SymbolKind,
};
use crate::lowir::Error;
use std::path::Path;
#[derive(Clone, Copy, Default)]
struct Header {
    name: u32,
    kind: u32,
    flags: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    align: u64,
    entry: u64,
}
struct Decoder<'a> {
    image: &'a [u8],
    out: Object,
    headers: Vec<Header>,
    sections: Vec<Option<SectionId>>,
    names: usize,
    symbols: Option<usize>,
    indexes: Option<usize>,
}
fn bad(path: &[u8], message: &str) -> Error {
    Error::ByteDiagnostic([path, b": ", message.as_bytes()].concat())
}
fn range(image: &[u8], offset: u64, size: u64) -> Option<&[u8]> {
    let start = usize::try_from(offset).ok()?;
    let length = usize::try_from(size).ok()?;
    image.get(start..start.checked_add(length)?)
}
fn word(image: &[u8], at: usize, width: usize) -> u64 {
    image[at..at + width]
        .iter()
        .enumerate()
        .fold(0, |n, (i, &b)| n | (u64::from(b) << (i * 8)))
}
fn word32(image: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([image[at], image[at + 1], image[at + 2], image[at + 3]])
}
impl<'a> Decoder<'a> {
    fn fail(&self, message: &str) -> Error {
        bad(&self.out.path, message)
    }
    fn bytes(&self, h: Header) -> Result<&'a [u8], Error> {
        range(self.image, h.offset, h.size)
            .ok_or_else(|| self.fail("section extends past the end of the file"))
    }
    fn headers(&mut self) -> Result<(), Error> {
        let table = word(self.image, 40, 8);
        if table == 0 {
            return Ok(());
        }
        let first = range(self.image, table, 64)
            .ok_or_else(|| self.fail("malformed section header table"))?;
        if word(self.image, 58, 2) != 64 {
            return Err(self.fail("malformed section header table"));
        }
        let mut count = word(self.image, 60, 2);
        let mut names = word(self.image, 62, 2);
        if count == 0 {
            count = word(first, 32, 8);
        }
        if names == 0xffff {
            names = word(first, 40, 4);
        }
        let bytes = count
            .checked_mul(64)
            .and_then(|size| range(self.image, table, size))
            .ok_or_else(|| self.fail("malformed section header table"))?;
        for (i, h) in bytes.as_chunks::<64>().0.iter().enumerate() {
            let header = Header {
                name: word32(h, 0),
                kind: word32(h, 4),
                flags: word(h, 8, 8),
                offset: word(h, 24, 8),
                size: word(h, 32, 8),
                link: word32(h, 40),
                info: word32(h, 44),
                align: word(h, 48, 8),
                entry: word(h, 56, 8),
            };
            if i != 0 && header.kind != 8 {
                let _ = self.bytes(header)?;
            }
            self.headers.push(header);
        }
        self.names = usize::try_from(names).map_err(|_| Error::Limit)?;
        if self.headers.get(self.names).is_none_or(|h| h.kind != 3) {
            return Err(self.fail("missing section name table"));
        }
        for (i, h) in self.headers.iter().enumerate().skip(1) {
            match h.kind {
                2 => {
                    if self.symbols.replace(i).is_some() {
                        return Err(self.fail("more than one symbol table"));
                    }
                }
                18 => self.indexes = Some(i),
                9 => return Err(self.fail("REL relocation tables are not used on x86-64")),
                _ => {}
            }
        }
        Ok(())
    }
    fn strings(&mut self) -> Result<(), Error> {
        let Some(index) = self.symbols else {
            return Ok(());
        };
        let h = self.headers[index];
        if h.entry != 24
            || self
                .headers
                .get(h.link as usize)
                .is_none_or(|s| s.kind != 3)
        {
            return Err(self.fail("malformed symbol table"));
        }
        let bytes = self.bytes(self.headers[h.link as usize])?;
        if bytes.first() != Some(&0) {
            return Err(self.fail("malformed string table"));
        }
        self.out.set_strings(bytes.to_vec());
        Ok(())
    }
    fn sections(&mut self) -> Result<(), Error> {
        self.sections = vec![None; self.headers.len()];
        if self.headers.is_empty() {
            return Ok(());
        }
        for i in 1..self.headers.len() {
            let h = self.headers[i];
            if matches!(h.kind, 2 | 18 | 3 | 4 | 17) {
                continue;
            }
            let table = self.bytes(self.headers[self.names])?;
            if u64::from(h.name) >= self.headers[self.names].size {
                return Err(self.fail("malformed section name"));
            }
            let text = table
                .get(h.name as usize..)
                .ok_or_else(|| self.fail("malformed section name"))?;
            let text = &text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())];
            let name = text.to_vec();
            let id = self.out.add_section(&name, SectionKind::Null, 1)?;
            self.sections[i] = Some(id);
        }
        for i in 1..self.headers.len() {
            let Some(id) = self.sections[i] else {
                continue;
            };
            let h = self.headers[i];
            let name = self.out.name(self.out.section(id).name);
            if h.flags & 2 == 0
                || matches!(h.kind, 7 | 0x7000_0001)
                || name == b".eh_frame"
                || name.starts_with(b".gcc_except_table")
            {
                continue;
            }
            if h.flags & 0x400 != 0 {
                return Err(self.fail("thread-local storage sections are not supported"));
            }
            let kind = match h.kind {
                1 => {
                    if h.flags & 4 != 0 {
                        SectionKind::Text
                    } else if h.flags & 1 != 0 {
                        SectionKind::Data
                    } else {
                        SectionKind::Rodata
                    }
                }
                8 => SectionKind::Bss,
                14 | 16 => SectionKind::InitArray,
                15 => SectionKind::FiniArray,
                _ => continue,
            };
            let align = h.align.max(1);
            if !align.is_power_of_two() {
                return Err(self.fail("section alignment is not a power of two"));
            }
            let bytes = if kind == SectionKind::Bss {
                Vec::new()
            } else {
                self.bytes(h)?.to_vec()
            };
            let section = self.out.section_mut(id);
            section.kind = kind;
            section.align = align;
            section.size = h.size;
            section.bytes = bytes;
        }
        Ok(())
    }
    fn symbols(&mut self) -> Result<(), Error> {
        let Some(index) = self.symbols else {
            return Ok(());
        };
        let h = self.headers[index];
        let count = h.size / 24;
        let names = self.headers[h.link as usize];
        let indexes = self
            .indexes
            .map(|i| self.headers[i])
            .filter(|h| h.size >= count * 4);
        let bytes = self.bytes(h)?;
        for (i, e) in bytes.as_chunks::<24>().0.iter().enumerate().skip(1) {
            let name = word32(e, 0);
            if u64::from(name) >= names.size {
                return Err(self.fail("malformed symbol name"));
            }
            let binding = match e[4] >> 4 {
                0 => Binding::Local,
                2 => Binding::Weak,
                _ => Binding::Global,
            };
            let ty = e[4] & 15;
            if ty == 6 {
                return Err(self.fail("thread-local symbols are not supported"));
            }
            if ty == 10 {
                return Err(self.fail("indirect function symbols are not supported"));
            }
            let kind = match ty {
                1 | 5 => SymbolKind::Object,
                2 => SymbolKind::Function,
                3 => SymbolKind::Section,
                4 => SymbolKind::File,
                _ => SymbolKind::None,
            };
            let mut at = word(e, 6, 2);
            let definition = if at >= 0xff00 && at != 0xffff {
                match at {
                    0xfff1 => Definition::Absolute,
                    0xfff2 => Definition::Common,
                    _ => return Err(self.fail("unsupported special section index")),
                }
            } else {
                if at == 0xffff {
                    let indexes =
                        indexes.ok_or_else(|| self.fail("missing extended section indexes"))?;
                    at = word(self.bytes(indexes)?, i * 4, 4);
                }
                let at = usize::try_from(at).map_err(|_| Error::Limit)?;
                let section = *self
                    .sections
                    .get(at)
                    .ok_or_else(|| self.fail("symbol section index out of range"))?;
                if at != 0 && section.is_none() && binding != Binding::Local {
                    return Err(self.fail("symbol defined in a non-content section"));
                }
                section.map_or(Definition::Undefined, Definition::Section)
            };
            let symbol = Symbol {
                name: Name(name),
                binding,
                kind,
                definition,
                value: word(e, 8, 8),
                size: word(e, 16, 8),
            };
            let _ = self.out.add_symbol(symbol)?;
        }
        Ok(())
    }
    fn groups(&mut self) -> Result<(), Error> {
        for i in 1..self.headers.len() {
            let h = self.headers[i];
            if h.kind != 17 {
                continue;
            }
            if h.size < 4
                || Some(h.link as usize) != self.symbols
                || h.info == 0
                || h.info as usize >= self.out.symbol_count()
            {
                return Err(self.fail("malformed section group"));
            }
            let bytes = self.bytes(h)?;
            if word(bytes, 0, 4) & 1 == 0 {
                continue;
            }
            let signature = self.out.symbol_id(h.info as usize)?;
            let mut sections = Vec::new();
            for e in bytes[4..].as_chunks::<4>().0 {
                let member = word32(e, 0) as usize;
                if member == 0 || member >= self.headers.len() {
                    return Err(self.fail("malformed section group member"));
                }
                if let Some(id) = self.sections[member] {
                    sections.push(id);
                }
            }
            let _ = self.out.add_group(Group {
                signature,
                sections,
            })?;
        }
        Ok(())
    }
    fn relocations(&mut self) -> Result<(), Error> {
        for i in 1..self.headers.len() {
            let h = self.headers[i];
            if h.kind != 4 {
                continue;
            }
            if h.info == 0 || h.info as usize >= self.headers.len() || h.entry != 24 {
                return Err(self.fail("malformed relocation table"));
            }
            let Some(id) = self.sections[h.info as usize] else {
                continue;
            };
            if self.out.section(id).kind == SectionKind::Null {
                continue;
            }
            if h.link as usize != self.symbols.unwrap_or(0) {
                return Err(self.fail("relocation table names another symbol table"));
            }
            let bytes = self.bytes(h)?;
            let mut relas = Vec::new();
            for e in bytes.as_chunks::<24>().0 {
                let offset = word(e, 0, 8);
                let symbol = word32(e, 12);
                let kind = word32(e, 8);
                if symbol as usize >= self.out.symbol_count() || offset > self.out.section(id).size
                {
                    return Err(self.fail("relocation out of range"));
                }
                let symbol = self.out.symbol_id(symbol as usize)?;
                let kind = match kind {
                    1 => RelocationKind::Absolute,
                    2 => RelocationKind::Relative,
                    4 => RelocationKind::Branch,
                    n => RelocationKind::Other(n),
                };
                let addend = i64::from_le_bytes(e[16..24].try_into().map_err(|_| Error::Limit)?);
                relas.push(Relocation {
                    offset,
                    kind,
                    symbol,
                    addend,
                });
            }
            self.out.section_mut(id).relocations.extend(relas);
        }
        Ok(())
    }
}
pub(crate) fn read(path: &Path) -> Result<Object, Error> {
    let image =
        std::fs::read(path).map_err(|_| Error::argument("cannot read ", path.as_os_str()))?;
    decode(&image, path.as_os_str().as_encoded_bytes().to_vec())
}
fn decode(image: &[u8], path: Vec<u8>) -> Result<Object, Error> {
    if !image.starts_with(b"\x7fELF") || image.len() < 64 {
        return Err(bad(&path, "not an ELF object"));
    }
    if image[4] != 2 || image[5] != 1 {
        return Err(bad(&path, "not a little-endian ELF64 object"));
    }
    if word(image, 16, 2) != 1 {
        return Err(bad(&path, "not a relocatable object"));
    }
    if word(image, 18, 2) != 62 {
        return Err(bad(&path, "not an x86-64 object"));
    }
    let mut decoder = Decoder {
        image,
        out: Object::new(path),
        headers: Vec::new(),
        sections: Vec::new(),
        names: 0,
        symbols: None,
        indexes: None,
    };
    decoder.headers()?;
    decoder.strings()?;
    decoder.sections()?;
    decoder.symbols()?;
    decoder.groups()?;
    decoder.relocations()?;
    Ok(decoder.out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_symbol_table_retains_null_link_and_symbol() -> Result<(), Error> {
        let mut object = Object::new(Vec::new());
        let data = object.add_section(b".data", SectionKind::Data, 8)?;
        let null = object.symbol_id(0)?;
        let section = object.section_mut(data);
        section.bytes = vec![0; 8];
        section.size = 8;
        section.relocations.push(Relocation {
            offset: 0,
            kind: RelocationKind::Absolute,
            symbol: null,
            addend: 7,
        });
        let mut image = super::super::elf_writer::bytes(&object)?;
        let table = usize::try_from(word(&image, 40, 8)).map_err(|_| Error::Limit)?;
        let count = usize::try_from(word(&image, 60, 2)).map_err(|_| Error::Limit)?;
        let mut relocation = None;
        let mut strings = 0;
        for i in 1..count {
            let header = table + i * 64;
            match word(&image, header + 4, 4) {
                2 => image[header + 4..header + 8].copy_from_slice(&3_u32.to_le_bytes()),
                3 => strings = i,
                4 => {
                    image[header + 40..header + 44].fill(0);
                    relocation = Some(header);
                }
                _ => {}
            }
        }
        let decoded = decode(&image, b"no-symbol-table".to_vec())?;
        assert_eq!(decoded.symbol_count(), 1);
        let section = decoded.section(decoded.section_id(1)?);
        assert_eq!(section.relocations.len(), 1);
        assert_eq!(section.relocations[0].symbol.index(), 0);
        assert_eq!(section.relocations[0].addend, 7);
        let relocation = relocation.ok_or(Error::Limit)?;
        image[relocation + 32..relocation + 40].fill(0);
        let decoded = decode(&image, b"empty-relocations".to_vec())?;
        assert!(
            decoded
                .section(decoded.section_id(1)?)
                .relocations
                .is_empty()
        );
        let strings = u32::try_from(strings).map_err(|_| Error::Limit)?;
        image[relocation + 40..relocation + 44].copy_from_slice(&strings.to_le_bytes());
        assert!(matches!(decode(&image, b"bad-link".to_vec()),
            Err(Error::ByteDiagnostic(bytes)) if bytes == b"bad-link: relocation table names another symbol table"));
        Ok(())
    }
    #[test]
    fn malformed_sizes_and_extended_indexes_return_diagnostics() {
        for length in 0..65 {
            assert!(decode(&vec![0; length], b"bad-object".to_vec()).is_err());
        }
        let mut header = vec![0; 64];
        header[..7].copy_from_slice(b"\x7fELF\x02\x01\x01");
        header[16] = 1;
        header[18] = 62;
        header[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        header[58] = 64;
        assert!(
            matches!(decode(&header, b"bad-object".to_vec()), Err(Error::ByteDiagnostic(bytes)) if bytes == b"bad-object: malformed section header table")
        );
    }
}
