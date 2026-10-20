//! Block-local scalar placement, spill homes and callee-save frame ownership. See NOTICE.
use super::{
    abi,
    mir::{Binding, Function, Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    value_facts::Facts,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{Body, TempId},
    vocabulary::Type,
};
const ORDER: [R; 12] = [
    R::R8,
    R::R9,
    R::RDI,
    R::RSI,
    R::RDX,
    R::RCX,
    R::RAX,
    R::RBX,
    R::R12,
    R::R13,
    R::R14,
    R::R15,
];
pub(super) const SAVED: [R; 5] = [R::RBX, R::R12, R::R13, R::R14, R::R15];
#[derive(Default)]
struct Location {
    reg: Option<R>,
    home: Option<i32>,
    homed: bool,
}
pub(super) struct Placement {
    locations: Vec<Location>,
    owners: [Option<TempId>; 16],
    frame: u32,
    pub(super) preserve: u32,
    vector_boundary: bool,
    excluded: u32,
    dying: Vec<TempId>,
}
impl Placement {
    pub(super) fn new(body: &Body, vector_boundary: bool, excluded: u32) -> Self {
        Self {
            locations: (0..body.temps().len())
                .map(|_| Location::default())
                .collect(),
            owners: [None; 16],
            frame: 0,
            preserve: 0,
            vector_boundary,
            excluded,
            dying: Vec::new(),
        }
    }
    fn bind(&mut self, temp: TempId, r: R) {
        self.owners[usize::from(r.code())] = Some(temp);
        self.locations[temp.index()].reg = Some(r);
        if SAVED.contains(&r) {
            self.preserve |= 1 << r.code();
        }
    }
    fn allocate(
        &self,
        facts: &Facts,
        temp: TempId,
        prefs: [Option<R>; 2],
        exclude: u32,
    ) -> Option<R> {
        let allowed = |r: R| {
            self.owners[usize::from(r.code())].is_none()
                && (facts.intervals[temp.index()].forbid | exclude | self.excluded)
                    & (1 << r.code())
                    == 0
        };
        for reg in prefs
            .into_iter()
            .chain([facts.intervals[temp.index()].hint])
            .flatten()
        {
            if ORDER.contains(&reg) && allowed(reg) {
                return Some(reg);
            }
        }
        if self.vector_boundary {
            for r in [
                R::R8,
                R::R9,
                R::RDI,
                R::RSI,
                R::RBX,
                R::R12,
                R::R13,
                R::R14,
                R::R15,
                R::RDX,
                R::RCX,
                R::RAX,
            ] {
                if allowed(r) {
                    return Some(r);
                }
            }
        } else {
            for r in ORDER {
                if allowed(r) {
                    return Some(r);
                }
            }
        }
        None
    }
    fn home(&mut self, body: &Body, temp: TempId, mir: &mut Function) -> Result<i32, Error> {
        if let Some(home) = self.locations[temp.index()].home {
            return Ok(home);
        }
        let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
        let bytes = abi::value_bytes(ty)?.max(1).div_ceil(8) * 8;
        let align = abi::value_align(ty)?.clamp(8, 16);
        let size = (u64::from(self.frame) + u64::from(bytes) + u64::from(align) - 1)
            & !(u64::from(align) - 1);
        if size > 0x7fff_0000 {
            return Err(Error::Diagnostic(
                "stack frame exceeds the 32-bit displacement range".into(),
            ));
        }
        self.frame = u32::try_from(size).map_err(|_| Error::Limit)?;
        let offset = -i32::try_from(size).map_err(|_| Error::Limit)?;
        self.locations[temp.index()].home = Some(offset);
        mir.homes += 1;
        mir.bindings.push(Binding {
            name: body.temp(temp).name,
            ty,
            offset,
            parameter: temp.index() < body.parameter_count,
        });
        Ok(offset)
    }
    pub(super) fn entry(
        &mut self,
        body: &Body,
        facts: &Facts,
        mir: &mut Function,
    ) -> Result<(), Error> {
        for (index, parameter) in body.temps().iter().take(body.parameter_count).enumerate() {
            if facts.intervals[index].last.is_none() {
                continue;
            }
            let temp = body.find_temp(parameter.name).ok_or(Error::Limit)?;
            let ty = parameter.ty.ok_or(Error::Limit)?;
            if ty.float_rank() != 0 || matches!(ty, Type::Object { .. } | Type::I128) {
                return Err(unsupported());
            }
            let [part] = mir.layout.parts(&mir.layout.arguments[index]) else {
                return Err(unsupported());
            };
            if let Some(r) = part.register
                && facts.intervals[index].forbid & (1 << r.code()) == 0
            {
                self.bind(temp, r);
            }
        }
        let incoming = mir
            .layout
            .arguments
            .iter()
            .enumerate()
            .filter(|(i, _)| facts.intervals[*i].last.is_some())
            .flat_map(|(_, arg)| mir.layout.parts(arg))
            .filter_map(|part| part.register)
            .fold(0, |mask, r| mask | (1 << r.code()));
        for (index, parameter) in body.temps().iter().take(body.parameter_count).enumerate() {
            if facts.intervals[index].last.is_none() {
                continue;
            }
            let temp = body.find_temp(parameter.name).ok_or(Error::Limit)?;
            if self.locations[index].reg.is_some() {
                continue;
            }
            let ty = parameter.ty.ok_or(Error::Limit)?;
            let [part] = mir.layout.parts(&mir.layout.arguments[index]) else {
                return Err(unsupported());
            };
            let source = part.register;
            let stack = part.stack;
            if let Some(reg) = source {
                if let Some(target) = self.allocate(facts, temp, [None, None], incoming) {
                    self.bind(temp, target);
                    mir.instructions.push(Instruction {
                        op: Op::Scalar(ScalarOp::Move(target, Operand::Register(reg))),
                        location: None,
                    });
                } else {
                    let home = self.home(body, temp, mir)?;
                    self.locations[index].homed = true;
                    mir.instructions.push(Instruction {
                        op: Op::Scalar(ScalarOp::Store(home, ty, reg)),
                        location: None,
                    });
                }
            } else {
                let home = self.home(body, temp, mir)?;
                self.locations[index].homed = true;
                mir.instructions.push(Instruction {
                    op: Op::Scalar(ScalarOp::Load(R::RAX, ty, 16_i32.wrapping_add(stack))),
                    location: None,
                });
                mir.instructions.push(Instruction {
                    op: Op::Scalar(ScalarOp::Store(home, ty, R::RAX)),
                    location: None,
                });
            }
        }
        Ok(())
    }
    pub(super) fn operand(&self, temp: TempId) -> Result<Operand, Error> {
        let loc = &self.locations[temp.index()];
        if let Some(r) = loc.reg {
            Ok(Operand::Register(r))
        } else if let Some(home) = loc.home {
            Ok(Operand::Frame(home))
        } else {
            Err(unsupported())
        }
    }
    pub(super) fn dying(
        &self,
        facts: &Facts,
        value: Operand,
        pos: usize,
        body: &Body,
        skip_parameters: bool,
    ) -> Option<R> {
        let r = value.register()?;
        if usize::from(r.code()) >= self.owners.len() {
            return None;
        }
        let temp = self.owners[usize::from(r.code())]?;
        if skip_parameters && self.vector_boundary && temp.index() < body.parameter_count {
            return None;
        }
        facts.intervals[temp.index()]
            .last
            .filter(|&last| last <= pos)
            .map(|_| r)
    }
    pub(super) fn parameter_mask(&self, body: &Body, operands: [Operand; 2]) -> u32 {
        if !self.vector_boundary {
            return 0;
        }
        operands
            .into_iter()
            .filter_map(Operand::register)
            .filter(|r| {
                usize::from(r.code()) < self.owners.len()
                    && self.owners[usize::from(r.code())]
                        .is_some_and(|t| t.index() < body.parameter_count)
            })
            .fold(0, |mask, r| mask | (1 << r.code()))
    }
    pub(super) fn release(&mut self, facts: &Facts, pos: usize) {
        for owner in &mut self.owners {
            if owner.is_some_and(|t| {
                facts.intervals[t.index()]
                    .last
                    .is_none_or(|last| last <= pos)
            }) && let Some(temp) = owner.take()
            {
                self.dying.push(temp);
            }
        }
    }
    pub(super) fn retire(&mut self) {
        for temp in self.dying.drain(..) {
            let loc = &mut self.locations[temp.index()];
            if loc
                .reg
                .is_some_and(|r| self.owners[usize::from(r.code())] != Some(temp))
            {
                loc.reg = None;
            }
        }
    }
    pub(super) fn result(
        &mut self,
        facts: &Facts,
        temp: TempId,
        pos: usize,
        prefs: [Option<R>; 2],
        exclude: u32,
        mir: &mut Function,
    ) -> R {
        self.release(facts, pos);
        if let Some(r) = self.allocate(facts, temp, prefs, exclude) {
            self.bind(temp, r);
            r
        } else {
            self.locations[temp.index()].homed = true;
            mir.spills += 1;
            if exclude & (1 << R::R10.code()) != 0 {
                R::R11
            } else {
                R::R10
            }
        }
    }
    pub(super) fn finish(
        &mut self,
        body: &Body,
        temp: TempId,
        r: R,
        mir: &mut Function,
        location: Option<crate::lowir::model::DebugLocation>,
    ) -> Result<(), Error> {
        if self.locations[temp.index()].homed {
            let home = self.home(body, temp, mir)?;
            let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
            mir.instructions.push(Instruction {
                op: Op::Scalar(ScalarOp::Store(home, ty, r)),
                location,
            });
        }
        Ok(())
    }
    pub(super) fn finish_frame(&self, mir: &mut Function) -> Result<(), Error> {
        let mut frame = u64::from(self.frame);
        if mir.scratch_bytes != 0 {
            frame = (frame + 48 + 15) & !15;
        }
        frame = (frame + 8 * u64::from(self.preserve.count_ones()) + 15) & !15;
        mir.stack_size = u32::try_from(frame).map_err(|_| Error::Limit)?;
        mir.preserve = self.preserve;
        Ok(())
    }
}
