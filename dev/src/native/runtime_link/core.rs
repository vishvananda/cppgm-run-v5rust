//! Machine-encoded course entry, system routines and fundamental RTTI. See NOTICE.
use super::super::{
    image::{Image, ImageSymbol, Section as ImageSection},
    object::{
        Binding, Definition, Object, Relocation, RelocationKind, SectionId, SectionKind, Symbol,
        SymbolKind,
    },
    runtime::{self, DispatcherSymbols},
    x86::{Assembler, Register as R, Rm},
};
use super::{Member, Members, Support, VTABLES, member_of, runtime_type, type_name};
use crate::lowir::Error;
use crate::support::hash::HashMap;
use std::sync::Arc;
struct Core {
    image: Image,
    names: Vec<Arc<[u8]>>,
    known: HashMap<Arc<[u8]>, ImageSymbol>,
}
impl Core {
    fn new() -> Self {
        Self {
            image: Image::new(0),
            names: Vec::new(),
            known: HashMap::default(),
        }
    }
    fn named(&mut self, name: &[u8]) -> Result<ImageSymbol, Error> {
        if let Some(&symbol) = self.known.get(name) {
            return Ok(symbol);
        }
        let symbol = self.image.local()?;
        let name: Arc<[u8]> = Arc::from(name);
        self.names.push(Arc::clone(&name));
        self.known.insert(name, symbol);
        Ok(symbol)
    }
    fn define_data(
        &mut self,
        symbol: ImageSymbol,
        section: ImageSection,
        size: u64,
        align: u64,
    ) -> Result<(), Error> {
        let at = self.image.reserve(section, size, align)?;
        self.image.define(symbol, section, at);
        Ok(())
    }
    fn begin(&mut self, a: &mut Assembler, base: u64, symbol: ImageSymbol) -> Result<(), Error> {
        while !(base + u64::from(a.position()?)).is_multiple_of(16) {
            a.byte(0xcc);
        }
        self.image
            .define(symbol, ImageSection::Text, base + u64::from(a.position()?));
        Ok(())
    }
    fn object(self) -> Result<Object, Error> {
        let mut out = Object::new(b"<runtime library>".to_vec());
        let text = out.add_section(b".text", SectionKind::Text, 16)?;
        let rodata = out.add_section(b".rodata", SectionKind::Rodata, 16)?;
        let data = out.add_section(b".data", SectionKind::Data, 16)?;
        let bss = out.add_section(b".bss", SectionKind::Bss, 64)?;
        for (id, bytes) in [
            (text, &self.image.text),
            (rodata, &self.image.rodata),
            (data, &self.image.data),
        ] {
            out.section_mut(id).bytes.extend_from_slice(bytes);
            out.section_mut(id).size = u64::try_from(bytes.len()).map_err(|_| Error::Limit)?;
        }
        out.section_mut(bss).size = self.image.bss;
        let mut symbols = Vec::new();
        for (id, from) in self.image.local_definitions() {
            let section: Option<SectionId> = match from.section {
                ImageSection::Text => Some(text),
                ImageSection::Rodata => Some(rodata),
                ImageSection::Data => Some(data),
                ImageSection::Bss => Some(bss),
                ImageSection::Undefined => None,
            };
            let mut symbol = Symbol {
                definition: section.map_or(Definition::Undefined, Definition::Section),
                value: from.offset,
                kind: if from.section == ImageSection::Text {
                    SymbolKind::Function
                } else {
                    SymbolKind::Object
                },
                ..Symbol::default()
            };
            let at = id.local_index().ok_or(Error::Limit)?;
            if let Some(name) = self.names.get(at).filter(|name| !name.is_empty()) {
                symbol.name = out.add_name(name)?;
                symbol.binding = if section.is_none() {
                    Binding::Global
                } else {
                    Binding::Weak
                };
                if section.is_none() {
                    symbol.kind = SymbolKind::None;
                }
            }
            symbols.push(out.add_symbol(symbol)?);
        }
        for fixup in self.image.code_fixups() {
            out.section_mut(text).relocations.push(Relocation {
                offset: u64::from(fixup.offset),
                kind: RelocationKind::Relative,
                symbol: symbols[fixup.target.local_index().ok_or(Error::Limit)?],
                addend: i64::from(fixup.addend) - i64::from(fixup.end),
            });
        }
        for fixup in self.image.data_fixups() {
            let section = if fixup.section == ImageSection::Rodata {
                rodata
            } else {
                data
            };
            out.section_mut(section).relocations.push(Relocation {
                offset: u64::try_from(fixup.offset).map_err(|_| Error::Limit)?,
                kind: RelocationKind::Absolute,
                symbol: symbols[fixup.target.local_index().ok_or(Error::Limit)?],
                addend: fixup.addend,
            });
        }
        Ok(out)
    }
    fn entry(&mut self) -> Result<(), Error> {
        let base = u64::try_from(self.image.text.len()).map_err(|_| Error::Limit)?;
        let mut a = Assembler::default();
        let start = self.named(b"_start")?;
        let exit = self.named(b"__cppgm_exit")?;
        let head = self.named(b"__cppgm_atexit_head")?;
        self.define_data(head, ImageSection::Bss, 8, 8)?;
        let thread = self.named(b"__cppgm_thread_atexit_head")?;
        self.define_data(thread, ImageSection::Bss, 8, 8)?;
        let dso = self.named(b"__dso_handle")?;
        self.define_data(dso, ImageSection::Bss, 8, 8)?;
        self.begin(&mut a, base, start)?;
        let next = a.label()?;
        let done = a.label()?;
        a.mov_rr(R::R12, R::RSP)?;
        let init = self.named(b"__cppgm_init_array_start")?;
        a.lea(R::RBX, Rm::rip(init, 0))?;
        let end = self.named(b"__cppgm_init_array_end")?;
        a.lea(R::R13, Rm::rip(end, 0))?;
        a.bind(next);
        a.alu_rr(7, R::RBX, Rm::Reg(R::R13), 8)?;
        a.branch(Some(3), done, true);
        a.call_rm(Rm::mem(R::RBX, 0))?;
        a.alu_ri(0, Rm::Reg(R::RBX), 8, 8)?;
        a.branch(None, next, true);
        a.bind(done);
        a.load(R::RDI, Rm::mem(R::R12, 0), 8, false)?;
        a.lea(R::RSI, Rm::mem(R::R12, 8))?;
        a.lea(
            R::RDX,
            Rm::Memory {
                base: Some(R::R12),
                index: Some(R::RDI),
                scale: 8,
                disp: 16,
                target: None,
            },
        )?;
        let main = self.named(b"main")?;
        a.call_symbol(main)?;
        a.op(0, false, 0x8b, 1, R::RDI.code(), Rm::Reg(R::RAX), false, 0)?;
        a.call_symbol(exit)?;
        a.resolve()?;
        self.begin(&mut a, base, exit)?;
        let fini = a.label()?;
        let done = a.label()?;
        a.push(R::RBP);
        a.op(0, false, 0x8b, 1, R::R12.code(), Rm::Reg(R::RDI), false, 0)?;
        for list in [thread, head] {
            let pop = a.label()?;
            let empty = a.label()?;
            a.bind(pop);
            a.load(R::RAX, Rm::rip(list, 0), 8, false)?;
            a.op(0, true, 0x85, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
            a.branch(Some(4), empty, true);
            a.load(R::RCX, Rm::mem(R::RAX, 8), 8, false)?;
            a.store(Rm::rip(list, 0), R::RCX, 8)?;
            a.load(R::RDI, Rm::mem(R::RAX, 16), 8, false)?;
            a.call_rm(Rm::mem(R::RAX, 0))?;
            a.branch(None, pop, true);
            a.bind(empty);
        }
        let end = self.named(b"__cppgm_fini_array_end")?;
        a.lea(R::RBX, Rm::rip(end, 0))?;
        let start = self.named(b"__cppgm_fini_array_start")?;
        a.lea(R::R13, Rm::rip(start, 0))?;
        a.bind(fini);
        a.alu_rr(7, R::RBX, Rm::Reg(R::R13), 8)?;
        a.branch(Some(6), done, true);
        a.alu_ri(5, Rm::Reg(R::RBX), 8, 8)?;
        a.call_rm(Rm::mem(R::RBX, 0))?;
        a.branch(None, fini, true);
        a.bind(done);
        runtime::emit_exit(&mut a, R::R12)?;
        a.resolve()?;
        self.image.append(a)
    }
    fn eh(&mut self) -> Result<(), Error> {
        let top = self.named(b"__cppgm_eh_top")?;
        let payload = self.named(b"__cppgm_eh_payload")?;
        let selector = self.named(b"__cppgm_eh_selector")?;
        let dispatch = self.named(b"__cppgm_eh_dispatch")?;
        let uncaught = self.named(b"__cppgm_eh_uncaught")?;
        self.define_data(top, ImageSection::Bss, 8, 8)?;
        self.define_data(payload, ImageSection::Bss, 16, 16)?;
        self.define_data(selector, ImageSection::Bss, 8, 8)?;
        DispatcherSymbols {
            top,
            selector,
            dispatch,
            uncaught,
        }
        .emit(&mut self.image)
    }
    fn system(&mut self) -> Result<(), Error> {
        let base = u64::try_from(self.image.text.len()).map_err(|_| Error::Limit)?;
        let mut a = Assembler::default();
        let symbol = self.named(b"__cppgm_mmap")?;
        self.begin(&mut a, base, symbol)?;
        let done = a.label()?;
        a.mov_rr(R::RSI, R::RDI)?;
        a.op(0, false, 0x33, 1, R::RDI.code(), Rm::Reg(R::RDI), false, 0)?;
        a.mov_ri(R::RDX, 3)?;
        a.mov_ri(R::R10, 0x22)?;
        a.mov_ri(R::R8, -1)?;
        a.op(0, false, 0x33, 1, R::R9.code(), Rm::Reg(R::R9), false, 0)?;
        a.mov_ri(R::RAX, 9)?;
        syscall(&mut a);
        a.alu_ri(7, Rm::Reg(R::RAX), -4096, 8)?;
        a.branch(Some(6), done, true);
        a.op(0, false, 0x33, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
        a.bind(done);
        a.ret();
        a.resolve()?;
        let symbol = self.named(b"__cppgm_munmap")?;
        self.begin(&mut a, base, symbol)?;
        a.mov_ri(R::RAX, 11)?;
        syscall(&mut a);
        a.ret();
        let symbol = self.named(b"__cppgm_mremap")?;
        self.begin(&mut a, base, symbol)?;
        let moved = a.label()?;
        a.mov_ri(R::R10, 1)?;
        a.mov_ri(R::RAX, 25)?;
        syscall(&mut a);
        a.alu_ri(7, Rm::Reg(R::RAX), -4096, 8)?;
        a.branch(Some(6), moved, true);
        a.op(0, false, 0x33, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
        a.bind(moved);
        a.ret();
        a.resolve()?;
        self.image.append(a)
    }
    fn terminate(&mut self) -> Result<(), Error> {
        let base = u64::try_from(self.image.text.len()).map_err(|_| Error::Limit)?;
        let mut a = Assembler::default();
        let terminate = self.named(b"_ZSt9terminatev")?;
        self.begin(&mut a, base, terminate)?;
        a.mov_ri(R::RDI, 134)?;
        runtime::emit_exit(&mut a, R::RDI)?;
        for name in [
            &b"__cxa_pure_virtual"[..],
            &b"__cxa_deleted_virtual"[..],
            &b"abort"[..],
        ] {
            let symbol = self.named(name)?;
            self.begin(&mut a, base, symbol)?;
            a.jump_symbol(terminate)?;
        }
        a.resolve()?;
        self.image.append(a)
    }
    fn runtime_type(&mut self, code: &[u8]) -> Result<(), Error> {
        let name = self.named(&[&b"_ZTS"[..], code].concat())?;
        if self.image.definition(name).section == ImageSection::Undefined {
            let at = self.image.reserve(
                ImageSection::Rodata,
                u64::try_from(code.len() + 1).map_err(|_| Error::Limit)?,
                1,
            )?;
            let offset = usize::try_from(at).map_err(|_| Error::Limit)?;
            self.image.rodata[offset..offset + code.len()].copy_from_slice(code);
            self.image.define(name, ImageSection::Rodata, at);
        }
        let information = self.named(&[&b"_ZTI"[..], code].concat())?;
        if self.image.definition(information).section != ImageSection::Undefined {
            return Ok(());
        }
        let (is_indirect, flags, pointee) = runtime_type(code).ok_or(Error::Limit)?;
        let at = self
            .image
            .reserve(ImageSection::Rodata, if is_indirect { 32 } else { 16 }, 16)?;
        self.image.define(information, ImageSection::Rodata, at);
        let vtable = self.named(VTABLES[if is_indirect { 4 } else { 3 }])?;
        self.image
            .add_data_fixup(ImageSection::Rodata, at, vtable, 16)?;
        self.image
            .add_data_fixup(ImageSection::Rodata, at + 8, name, 0)?;
        if is_indirect {
            self.image.rodata[usize::try_from(at + 16).map_err(|_| Error::Limit)?] = flags;
            let target = self.named(&[&b"_ZTI"[..], pointee].concat())?;
            self.image
                .add_data_fixup(ImageSection::Rodata, at + 24, target, 0)?;
            self.runtime_type(pointee)?;
        }
        Ok(())
    }
}
fn syscall(a: &mut Assembler) {
    a.byte(0x0f);
    a.byte(0x05);
}
pub(super) fn build(members: &Members, names: &[&[u8]]) -> Result<Object, Error> {
    let mut core = Core::new();
    if members.contains(Member::Entry) {
        core.entry()?;
    }
    if members.contains(Member::EhCore) {
        core.eh()?;
    }
    if members.contains(Member::System) {
        core.system()?;
    }
    if members.contains(Member::Terminate) {
        core.terminate()?;
    }
    if members.contains(Member::TypeVtables) {
        for name in VTABLES {
            let symbol = core.named(name)?;
            core.define_data(symbol, ImageSection::Rodata, 32, 16)?;
        }
    }
    if members.contains(Member::Support(Support::Exceptions)) {
        core.runtime_type(b"v")?;
        core.runtime_type(b"Dn")?;
    }
    for &name in names {
        match member_of(name) {
            Some(Member::Builtins) => {
                let builtin = if name.starts_with(b"cppgm_builtin_") {
                    name.to_vec()
                } else {
                    [&b"cppgm_builtin_"[..], name].concat()
                };
                let symbol = core.named(name)?;
                let base = core.image.align_text()?;
                core.image.define(symbol, ImageSection::Text, base);
                let mut a = Assembler::default();
                let known = super::super::builtins::emit(&mut a, &builtin)?;
                if !known {
                    return Err(Error::Construction("unknown runtime builtin".into()));
                }
                a.resolve()?;
                core.image.append(a)?;
            }
            Some(Member::Fundamental) => core.runtime_type(type_name(name).ok_or(Error::Limit)?)?,
            _ => {}
        }
    }
    core.object()
}
