//! Course handler-stack selection facts, MIR and machine effects. See NOTICE.
use super::{
    image::ImageSymbol,
    mir::{BlockId, Instruction, Op},
    runtime::Symbols,
    scalar_mir::{Operand, bytes, signed},
    scalar_selection::Selection,
    x86::{Assembler, Label, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{Program, RuntimeInstruction as Runtime, SymbolId, TempId},
    vocabulary::Type,
};
use std::io::Write;

#[derive(Clone, Copy)]
pub(super) enum ExceptionOp {
    Push {
        target: BlockId,
        cleanup: bool,
        offset: i32,
    },
    Pop,
    Mark,
    Catch {
        type_info: Option<SymbolId>,
        selector: u32,
    },
    Read {
        ty: Type,
        destination: R,
    },
    Selector(R),
    Throw(Operand),
    Resume,
}
impl ExceptionOp {
    pub(super) fn write<W: Write>(
        self,
        program: &Program,
        out: &mut W,
        mut label: impl FnMut(BlockId, &mut W) -> std::io::Result<()>,
    ) -> std::io::Result<()> {
        match self {
            Self::Push {
                target, cleanup, ..
            } => {
                write!(out, "    eh_push ")?;
                label(target, out)?;
                write!(out, ", {}", u8::from(cleanup))
            }
            Self::Pop => write!(out, "    eh_pop"),
            Self::Mark => write!(out, "    eh_mark"),
            Self::Catch {
                type_info,
                selector,
            } => {
                write!(out, "    eh_catch ")?;
                if let Some(symbol) = type_info {
                    Operand::Symbol(symbol).write(program, out)?;
                    write!(out, ", ")?;
                }
                write!(out, "{selector}")
            }
            Self::Read { destination, .. } => {
                write!(out, "    load_exception {}", destination.spelling())
            }
            Self::Selector(r) => write!(out, "    load_selector {}", r.spelling()),
            Self::Throw(value) => {
                write!(out, "    throw ")?;
                value.write(program, out)
            }
            Self::Resume => write!(out, "    resume"),
        }
    }
    pub(super) fn rebase(self, begin: i32, bytes: u32) -> Self {
        if let Self::Throw(v) = self {
            Self::Throw(v.rebase(begin, bytes))
        } else {
            self
        }
    }
    pub(super) fn emit(
        self,
        a: &mut Assembler,
        runtime: Symbols,
        labels: &[Label],
        pushed: bool,
    ) -> Result<(), Error> {
        match self {
            Self::Push {
                target,
                cleanup,
                offset,
            } => {
                if pushed {
                    a.alu_ri(5, Rm::Reg(R::RSP), 80, 8)?;
                    a.mov_rr(R::R11, R::RSP)?;
                } else {
                    a.lea(R::R11, Rm::mem(R::RBP, offset))?;
                }
                a.load(R::R10, Rm::rip(runtime.top, 0), 8, false)?;
                a.store(Rm::mem(R::R11, 0), R::R10, 8)?;
                a.lea_label(R::R10, labels[target.index()]);
                a.store(Rm::mem(R::R11, 8), R::R10, 8)?;
                if cleanup {
                    a.lea(R::R10, Rm::mem(R::RBP, 1))?;
                    a.store(Rm::mem(R::R11, 16), R::R10, 8)?;
                } else {
                    a.store(Rm::mem(R::R11, 16), R::RBP, 8)?;
                }
                a.store(Rm::mem(R::R11, 24), R::RSP, 8)?;
                for (index, reg) in [R::RBX, R::R12, R::R13, R::R14, R::R15]
                    .into_iter()
                    .enumerate()
                {
                    a.store(
                        Rm::mem(
                            R::R11,
                            32 + 8 * i32::try_from(index).map_err(|_| Error::Limit)?,
                        ),
                        reg,
                        8,
                    )?;
                }
                a.store(Rm::rip(runtime.top, 0), R::R11, 8)
            }
            Self::Pop => {
                a.load(R::R11, Rm::rip(runtime.top, 0), 8, false)?;
                let previous = if pushed { R::R10 } else { R::R11 };
                a.load(previous, Rm::mem(R::R11, 0), 8, false)?;
                a.store(Rm::rip(runtime.top, 0), previous, 8)?;
                if pushed {
                    a.lea(R::RSP, Rm::mem(R::R11, 80))?;
                }
                Ok(())
            }
            Self::Throw(value) => {
                let to = Rm::rip(runtime.payload, 0);
                if let Some(reg) = value.register() {
                    a.store(to, reg, 8)?;
                } else if let Operand::Immediate(bits) = value {
                    let word = super::mir::low_word(bits)?;
                    if i32::try_from(word).is_ok() {
                        a.store_imm(to, word, 8)?;
                    } else {
                        a.mov_ri(R::R11, word)?;
                        a.store(to, R::R11, 8)?;
                    }
                } else {
                    a.load(R::R11, value.rm()?, 8, false)?;
                    a.store(to, R::R11, 8)?;
                }
                a.jump_symbol(runtime.dispatch)
            }
            Self::Resume => a.jump_symbol(runtime.dispatch),
            Self::Mark => {
                let skip = a.label()?;
                active_try(a, runtime.top, skip)?;
                a.alu_ri(1, Rm::mem(R::R11, 16), 1, 1)?;
                a.bind(skip);
                Ok(())
            }
            Self::Read { ty, destination } => {
                let taken = a.label()?;
                active_try(a, runtime.top, taken)?;
                let previous = if pushed { R::R10 } else { R::R11 };
                a.load(previous, Rm::mem(R::R11, 0), 8, false)?;
                a.store(Rm::rip(runtime.top, 0), previous, 8)?;
                if pushed {
                    a.lea(R::RSP, Rm::mem(R::R11, 80))?;
                }
                a.bind(taken);
                a.load(
                    destination,
                    Rm::rip(runtime.payload, 0),
                    bytes(ty),
                    signed(ty) && matches!(ty, Type::I8 | Type::I16 | Type::I32),
                )
            }
            Self::Selector(r) => a.load(r, Rm::rip(runtime.selector, 0), 4, false),
            Self::Catch {
                type_info,
                selector,
            } => {
                if let Some(symbol) = type_info {
                    a.lea(R::RDI, Rm::rip(ImageSymbol::Program(symbol), 0))?;
                } else {
                    a.op(0, false, 0x33, 1, R::RDI.code(), Rm::Reg(R::RDI), false, 0)?;
                }
                a.byte(0xbe);
                a.imm(i64::from(selector), 4);
                a.call_symbol(runtime.catch)
            }
        }
    }
}
fn active_try(a: &mut Assembler, top: ImageSymbol, skip: Label) -> Result<(), Error> {
    a.load(R::R11, Rm::rip(top, 0), 8, false)?;
    a.op(0, true, 0x85, 1, R::R11.code(), Rm::Reg(R::R11), false, 0)?;
    a.branch(Some(4), skip, true);
    a.lea(R::R10, Rm::mem(R::RBP, 2))?;
    a.op(
        0,
        true,
        0x39,
        1,
        R::R10.code(),
        Rm::mem(R::R11, 16),
        false,
        0,
    )?;
    a.branch(Some(5), skip, true);
    Ok(())
}
fn gpr(ty: Type) -> bool {
    ty == Type::Ptr || ty.bits() != 0 && ty != Type::I128
}
impl Selection<'_> {
    pub(super) fn exception(
        &mut self,
        result: Option<TempId>,
        inst: &Runtime,
        targets: &[Option<BlockId>],
    ) -> Result<(), Error> {
        let op = match *inst {
            Runtime::EhTry(target) | Runtime::EhCleanup(Some(target)) => ExceptionOp::Push {
                target: targets[target.index()].ok_or(Error::Limit)?,
                cleanup: matches!(inst, Runtime::EhCleanup(_)),
                offset: if self.mir.frame_policy.records_pushed {
                    0
                } else {
                    self.placement.frame_alloc(80, 8)?
                },
            },
            Runtime::EhCleanup(None) => ExceptionOp::Mark,
            Runtime::EhEnd => ExceptionOp::Pop,
            Runtime::EhCatch(symbol, selector) => ExceptionOp::Catch {
                type_info: Some(symbol),
                selector: selector_word(selector)?,
            },
            Runtime::EhCatchAll(selector) => ExceptionOp::Catch {
                type_info: None,
                selector: selector_word(selector)?,
            },
            Runtime::EhFilter(..) => {
                return Err(Error::Diagnostic(
                    "exception filters are not supported by the native runtime".into(),
                ));
            }
            Runtime::Exception(ty) => {
                if !gpr(ty) {
                    return Err(Error::Diagnostic(
                        "exception payload of a non-integer type is not supported".into(),
                    ));
                }
                let temp = result.ok_or(Error::Limit)?;
                let destination = self.result(temp, [None, None], 0);
                self.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Exception(ExceptionOp::Read { ty, destination }),
                    location: self.location,
                });
                return self.finish(temp, destination);
            }
            Runtime::ExceptionSelector(_) => {
                let temp = result.ok_or(Error::Limit)?;
                let r = self.result(temp, [None, None], 0);
                self.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Exception(ExceptionOp::Selector(r)),
                    location: self.location,
                });
                return self.finish(temp, r);
            }
            Runtime::Throw(ty, value) => {
                if !gpr(ty) {
                    return Err(Error::Diagnostic(
                        "throw of a non-integer payload is not supported".into(),
                    ));
                }
                ExceptionOp::Throw(self.operand(value, ty, R::R10)?)
            }
            Runtime::Resume => ExceptionOp::Resume,
            _ => return Err(super::value_selection::unsupported()),
        };
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Exception(op),
            location: self.location,
        });
        Ok(())
    }
}
fn selector_word(value: Option<i64>) -> Result<u32, Error> {
    u32::try_from(value.unwrap_or(0).cast_unsigned() & u64::from(u32::MAX))
        .map_err(|_| Error::Limit)
}
