//! Standalone process entry, atexit and course EH dispatcher. See NOTICE.
use super::{
    builtins,
    image::{Image, ImageSymbol, Section},
    startup::Startup,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{Error, model::Program};
pub(super) struct Runtime {
    top: ImageSymbol,
    selector: ImageSymbol,
    dispatch: ImageSymbol,
    uncaught: ImageSymbol,
    payload: ImageSymbol,
    catch: ImageSymbol,
    wide_division: [ImageSymbol; 4],
    float_conversion: [ImageSymbol; 12],
}
#[derive(Clone, Copy)]
pub(super) struct Symbols {
    pub(super) top: ImageSymbol,
    pub(super) payload: ImageSymbol,
    pub(super) selector: ImageSymbol,
    pub(super) dispatch: ImageSymbol,
    pub(super) uncaught: ImageSymbol,
    pub(super) catch: ImageSymbol,
    pub(super) wide_division: [ImageSymbol; 4],
    pub(super) float_conversion: [ImageSymbol; 12],
}
impl Runtime {
    pub(super) fn symbols(&self) -> Symbols {
        Symbols {
            top: self.top,
            payload: self.payload,
            selector: self.selector,
            dispatch: self.dispatch,
            uncaught: self.uncaught,
            catch: self.catch,
            wide_division: self.wide_division,
            float_conversion: self.float_conversion,
        }
    }
    pub(super) fn declare(image: &mut Image) -> Result<Self, Error> {
        let top = image.bss_symbol(8, 8)?;
        let payload = image.bss_symbol(16, 16)?;
        let selector = image.bss_symbol(8, 8)?;
        let dispatch = image.local()?;
        let uncaught = image.local()?;
        // Catch, integer division and floating conversion symbols are reserved
        // here as in the frozen runtime; their definitions belong to link mode.
        let catch = image.local()?;
        let wide_division = [
            image.local()?,
            image.local()?,
            image.local()?,
            image.local()?,
        ];
        let mut float_conversion = [catch; 12];
        for symbol in &mut float_conversion {
            *symbol = image.local()?;
        }
        Ok(Self {
            top,
            selector,
            dispatch,
            uncaught,
            payload,
            catch,
            wide_division,
            float_conversion,
        })
    }

    pub(super) fn emit(
        &self,
        image: &mut Image,
        program: &Program,
        startup: &Startup,
    ) -> Result<ImageSymbol, Error> {
        self.symbols().dispatcher(image)?;
        for symbol in program.symbols() {
            let Some(function) = program.function(symbol) else {
                continue;
            };
            if function.body.is_some() {
                continue;
            }
            let id = program.find_symbol(symbol.name).ok_or(Error::Limit)?;
            if let Some(object) = function.metadata.tls_for {
                emit_tls_wrapper(
                    image,
                    ImageSymbol::Program(id),
                    ImageSymbol::Program(object),
                )?;
                continue;
            }
            let mut a = Assembler::default();
            if let Some(object) = function.metadata.object {
                if !builtins::emit(&mut a, program.names.bytes(object))? {
                    continue;
                }
            } else {
                continue;
            }
            let base = image.align_text()?;
            image.define(ImageSymbol::Program(id), Section::Text, base);
            a.resolve()?;
            image.append(a)?;
        }
        let exit_state = if let Some(symbol) = startup.atexit {
            let state = image.bss_symbol(24, 8)?;
            exit_registration(image, ImageSymbol::Program(symbol), state)?;
            Some(state)
        } else {
            None
        };
        let start = image.local()?;
        let base = image.align_text()?;
        image.define(start, Section::Text, base);
        let mut a = Assembler::default();
        if let Some(init) = startup.init {
            a.call_symbol(ImageSymbol::Program(init))?;
        }
        if startup.entry_parameters >= 1 {
            a.load(R::RDI, Rm::mem(R::RSP, 0), 8, false)?;
            a.lea(R::RSI, Rm::mem(R::RSP, 8))?;
        }
        a.call_symbol(ImageSymbol::Program(startup.entry.ok_or(Error::Limit)?))?;
        if startup.fini.is_some() || exit_state.is_some() {
            a.mov_rr(R::R12, R::RAX)?;
            if let Some(state) = exit_state {
                exit_drain(&mut a, state)?;
            }
            if let Some(fini) = startup.fini {
                a.call_symbol(ImageSymbol::Program(fini))?;
            }
            emit_exit(&mut a, R::R12)?;
        } else {
            emit_exit(&mut a, R::RAX)?;
        }
        a.resolve()?;
        image.append(a)?;
        Ok(start)
    }
}
pub(super) fn emit_exit(a: &mut Assembler, status: R) -> Result<(), Error> {
    a.mov_rr(R::RDI, status)?;
    a.mov_ri(R::RAX, 231)?;
    a.byte(0x0f);
    a.byte(0x05);
    Ok(())
}
fn exit_registration(
    image: &mut Image,
    symbol: ImageSymbol,
    state: ImageSymbol,
) -> Result<(), Error> {
    let base = image.align_text()?;
    image.define(symbol, Section::Text, base);
    let mut a = Assembler::default();
    let carve = a.label()?;
    let failed = a.label()?;
    a.load(R::RAX, Rm::rip(state, 8), 8, false)?;
    a.alu_rr(7, R::RAX, Rm::rip(state, 16), 8)?;
    a.branch(Some(5), carve, true);
    a.push(R::RDI);
    a.mov_ri(R::RAX, 9)?;
    a.mov_ri(R::RDI, 0)?;
    a.mov_ri(R::RSI, 4096)?;
    a.mov_ri(R::RDX, 3)?;
    a.mov_ri(R::R10, 0x22)?;
    a.mov_ri(R::R8, -1)?;
    a.mov_ri(R::R9, 0)?;
    a.byte(0x0f);
    a.byte(0x05);
    a.pop(R::RDI);
    a.op(0, true, 0x85, 1, R::RAX.code(), Rm::Reg(R::RAX), false, 0)?;
    a.branch(Some(8), failed, true);
    a.lea(R::RDX, Rm::mem(R::RAX, 4096))?;
    a.store(Rm::rip(state, 16), R::RDX, 8)?;
    a.bind(carve);
    a.lea(R::RDX, Rm::mem(R::RAX, 16))?;
    a.store(Rm::rip(state, 8), R::RDX, 8)?;
    a.store(Rm::mem(R::RAX, 0), R::RDI, 8)?;
    a.load(R::RDX, Rm::rip(state, 0), 8, false)?;
    a.store(Rm::mem(R::RAX, 8), R::RDX, 8)?;
    a.store(Rm::rip(state, 0), R::RAX, 8)?;
    a.mov_ri(R::RAX, 0)?;
    a.ret();
    a.bind(failed);
    a.mov_ri(R::RAX, -1)?;
    a.ret();
    a.resolve()?;
    image.append(a)
}
fn exit_drain(a: &mut Assembler, state: ImageSymbol) -> Result<(), Error> {
    let next = a.label()?;
    let done = a.label()?;
    a.bind(next);
    a.load(R::RDI, Rm::rip(state, 0), 8, false)?;
    a.op(0, true, 0x85, 1, R::RDI.code(), Rm::Reg(R::RDI), false, 0)?;
    a.branch(Some(4), done, true);
    a.load(R::RAX, Rm::mem(R::RDI, 8), 8, false)?;
    a.store(Rm::rip(state, 0), R::RAX, 8)?;
    a.call_rm(Rm::mem(R::RDI, 0))?;
    a.branch(None, next, true);
    a.bind(done);
    Ok(())
}

pub(super) const IMPORT_NAMES: [&[u8]; 24] = [
    b"__cppgm_eh_top",
    b"__cppgm_eh_payload",
    b"__cppgm_eh_selector",
    b"__cppgm_eh_dispatch",
    b"__cppgm_eh_uncaught",
    b"__cppgm_eh_catch",
    b"__divti3",
    b"__udivti3",
    b"__modti3",
    b"__umodti3",
    b"__floattisf",
    b"__floattidf",
    b"__floattixf",
    b"__floatuntisf",
    b"__floatuntidf",
    b"__floatuntixf",
    b"__fixsfti",
    b"__fixdfti",
    b"__fixxfti",
    b"__fixunssfti",
    b"__fixunsdfti",
    b"__fixunsxfti",
    b"_Unwind_Resume",
    b"__gxx_personality_v0",
];
impl Symbols {
    pub(super) fn imports(image: &mut Image) -> Result<Self, Error> {
        let top = image.local()?;
        let payload = image.local()?;
        let selector = image.local()?;
        let dispatch = image.local()?;
        let uncaught = image.local()?;
        let catch = image.local()?;
        let mut wide_division = [catch; 4];
        for symbol in &mut wide_division {
            *symbol = image.local()?;
        }
        let mut float_conversion = [catch; 12];
        for symbol in &mut float_conversion {
            *symbol = image.local()?;
        }
        let _ = image.local()?;
        let _ = image.local()?;
        Ok(Self {
            top,
            payload,
            selector,
            dispatch,
            uncaught,
            catch,
            wide_division,
            float_conversion,
        })
    }
    pub(super) fn dispatcher(&self, image: &mut Image) -> Result<(), Error> {
        DispatcherSymbols {
            top: self.top,
            selector: self.selector,
            dispatch: self.dispatch,
            uncaught: self.uncaught,
        }
        .emit(image)
    }
}

pub(super) fn emit_tls_wrapper(
    image: &mut Image,
    symbol: ImageSymbol,
    object: ImageSymbol,
) -> Result<(), Error> {
    let base = image.align_text()?;
    image.define(symbol, Section::Text, base);
    let mut a = Assembler::default();
    a.lea(R::RAX, Rm::rip(object, 0))?;
    a.ret();
    a.resolve()?;
    image.append(a)
}

pub(super) struct DispatcherSymbols {
    pub(super) top: ImageSymbol,
    pub(super) selector: ImageSymbol,
    pub(super) dispatch: ImageSymbol,
    pub(super) uncaught: ImageSymbol,
}
impl DispatcherSymbols {
    pub(super) fn emit(&self, image: &mut Image) -> Result<(), Error> {
        let base = image.align_text()?;
        image.define(self.uncaught, Section::Text, base);
        let mut a = Assembler::default();
        a.mov_ri(R::RDI, 134)?;
        emit_exit(&mut a, R::RDI)?;
        while a.code.len() % 16 != 0 {
            a.byte(0xcc);
        }
        image.define(
            self.dispatch,
            Section::Text,
            base + u64::from(a.position()?),
        );
        let next = a.label()?;
        let skip = a.label()?;
        let found = a.label()?;
        a.load(R::R11, Rm::rip(self.top, 0), 8, false)?;
        a.bind(next);
        a.op(0, true, 0x85, 1, R::R11.code(), Rm::Reg(R::R11), false, 0)?;
        a.branch(Some(5), skip, true);
        a.jump_symbol(self.uncaught)?;
        a.bind(skip);
        a.op(0, false, 0xf6, 1, 0, Rm::mem(R::R11, 16), true, 1)?;
        a.imm(2, 1);
        a.branch(Some(4), found, true);
        a.load(R::R11, Rm::mem(R::R11, 0), 8, false)?;
        a.branch(None, next, true);
        a.bind(found);
        a.alu_ri(1, Rm::mem(R::R11, 16), 2, 1)?;
        a.store(Rm::rip(self.top, 0), R::R11, 8)?;
        for (index, reg) in [R::RBX, R::R12, R::R13, R::R14, R::R15]
            .into_iter()
            .enumerate()
        {
            a.load(
                reg,
                Rm::mem(
                    R::R11,
                    32 + 8 * i32::try_from(index).map_err(|_| Error::Limit)?,
                ),
                8,
                false,
            )?;
        }
        a.load(R::RBP, Rm::mem(R::R11, 16), 8, false)?;
        a.alu_ri(4, Rm::Reg(R::RBP), !3, 8)?;
        a.load(R::RSP, Rm::mem(R::R11, 24), 8, false)?;
        a.store_imm(Rm::rip(self.selector, 0), 0, 8)?;
        a.op(0, false, 0xff, 1, 4, Rm::mem(R::R11, 8), false, 0)?;
        a.resolve()?;
        image.append(a)
    }
}
