//! Block-local scalar placement, spill homes and callee-save frame ownership. See NOTICE.
use super::{
    abi,
    mir::{Binding, BindingKind, Function, Instruction, Op},
    scalar_mir::{Operand, ScalarOp},
    value_facts::Facts,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{BlockId, Body, TempId},
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
#[derive(Clone, Copy, PartialEq, Eq)]
enum RegisterClass {
    Gpr,
    Vector,
    Memory,
}
#[derive(Default)]
struct Location {
    reg: Option<R>,
    home: Option<i32>,
    homed: bool,
    stored: bool,
}
pub(super) struct Placement {
    pub(super) host_parameters: Vec<bool>,
    pub(super) sret_home: Option<i32>,
    pub(super) variadic: Option<super::variadic::SaveArea>,
    pub(super) globals: super::global_placement::Globals,
    incoming: Vec<Option<R>>,
    block: Option<BlockId>,
    pinned: u32,
    written: u32,
    block_globals: u32,
    exit_block: bool,
    last: Vec<Option<usize>>,
    available: Vec<u32>,
    emitted: Vec<bool>,
    available_in: u32,
    locations: Vec<Location>,
    owners: [Option<TempId>; 32],
    classes: Vec<RegisterClass>,
    frame: u32,
    frame_objects: Vec<super::frame_layout::Object>,
    slots: Vec<Option<i32>>,
    pub(super) preserve: u32,
    work: std::cell::Cell<super::mir::AllocationWork>,
    vector_boundary: bool,
    excluded: u32,
    dying: Vec<TempId>,
    cleared: Vec<(TempId, R)>,
}
impl Placement {
    pub(super) fn new(
        body: &Body,
        layout: &abi::Layout,
        globals: super::global_placement::Globals,
        vector_boundary: bool,
        excluded: u32,
    ) -> Self {
        Self {
            host_parameters: Vec::new(),
            sret_home: None,
            variadic: None,
            globals,
            incoming: layout
                .arguments
                .iter()
                .zip(body.temps())
                .map(|(arg, temp)| {
                    if temp.ty.is_some_and(|ty| {
                        super::aggregate::memory_value(ty) || ty.float_rank() != 0
                    }) {
                        return None;
                    }
                    let [part] = layout.parts(arg) else {
                        return None;
                    };
                    part.register
                })
                .collect(),
            block: None,
            pinned: 0,
            written: 0,
            block_globals: 0,
            exit_block: false,
            last: vec![None; body.temps().len()],
            available: vec![0; body.blocks().len()],
            emitted: vec![false; body.blocks().len()],
            available_in: 0,
            locations: (0..body.temps().len())
                .map(|_| Location::default())
                .collect(),
            owners: [None; 32],
            classes: body
                .temps()
                .iter()
                .map(|t| match t.ty {
                    Some(Type::I128 | Type::F80 | Type::Object { .. }) => RegisterClass::Memory,
                    Some(Type::F32 | Type::F64) => RegisterClass::Vector,
                    _ => RegisterClass::Gpr,
                })
                .collect(),
            frame: 0,
            frame_objects: Vec::new(),
            slots: vec![None; body.slots().len()],
            preserve: 0,
            work: std::cell::Cell::default(),
            vector_boundary,
            excluded,
            dying: Vec::new(),
            cleared: Vec::new(),
        }
    }
    fn bind(&mut self, temp: TempId, r: R) {
        self.work.get_mut().grants += 1;
        self.owners[usize::from(r.code())] = Some(temp);
        self.locations[temp.index()].reg = Some(r);
        if self.incoming.get(temp.index()).copied().flatten() != Some(r) {
            self.written |= 1 << r.code();
        }
        if SAVED.contains(&r) || r == R::RBP {
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
        if self.classes[temp.index()] == RegisterClass::Memory {
            self.note_busy(0);
            return None;
        }
        let allowed = |r: R| {
            self.owners[usize::from(r.code())].is_none()
                && (facts.intervals[temp.index()].forbid | exclude | self.excluded | self.pinned)
                    & (1 << r.code())
                    == 0
        };
        let vector = self.classes[temp.index()] == RegisterClass::Vector;
        let pool = if vector {
            0x00ff_0000
        } else {
            ORDER.iter().fold(0, |mask, r| mask | (1 << r.code()))
                | if self.globals.rbp_free {
                    1 << R::RBP.code()
                } else {
                    0
                }
        };
        let prefs = if facts.level >= 1
            && facts.intervals[temp.index()].hint.is_some()
            && facts.intervals[temp.index()].hint == prefs[1]
        {
            [prefs[1], prefs[0]]
        } else {
            prefs
        };
        for reg in prefs
            .into_iter()
            .chain([facts.intervals[temp.index()].hint])
            .flatten()
        {
            if (if vector {
                (16..24).contains(&reg.code())
            } else {
                ORDER.contains(&reg) || self.globals.rbp_free && reg == R::RBP
            }) && allowed(reg)
            {
                return Some(reg);
            }
        }
        if vector {
            let register = (0..8).filter_map(|i| R::xmm(i).ok()).find(|&r| allowed(r));
            if register.is_none() {
                self.note_busy(pool);
            }
            return register;
        }
        if facts.intervals[temp.index()].forbid & super::abi::CALLER_SAVED
            == super::abi::CALLER_SAVED
        {
            for r in SAVED {
                if allowed(r) {
                    return Some(r);
                }
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
        if self.globals.rbp_free && allowed(R::RBP) {
            return Some(R::RBP);
        }
        self.note_busy(pool);
        None
    }
    fn note_busy(&self, pool: u32) {
        let mut work = self.work.get();
        work.busy += 1;
        let parameter = self.owners.iter().enumerate().any(|(code, owner)| {
            pool & (1 << code) != 0 && owner.is_some_and(|temp| temp.index() < self.incoming.len())
        });
        if parameter {
            work.busy_parameters += 1;
        } else {
            work.busy_values += 1;
        }
        self.work.set(work);
    }
    pub(super) fn atomic_staging(
        &mut self,
        addresses: u32,
        mir: &Function,
    ) -> Result<(R, Option<i32>), Error> {
        let allocatable = ORDER.iter().fold(0, |mask, r| mask | (1 << r.code()));
        let occupied = self.owners.iter().enumerate().fold(0, |mask, (i, owner)| {
            mask | if owner.is_some() { 1 << i } else { 0 }
        });
        // Staging follows emit_atomic's reserved_gprs, independently of the
        // ordinary allocator's vector-boundary exclusion of R8.
        let reserved = if mir.aligned_base == 0 {
            0
        } else {
            1 << R::R12.code()
        };
        let busy = occupied | self.pinned | addresses | reserved;
        let mut free = allocatable & !busy;
        if free == 0 {
            free = ((1 << R::R10.code()) | (1 << R::R11.code())) & !busy;
        }
        let mut home = None;
        if free == 0 {
            free = allocatable & !(addresses | reserved);
            if free == 0 {
                return Err(Error::Construction(
                    "no register for atomic compare-exchange".into(),
                ));
            }
            home = Some(self.frame_alloc(8, 8)?);
        }
        let code = free.trailing_zeros();
        let r = ORDER
            .into_iter()
            .chain([R::R10, R::R11])
            .find(|r| u32::from(r.code()) == code)
            .ok_or(Error::Limit)?;
        if SAVED.contains(&r) || r == R::RBP {
            self.preserve |= 1 << r.code();
        }
        self.written |= 1 << r.code();
        Ok((r, home))
    }
    pub(super) fn frame_alloc(&mut self, bytes: u64, align: u32) -> Result<i32, Error> {
        let align = u64::from(align.clamp(1, 16));
        let size = (u64::from(self.frame)
            .checked_add(bytes)
            .and_then(|n| n.checked_add(align - 1))
            .ok_or(Error::Limit)?)
            & !(align - 1);
        if size > 0x7fff_0000 {
            return Err(Error::Diagnostic(
                "stack frame exceeds the 32-bit displacement range".into(),
            ));
        }
        self.frame = u32::try_from(size).map_err(|_| Error::Limit)?;
        let offset = -i32::try_from(size).map_err(|_| Error::Limit)?;
        self.frame_objects.push(super::frame_layout::Object::new(
            offset,
            u32::try_from(bytes).map_err(|_| Error::Limit)?,
            u32::try_from(align).map_err(|_| Error::Limit)?,
        ));
        Ok(offset)
    }
    pub(super) fn slot(&self, slot: crate::lowir::model::SlotId) -> Result<i32, Error> {
        self.slots[slot.index()].ok_or(Error::Limit)
    }
    pub(super) fn layout_slots(
        &mut self,
        body: &Body,
        states: &[super::slot_analysis::Slot],
        facts: &Facts,
        addresses: &super::address_facts::Addresses,
        mir: &mut Function,
    ) -> Result<(), Error> {
        use super::slot_analysis::Slot;
        let owners = super::object_storage::shared_slots(body, facts, states);
        let sharing = |index: usize| {
            owners
                .get(index)
                .copied()
                .flatten()
                .filter(|owner| owner.index() != index)
        };
        for (slot, state) in body.slots().iter().zip(states) {
            if matches!(state, Slot::Memory) && matches!(slot.ty, Type::Object { .. }) {
                mir.aligned_base = mir.aligned_base.max(abi::value_align(slot.ty)?);
            }
        }
        if mir.aligned_base <= 16 {
            mir.aligned_base = 0;
        }
        let mut area = 0_u64;
        for (i, (slot, state)) in body.slots().iter().zip(states).enumerate() {
            if !matches!(state, Slot::Memory)
                || abi::value_align(slot.ty)? <= 16
                || sharing(i).is_some()
            {
                continue;
            }
            let align = u64::from(abi::value_align(slot.ty)?);
            area = (area + align - 1) & !(align - 1);
            self.slots[i] = Some(i32::try_from(area).map_err(|_| Error::Limit)?);
            area += (u64::from(abi::value_bytes(slot.ty)?) + 7) & !7;
        }
        if area == 0 {
            mir.aligned_base = 0;
        } else {
            mir.aligned_disp = self
                .frame_alloc(area + u64::from(mir.aligned_base), 16)?
                .wrapping_add(16);
            mir.aligned_bytes = u32::try_from(area).map_err(|_| Error::Limit)?;
            self.preserve |= 1 << R::R12.code();
            self.excluded |= 1 << R::R12.code();
        }
        for (i, (slot, state)) in body.slots().iter().zip(states).enumerate() {
            if !matches!(state, Slot::Memory) {
                continue;
            }
            let ty = slot.ty;
            let bytes = (u64::from(abi::value_bytes(ty)?) + 7) & !7;
            let mut align = abi::value_align(ty)?;
            if matches!(ty, Type::Object { .. }) {
                align = align.max(8);
            }
            let offset = if let Some(owner) = sharing(i) {
                self.slot(owner)?
            } else if mir.aligned_base != 0 && align > 16 {
                self.slots[i]
                    .ok_or(Error::Limit)?
                    .wrapping_add(mir.aligned_disp)
            } else {
                self.frame_alloc(bytes.max(8), align)?
            };
            self.slots[i] = Some(offset);
            mir.bindings.push(Binding {
                name: Some(slot.name),
                ty,
                offset,
                kind: BindingKind::Slot,
            });
        }
        for index in 0..body.parameter_count {
            if self.locations[index].home.is_some() {
                continue;
            }
            let parameter = body.temp_id(index).ok_or(Error::Limit)?;
            if let Some(slot) =
                super::object_storage::parameter_home(body, facts, addresses, states, parameter)?
            {
                let offset = self.slot(slot)?;
                self.locations[index].home = Some(offset);
                mir.bindings.push(Binding {
                    name: Some(body.temp(parameter).name),
                    ty: body.temp(parameter).ty.ok_or(Error::Limit)?,
                    offset,
                    kind: BindingKind::Parameter,
                });
            }
        }
        Ok(())
    }
    pub(super) fn stored(&self, temp: TempId) -> bool {
        self.locations[temp.index()].stored
    }
    pub(super) fn mark_stored(&mut self, temp: TempId) {
        self.locations[temp.index()].stored = true;
    }
    pub(super) fn home(
        &mut self,
        body: &Body,
        temp: TempId,
        mir: &mut Function,
    ) -> Result<i32, Error> {
        if let Some(home) = self.locations[temp.index()].home {
            return Ok(home);
        }
        let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
        let bytes = abi::value_bytes(ty)?.max(1).div_ceil(8) * 8;
        let align = abi::value_align(ty)?.clamp(8, 16);
        let offset = self.frame_alloc(u64::from(bytes), align)?;
        self.locations[temp.index()].home = Some(offset);
        mir.homes += 1;
        mir.bindings.push(Binding {
            name: Some(body.temp(temp).name),
            ty,
            offset,
            kind: if temp.index() < body.parameter_count {
                BindingKind::Parameter
            } else {
                BindingKind::Temp
            },
        });
        Ok(offset)
    }
    pub(super) fn enter_block(
        &mut self,
        body: &Body,
        facts: &Facts,
        addresses: &super::address_facts::Addresses,
        block: BlockId,
    ) {
        for owner in &mut self.owners {
            if let Some(temp) = owner.take() {
                self.locations[temp.index()].reg = None;
                self.work.get_mut().releases += 1;
            }
        }
        self.restore_global_registers();
        if !self.globals.splits.homes.is_empty() {
            for k in self.globals.splits.home_first[block.index()]
                ..self.globals.splits.home_first[block.index() + 1]
            {
                let temp = self.globals.splits.homes[k];
                if let Some(register) = self.globals.registers[temp.index()].take() {
                    self.cleared.push((temp, register));
                }
            }
        }
        self.dying.clear();
        self.block = Some(block);
        self.pinned = self.globals.reserved[block.index()];
        self.block_globals = 0;
        self.written = 0;
        self.exit_block = facts.flow.successors(block).is_empty();
        self.last[..body.parameter_count.min(32)].fill(None);
        for index in
            self.globals.binding_first[block.index()]..self.globals.binding_first[block.index() + 1]
        {
            if self.globals.precise {
                let entry = self.globals.entries[index];
                self.last[entry.temp.index()] = entry.release;
                if entry.live_in
                    && let Some(reg) = self.globals.registers[entry.temp.index()]
                {
                    self.bind(entry.temp, reg);
                    self.block_globals |= 1 << reg.code();
                }
            } else {
                let temp = self.globals.bindings[index];
                self.last[temp.index()] = None;
                if let Some(reg) = self.globals.registers[temp.index()] {
                    self.bind(temp, reg);
                }
            }
        }
        let mut late = [None; 16];
        let mut early = [None; 16];
        for pos in facts.flow.ranges[block.index()].clone() {
            let (clobber, before) = facts.clobbers[pos];
            for reg in 0..16 {
                if clobber & (1 << reg) != 0 && late[reg].is_none() {
                    late[reg] = Some(pos);
                }
                if before & (1 << reg) != 0 && early[reg].is_none() {
                    early[reg] = Some(pos);
                }
            }
        }
        for pos in facts.flow.ranges[block.index()].clone() {
            let inst = body.instruction(facts.flow.instructions[pos]);
            if matches!(inst.kind, crate::lowir::model::InstructionKind::Phi(..))
                || !facts.active[pos]
                    && facts.reads[pos] == pos
                    && !inst.result.is_some_and(|t| addresses.lazy[t.index()])
            {
                continue;
            }
            let at = facts.reads[pos];
            body.visit_values(&inst.kind, |v| {
                for temp in super::global_placement::parts(body, facts, addresses, v)
                    .into_iter()
                    .flatten()
                {
                    if !self.globals.precise && self.globals.registers[temp.index()].is_some() {
                        self.last[temp.index()] =
                            Some(self.last[temp.index()].map_or(at, |old| old.max(at)));
                    }
                    if temp.index() < body.parameter_count
                        && facts.global[temp.index()]
                        && self.globals.registers[temp.index()].is_none()
                        && let Some(reg) = self.incoming[temp.index()]
                        && reg.code() < 16
                        && late[usize::from(reg.code())].is_none_or(|first| at <= first)
                        && early[usize::from(reg.code())].is_none_or(|first| at < first)
                    {
                        self.last[temp.index()] =
                            Some(self.last[temp.index()].map_or(at, |old| old.max(at)));
                    }
                }
            });
        }
        self.available_in = if block == body.layout[0] {
            (0..body.parameter_count.min(32))
                .filter(|&p| {
                    facts.global[p]
                        && self.globals.registers[p].is_none()
                        && self.incoming[p].is_some()
                })
                .fold(0, |mask, p| mask | (1 << p))
        } else {
            let preds = facts.flow.predecessors(block);
            if preds.is_empty() || facts.flow.handlers[block.index()] {
                0
            } else {
                preds.iter().fold(u32::MAX, |mask, b| {
                    mask & if self.emitted[b.index()] {
                        self.available[b.index()]
                    } else {
                        0
                    }
                })
            }
        };
        if block != body.layout[0] {
            for p in 0..body.parameter_count.min(32) {
                if self.available_in & (1 << p) == 0
                    || self.globals.registers[p].is_some()
                    || self.last[p].is_none()
                {
                    continue;
                }
                if let Some(reg) = self.incoming[p]
                    && self.owners[usize::from(reg.code())].is_none()
                    && self.pinned & (1 << reg.code()) == 0
                    && (!self.globals.precise
                        || self.globals.defined[block.index()] & (1 << reg.code()) == 0)
                    && let Some(temp) = body.temp_id(p)
                {
                    self.bind(temp, reg);
                }
            }
        }
        self.written = 0;
    }
    pub(super) fn note_clobbers(&mut self, mask: u32) {
        self.written |= mask;
    }
    pub(super) fn finish_block(&mut self, block: BlockId) {
        let written = self.written
            | if self.globals.precise {
                self.block_globals
            } else {
                self.pinned
            };
        self.available[block.index()] = self.available_in;
        for p in 0..self.incoming.len().min(32) {
            if self.incoming[p].is_some_and(|r| written & (1 << r.code()) != 0) {
                self.available[block.index()] &= !(1 << p);
            }
        }
        self.emitted[block.index()] = true;
    }
    pub(super) fn entry(
        &mut self,
        body: &Body,
        facts: &Facts,
        mir: &mut Function,
    ) -> Result<Vec<super::parallel_transfer::Transfer>, Error> {
        use super::parallel_transfer::Transfer;
        if mir.layout.indirect_result {
            let home = self.frame_alloc(8, 8)?;
            self.sret_home = Some(home);
            mir.instructions.push(Instruction {
                volatile: false,
                op: Op::Scalar(ScalarOp::Store(
                    Operand::Frame(home),
                    Type::Ptr,
                    Operand::Register(R::RDI),
                )),
                location: None,
            });
        }
        if mir.variadic && self.variadic.is_none() {
            let area = super::variadic::SaveArea::allocate(self, &mir.layout)?;
            self.variadic = Some(area);
            area.entry(mir)?;
        }
        if mir.object_model == super::object_compile::ObjectModel::Host {
            super::host_parameters::normalize(body, facts, mir, &self.host_parameters)?;
        }
        for (index, parameter) in body.temps().iter().take(body.parameter_count).enumerate() {
            if facts.intervals[index].uses == 0 {
                continue;
            }
            let ty = parameter.ty.ok_or(Error::Limit)?;
            if super::aggregate::memory_value(ty) {
                continue;
            }
            if ty.float_rank() != 0 {
                continue;
            }
            super::memory::scalar(ty)?;
            let temp = body.temp_id(index).ok_or(Error::Limit)?;
            if let Some(reg) = self.incoming[index]
                && self.globals.registers[index].is_none()
                && facts.intervals[index].last.is_some()
                && facts.intervals[index].forbid & (1 << reg.code()) == 0
                && self.pinned & (1 << reg.code()) == 0
                && self.owners[usize::from(reg.code())].is_none()
            {
                self.bind(temp, reg);
            }
        }
        let incoming = mir
            .layout
            .arguments
            .iter()
            .enumerate()
            .filter(|(i, _)| facts.intervals[*i].uses != 0)
            .filter_map(|(_, arg)| match mir.layout.parts(arg) {
                [part] => part.register,
                _ => None,
            })
            .fold(0, |mask, r| mask | (1 << r.code()));
        let mut transfers = Vec::new();
        for (index, parameter) in body.temps().iter().take(body.parameter_count).enumerate() {
            if facts.intervals[index].uses == 0 {
                continue;
            }
            let temp = body.temp_id(index).ok_or(Error::Limit)?;
            let ty = parameter.ty.ok_or(Error::Limit)?;
            if super::aggregate::memory_value(ty) {
                let argument = &mir.layout.arguments[index];
                let parts = mir.layout.parts(argument);
                if parts.is_empty() {
                    self.home(body, temp, mir)?;
                } else if parts[0].register.is_none() {
                    self.locations[index].home = Some(16_i32.wrapping_add(parts[0].stack));
                } else {
                    let home = self.home(body, temp, mir)?;
                    let parts = mir.layout.parts(&mir.layout.arguments[index]);
                    for &part in parts {
                        transfers.push(Transfer {
                            destination: Operand::Frame(
                                home.wrapping_add(part.offset.cast_signed()),
                            ),
                            source: Operand::Register(part.register.ok_or(Error::Limit)?),
                            ty: part.move_type(),
                            source_type: None,
                            address: false,
                        });
                    }
                }
                continue;
            }
            let [part] = mir.layout.parts(&mir.layout.arguments[index]) else {
                return Err(unsupported());
            };
            let source = if let Some(reg) = part.register {
                Operand::Register(reg)
            } else {
                Operand::Frame(16_i32.wrapping_add(part.stack))
            };
            if ty.float_rank() != 0 {
                if part.register.is_none() {
                    self.locations[index].home = source.frame_offset();
                } else {
                    self.locations[index].homed = !facts.global[index];
                    transfers.push(Transfer {
                        destination: Operand::Frame(self.home(body, temp, mir)?),
                        source,
                        ty,
                        source_type: None,
                        address: false,
                    });
                }
                continue;
            }
            let destination = if let Some(reg) = self.globals.registers[index] {
                if part.register.is_none() {
                    self.locations[index].home = source.frame_offset();
                }
                Operand::Register(reg)
            } else if part.register.is_none() {
                let home = self.home(body, temp, mir)?;
                self.locations[index].homed = true;
                mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Scalar(ScalarOp::Load(R::RAX, ty, source)),
                    location: None,
                });
                mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Scalar(ScalarOp::Store(
                        Operand::Frame(home),
                        ty,
                        Operand::Register(R::RAX),
                    )),
                    location: None,
                });
                continue;
            } else if facts.global[index] {
                Operand::Frame(self.home(body, temp, mir)?)
            } else if self.locations[index].reg == part.register {
                continue;
            } else if let Some(reg) = self.allocate(facts, temp, [None, None], incoming) {
                self.bind(temp, reg);
                Operand::Register(reg)
            } else {
                self.locations[index].homed = true;
                Operand::Frame(self.home(body, temp, mir)?)
            };
            transfers.push(Transfer {
                destination,
                source,
                ty,
                source_type: None,
                address: false,
            });
        }
        Ok(transfers)
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
        self.dies(facts, temp, pos).then_some(r)
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
    fn dies(&self, facts: &Facts, temp: TempId, pos: usize) -> bool {
        if self.globals.registers[temp.index()].is_some() {
            return if self.globals.precise {
                self.last[temp.index()].is_some_and(|last| last <= pos)
            } else {
                self.exit_block && self.last[temp.index()].is_none_or(|last| last <= pos)
            };
        }
        let own =
            facts.definitions[temp.index()].map_or(facts.flow.blocks[0], |p| facts.flow.blocks[p]);
        let last = if self.block == Some(own) {
            facts.intervals[temp.index()].last
        } else {
            self.last[temp.index()]
        };
        last.is_none_or(|last| last <= pos)
    }
    pub(super) fn release(&mut self, facts: &Facts, pos: usize) {
        for reg in 0..self.owners.len() {
            if let Some(temp) = self.owners[reg]
                && self.dies(facts, temp, pos)
            {
                self.owners[reg] = None;
                self.work.get_mut().releases += 1;
                self.pinned &= !(1 << reg);
                self.written |= 1 << reg;
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
        if let Some(r) = self.globals.registers[temp.index()] {
            if self.globals.precise {
                self.bind(temp, r);
            }
            self.work.get_mut().defined_in_plan += 1;
            return r;
        }
        if let Some(r) = self.allocate(facts, temp, prefs, exclude) {
            if facts.global[temp.index()] {
                self.work.get_mut().defined_other_register += 1;
            }
            self.bind(temp, r);
            r
        } else {
            self.locations[temp.index()].homed = true;
            if !facts.global[temp.index()]
                && facts.intervals[temp.index()]
                    .last
                    .is_some_and(|last| last > pos)
            {
                mir.spills += 1;
            } else if facts.global[temp.index()] {
                self.work.get_mut().defined_frame += 1;
            }
            if self.classes[temp.index()] == RegisterClass::Vector {
                // The selector reserves xmm14 as its result scratch.
                R::XMM14
            } else if exclude & (1 << R::R10.code()) != 0 {
                R::R11
            } else {
                R::R10
            }
        }
    }
    pub(super) fn finish(
        &mut self,
        body: &Body,
        facts: &Facts,
        temp: TempId,
        r: R,
        mir: &mut Function,
        location: Option<crate::lowir::model::DebugLocation>,
    ) -> Result<(), Error> {
        if self.globals.registers[temp.index()].is_some() {
            return Ok(());
        }
        if self.locations[temp.index()].homed || facts.global[temp.index()] {
            let home = self.home(body, temp, mir)?;
            let ty = body.temp(temp).ty.ok_or(Error::Limit)?;
            mir.instructions.push(Instruction {
                volatile: false,
                op: if r.code() >= 16 {
                    Op::FloatCopy(super::float_transport::FloatCopy {
                        ty,
                        destination: Operand::Frame(home),
                        source: Operand::Register(r),
                    })
                } else {
                    Op::Scalar(ScalarOp::Store(
                        Operand::Frame(home),
                        ty,
                        Operand::Register(r),
                    ))
                },
                location,
            });
        }
        if self.locations[temp.index()].homed {
            if self.locations[temp.index()].reg == Some(r)
                && self.owners[usize::from(r.code())] == Some(temp)
            {
                self.owners[usize::from(r.code())] = None;
                self.work.get_mut().releases += 1;
            }
            self.locations[temp.index()].reg = None;
        }
        Ok(())
    }
    pub(super) fn excluded(&self) -> u32 {
        self.excluded
    }
    pub(super) fn force_home(&mut self, temp: TempId) {
        self.locations[temp.index()].homed = true;
    }
    pub(super) fn finish_frame(&mut self, mir: &mut Function) -> Result<(), Error> {
        if mir.scratch_bytes != 0 {
            mir.scratch_offset = self.frame_alloc(48, 16)?;
        }
        let frame = (u64::from(self.frame) + 8 * u64::from(self.preserve.count_ones()) + 15) & !15;
        mir.stack_size = u32::try_from(frame).map_err(|_| Error::Limit)?;
        mir.preserve = self.preserve;
        mir.allocation_work = self.work.get();
        Ok(())
    }
    pub(super) fn restore_global_registers(&mut self) {
        for (temp, register) in self.cleared.drain(..) {
            self.globals.registers[temp.index()] = Some(register);
        }
    }
    pub(super) fn optimize_frame(&self, mir: &mut Function) -> Result<(), Error> {
        super::frame_lifetimes::forward_copies(mir, &self.frame_objects);
        let dropped = super::frame_lifetimes::drop_stores(mir, &self.frame_objects)?;
        mir.dead_frame_stores += dropped;
        super::frame_lifetimes::drop_unused(mir);
        super::frame_layout::order(mir, &self.frame_objects);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{InstructionKind as I, Program},
    };

    #[test]
    fn atomic_borrowing_preserves_owners_and_reserves_only_the_aligned_base() -> Result<(), Error> {
        let mut program = Program::default();
        let mut builder =
            FunctionBuilder::new(&mut program, "staging", &[("value", Type::I64)], Type::I64)?;
        builder.block("entry")?;
        let value = builder.parameter(0)?;
        builder.effect(I::Return(Type::I64, Some(value)))?;
        builder.finish()?;
        let function = &program.functions()[0];
        let body = function.body.as_ref().ok_or(Error::Limit)?;
        let layout = abi::Layout::classify(program.signature(function.signature))?;
        let flow = super::super::control_flow::Flow::analyze(body)?;
        let slots = super::super::slot_analysis::analyze(body, &flow)?;
        let mut facts = Facts::analyze(body, &slots, flow, 0)?;
        let addresses = super::super::address_facts::Addresses::analyze(
            &program,
            body,
            &mut facts,
            super::super::object_compile::ObjectModel::Course,
        )?;
        let globals =
            super::super::global_placement::Globals::analyze(body, &facts, &addresses, &layout, 0)?;
        let mut mir = Function::new(
            program.find_symbol(function.name).ok_or(Error::Limit)?,
            function.signature,
            body.block(body.layout[0]).name,
            layout,
        );
        let mut placement = Placement::new(body, &mir.layout, globals, true, 1 << R::R8.code());
        let temp = body.temp_id(0).ok_or(Error::Limit)?;
        for r in ORDER {
            placement.owners[usize::from(r.code())] = Some(temp);
        }
        placement.owners[usize::from(R::R8.code())] = None;
        placement.owners[usize::from(R::R12.code())] = None;
        mir.aligned_base = 64;
        let addresses = (1 << R::RAX.code()) | (1 << R::R10.code()) | (1 << R::R11.code());
        let owners = placement.owners;
        assert_eq!(placement.atomic_staging(addresses, &mir)?, (R::R8, None));
        assert_eq!(placement.owners, owners);
        placement.owners[usize::from(R::R8.code())] = Some(temp);
        let owners = placement.owners;
        assert_eq!(
            placement.atomic_staging(addresses, &mir)?,
            (R::RCX, Some(-8))
        );
        assert_eq!(placement.owners, owners);
        placement.owners[usize::from(R::RBX.code())] = None;
        assert_eq!(placement.atomic_staging(addresses, &mir)?, (R::RBX, None));
        assert_ne!(placement.preserve & (1 << R::RBX.code()), 0);
        Ok(())
    }
}
