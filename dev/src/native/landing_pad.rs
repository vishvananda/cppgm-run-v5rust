//! Host landing MIR effects and selector translation (`emit_control`, `emitter_memory`). See NOTICE.
use super::{
    mir::{self, BlockName, Instruction, Op},
    region_flow::{Plan, Site, Tables},
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    x86::{Assembler, Register as R, Rm},
};
use crate::lowir::{
    Error,
    model::{BlockId, InstId, RuntimeInstruction as Runtime, TempId},
    vocabulary::Type,
};
pub(super) fn runtime(runtime: &Runtime) -> bool {
    matches!(
        runtime,
        Runtime::Exception(_)
            | Runtime::ExceptionSelector(_)
            | Runtime::Resume
            | Runtime::EhTry(_)
            | Runtime::EhCleanup(_)
            | Runtime::EhCatch(..)
            | Runtime::EhCatchAll(_)
            | Runtime::EhFilter(..)
            | Runtime::EhEnd
    )
}
pub(super) fn protect_call(
    mir: &mut mir::Function,
    first: usize,
    plan: &Plan,
    id: InstId,
) -> Result<(), Error> {
    let Site::Landing(to) = plan.sites[id.index()] else {
        return Ok(());
    };
    let landing = Some(plan.indexes[to.index()].ok_or(Error::Limit)?);
    for inst in &mut mir.instructions[first..] {
        if let Op::Call(site) = &mut inst.op {
            site.landing = landing;
        }
    }
    Ok(())
}
#[derive(Clone, Copy)]
pub(super) enum Effect {
    Landing { copy_selector: bool },
    Select(super::region_flow::LandingId),
    CatchAll(i64),
    Resume,
}
impl Effect {
    // The frozen MIR carries these frame addresses explicitly. This typed
    // effect derives them from the single function-owned exception area;
    // frame ordering counts each address and relocates that area once.
    pub(super) fn visit_slots(self, tables: &Tables, mut visit: impl FnMut(i32)) {
        let slot = tables.slot;
        match self {
            Self::Landing { copy_selector } => {
                visit(slot);
                visit(slot.wrapping_add(if tables.raw { 16 } else { 8 }));
                if copy_selector {
                    visit(slot.wrapping_add(8));
                }
            }
            Self::Select(_) => {
                visit(slot.wrapping_add(8));
                visit(slot.wrapping_add(16));
            }
            Self::CatchAll(_) => visit(slot.wrapping_add(8)),
            Self::Resume => {}
        }
    }
    pub(super) fn emit(self, a: &mut Assembler, tables: &Tables) -> Result<(), Error> {
        let slot = tables.slot;
        match self {
            Self::Landing { copy_selector } => {
                a.store(Rm::frame(slot), R::RAX, 8)?;
                a.store(Rm::frame(slot + if tables.raw { 16 } else { 8 }), R::RDX, 8)?;
                if copy_selector {
                    a.store(Rm::frame(slot + 8), R::RDX, 8)?;
                }
            }
            Self::Select(index) => {
                let done = a.label()?;
                a.load(R::R10, Rm::frame(slot + 16), 8, false)?;
                for &(filter, selector) in &tables.map[tables.landings[index.index()].map.clone()] {
                    let next = a.label()?;
                    a.alu_ri(7, Rm::Reg(R::R10), i64::from(filter), 4)?;
                    a.branch(Some(5), next, false);
                    a.store_imm(Rm::frame(slot + 8), i64::from(selector), 8)?;
                    a.branch(None, done, false);
                    a.bind(next);
                }
                a.store_imm(Rm::frame(slot + 8), 0, 8)?;
                a.bind(done);
            }
            Self::CatchAll(selector) => {
                let skip = a.label()?;
                a.alu_ri(7, Rm::frame(slot + 8), 0, 8)?;
                a.branch(Some(5), skip, true);
                a.store_imm(Rm::frame(slot + 8), selector, 8)?;
                a.bind(skip);
            }
            Self::Resume => {
                return Err(Error::Construction(
                    "host resume requires its terminal label".into(),
                ));
            }
        }
        Ok(())
    }
}
impl Selection<'_> {
    fn host_effect(&mut self, effect: Effect) {
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::HostEh(effect),
            location: self.location,
        });
    }
    pub(super) fn host_entry_selector(&mut self, selector: i64) {
        if selector != 0 {
            self.host_effect(Effect::CatchAll(selector));
        }
    }
    pub(super) fn host_pad(
        &mut self,
        plan: &Plan,
        block: BlockId,
        target: mir::BlockId,
    ) -> Result<(), Error> {
        let Some(index) = plan.indexes[block.index()] else {
            return Ok(());
        };
        self.location = None;
        let landing = &self.mir.eh.landings[index.index()];
        let pad = landing.pad;
        let select = landing.select;
        let translate = landing.translate;
        let name = BlockName::from_lowir(self.body.block(block));
        if let Some(pad) = pad {
            if self.mir.begin_block(name)? != pad {
                return Err(Error::Limit);
            }
            self.mir.blocks[pad.index()].landing = true;
            self.host_effect(Effect::Landing {
                copy_selector: self.mir.eh.raw && !translate,
            });
            self.jump(if translate {
                select.ok_or(Error::Limit)?
            } else {
                target
            });
            self.mir.end_block();
        }
        if let Some(select) = select {
            if self.mir.begin_block(name)? != select {
                return Err(Error::Limit);
            }
            self.mir.blocks[select.index()].landing = true;
            self.host_effect(Effect::Select(index));
            self.jump(target);
            self.mir.end_block();
        }
        Ok(())
    }
    pub(super) fn host_resume(
        &mut self,
        plan: &Plan,
        site: Site,
        targets: &[Option<mir::BlockId>],
    ) -> Result<(), Error> {
        if let Site::Landing(block) = site {
            let index = plan.indexes[block.index()].ok_or(Error::Limit)?;
            self.jump(
                self.mir.eh.landings[index.index()]
                    .select
                    .or(targets[block.index()])
                    .ok_or(Error::Limit)?,
            );
        } else {
            self.host_effect(Effect::Resume);
        }
        Ok(())
    }
    pub(super) fn host_exception(
        &mut self,
        plan: &Plan,
        id: InstId,
        result: Option<TempId>,
        runtime: &Runtime,
        targets: &[Option<mir::BlockId>],
    ) -> Result<(), Error> {
        match *runtime {
            Runtime::Exception(ty) | Runtime::ExceptionSelector(ty) => {
                if ty != Type::Ptr && (ty.bits() == 0 || ty == Type::I128) {
                    return Err(Error::Diagnostic(
                        "exception payload of a non-integer type is not supported".into(),
                    ));
                }
                let temp = result.ok_or(Error::Limit)?;
                let destination = self.result(temp, [None, None], 0);
                let offset = self.mir.eh.slot
                    + if matches!(runtime, Runtime::ExceptionSelector(_)) {
                        8
                    } else {
                        0
                    };
                self.emit(ScalarOp::Load(destination, ty, Operand::Frame(offset)));
                self.finish(temp, destination)?;
            }
            Runtime::Resume => self.host_resume(plan, plan.sites[id.index()], targets)?,
            Runtime::EhCatchAll(selector) if plan.sites[id.index()] == Site::Clause => {
                self.host_entry_selector(selector.unwrap_or(0));
            }
            Runtime::EhTry(_)
            | Runtime::EhCleanup(_)
            | Runtime::EhCatch(..)
            | Runtime::EhCatchAll(_)
            | Runtime::EhFilter(..)
            | Runtime::EhEnd => {}
            _ => return Err(super::value_selection::unsupported()),
        }
        Ok(())
    }
}
