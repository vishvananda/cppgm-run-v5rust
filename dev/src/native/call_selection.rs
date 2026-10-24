//! Scalar ABI argument staging, call targets and result placement. See NOTICE.
use super::{
    abi::CALLER_SAVED,
    call_facts::{CallKind, CallPlan},
    memory::{Base, Memory},
    mir::{Binding, BindingKind, CallSite, Instruction, Op},
    parallel_transfer::{self, Transfer},
    scalar_mir::{Operand, ScalarOp},
    scalar_selection::Selection,
    value_facts::Alias,
    value_selection::unsupported,
    x86::Register as R,
};
use crate::lowir::{
    Error,
    model::{Call, InstructionKind as I, Program, SymbolDetail, TempId, Value},
    vocabulary::{Binary, Pass, Type},
};
#[derive(Default)]
pub(super) struct Staging {
    moves: Vec<Transfer>,
    constants: Vec<(Operand, Type, Operand)>,
    mask: u32,
}
impl Selection<'_> {
    pub(super) fn store_transfer(
        &mut self,
        destination: Operand,
        ty: Type,
        source: Operand,
        read: Option<Type>,
        address: bool,
    ) {
        if ty == Type::F80
            || (ty.float_rank() != 0 || ty == Type::I128)
                && (source.register().is_some_and(|r| r.code() >= 16)
                    || matches!(source, Operand::Immediate(_)))
        {
            self.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::FloatCopy(super::float_transport::FloatCopy {
                    ty,
                    destination,
                    source,
                }),
                location: self.location,
            });
        } else if address {
            self.emit(ScalarOp::Lea(R::R11, source));
            self.emit(ScalarOp::Store(
                destination,
                Type::Ptr,
                Operand::Register(R::R11),
            ));
        } else if source.is_memory() {
            let integer = if ty == Type::F32 {
                Type::I32
            } else if ty == Type::F64 {
                Type::I64
            } else {
                ty
            };
            self.load(R::R11, source, read.unwrap_or(integer));
            self.emit(ScalarOp::Store(
                destination,
                integer,
                Operand::Register(R::R11),
            ));
        } else if source.is_symbol() {
            self.load(R::R11, source, Type::I64);
            self.emit(ScalarOp::Store(
                destination,
                Type::Ptr,
                Operand::Register(R::R11),
            ));
        } else {
            self.emit(ScalarOp::Store(destination, ty, source));
        }
    }
    fn call_source(
        &mut self,
        value: Value,
        ty: Type,
        pass: Option<Pass>,
    ) -> Result<(Operand, Option<Type>, bool), Error> {
        if ty == Type::F80 {
            return Ok((self.aggregate_operand(value, ty)?, None, false));
        }
        if super::aggregate::memory_value(ty) {
            return Ok((self.aggregate_operand(value, ty)?, None, false));
        }
        let resolved = self.facts.resolve(self.body, value);
        if let Alias::Slot(slot) = resolved {
            return Ok((
                Operand::Frame(self.placement.slot(slot)?),
                None,
                pass.is_some(),
            ));
        }
        if pass == Some(Pass::ByAddress)
            && let Value::Temp(original) = value
            && self.body.temp(original).ty != Some(Type::Ptr)
        {
            let own = self.body.temp(original).ty.ok_or(Error::Limit)?;
            if !super::aggregate::memory_value(own) && own.float_rank() == 0 {
                super::memory::scalar(own)?;
            }
            let home = self.placement.frame_alloc(
                u64::from(super::abi::value_bytes(own)?).max(1).div_ceil(8) * 8,
                super::abi::value_align(own)?.max(8),
            )?;
            self.mir.bindings.push(Binding {
                name: Some(self.body.temp(original).name),
                ty: own,
                offset: home,
                kind: BindingKind::Temp,
            });
            let (source, read) = if let Alias::Temporary(t) = resolved {
                let op = self.temp_operand(t)?;
                let source_ty = self.body.temp(t).ty.ok_or(Error::Limit)?;
                let read = (super::abi::value_bytes(source_ty)? < super::abi::value_bytes(own)?)
                    .then_some(source_ty);
                (op, read)
            } else {
                (
                    if super::aggregate::memory_value(own) {
                        self.aggregate_operand(value, own)?
                    } else if own.float_rank() != 0 {
                        self.float_operand(value, own)?
                    } else {
                        self.operand(value, own, R::R10)?
                    },
                    None,
                )
            };
            if let Type::Object { .. } = own {
                self.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::Bulk(super::bulk::Bulk::new(
                        Operand::Frame(home),
                        Some(source),
                        u64::from(super::abi::value_bytes(own)?),
                        super::abi::value_align(own)?,
                    )),
                    location: self.location,
                });
            } else if own == Type::I128 {
                self.mir.instructions.push(Instruction {
                    volatile: false,
                    op: Op::WideMove {
                        destination: Operand::Frame(home),
                        source,
                    },
                    location: self.location,
                });
            } else {
                self.store_transfer(Operand::Frame(home), own, source, read, false);
            }
            return Ok((Operand::Frame(home), None, true));
        }
        if let Alias::Temporary(t) = resolved {
            let source = self.temp_operand(t)?;
            return Ok((
                source,
                source.is_memory().then_some(super::abi::move_type(
                    self.body.temp(t).ty.ok_or(Error::Limit)?,
                )),
                false,
            ));
        }
        Ok((
            if ty.float_rank() != 0 {
                self.float_operand(value, ty)?
            } else {
                self.operand(value, ty, R::R10)?
            },
            None,
            false,
        ))
    }
    fn stage_arguments(
        &mut self,
        call: &Call,
        result: Option<TempId>,
        plan: &CallPlan,
        out: &mut Staging,
    ) -> Result<(), Error> {
        out.moves.clear();
        out.constants.clear();
        out.mask = 0;
        for (index, &value) in self.body.arguments(call.arguments).iter().enumerate() {
            let ty = plan.types[index];
            let (source, read, address) = self.call_source(value, ty, plan.passes[index])?;
            let parts = plan.layout.parts(&plan.layout.arguments[index]);
            if parts.is_empty() {
                continue;
            }
            let part = parts[0];
            if part.register.is_some() {
                for &part in parts {
                    let destination = part.register.ok_or(Error::Limit)?;
                    out.mask |= 1 << destination.code();
                    out.moves.push(Transfer {
                        destination: Operand::Register(destination),
                        source: if super::aggregate::memory_value(ty) {
                            super::aggregate::part_source(source, part)
                        } else {
                            source
                        },
                        ty: if super::aggregate::memory_value(ty) {
                            part.move_type()
                        } else {
                            ty
                        },
                        source_type: read,
                        address,
                    });
                }
            } else {
                let dst = Operand::Memory(Memory::new(Base::Register(R::RSP), part.stack));
                if matches!(ty, Type::Object { .. }) {
                    self.mir.instructions.push(Instruction {
                        volatile: false,
                        op: Op::Bulk(super::bulk::Bulk::new(
                            dst,
                            Some(source),
                            u64::from(part.size),
                            8,
                        )),
                        location: self.location,
                    });
                } else if ty == Type::I128 {
                    self.mir.instructions.push(Instruction {
                        volatile: false,
                        op: Op::WideMove {
                            destination: dst,
                            source,
                        },
                        location: self.location,
                    });
                } else if !address
                    && ty.float_rank() == 0
                    && (matches!(source, Operand::Immediate(_)) || source.is_symbol())
                {
                    out.constants.push((dst, ty, source));
                } else {
                    self.store_transfer(dst, ty, source, read, address);
                }
            }
        }
        out.moves
            .sort_by_key(|m| m.destination.register().is_some_and(|r| r.code() >= 16));
        if plan.layout.indirect_result {
            let result = result.ok_or_else(unsupported)?;
            let source = Operand::Frame(self.placement.home(self.body, result, &mut self.mir)?);
            out.moves.push(Transfer {
                destination: Operand::Register(R::RDI),
                source,
                ty: Type::Void,
                source_type: None,
                address: true,
            });
            out.mask |= 1 << R::RDI.code();
        }
        if plan.variadic {
            out.moves.push(Transfer {
                destination: Operand::Register(R::RAX),
                source: Operand::Immediate(u128::from(plan.layout.vector_count())),
                ty: Type::Void,
                source_type: None,
                address: false,
            });
            out.mask |= 1 << R::RAX.code();
        }
        Ok(())
    }
    fn call_target(
        &mut self,
        program: &Program,
        call: &Call,
        moves: &mut Vec<Transfer>,
    ) -> Result<Operand, Error> {
        let callee = self.facts.resolve(self.body, call.callee);
        let storage = match callee {
            Alias::Symbol(s) if matches!(program.symbol(s).detail, SymbolDetail::Global(_)) => {
                Some((s, 0))
            }
            Alias::Temporary(t) => {
                self.addresses.expressions[t.index()].and_then(|e| match e.base {
                    super::address_facts::AddressBase::Symbol(s)
                        if e.index.is_none()
                            && matches!(program.symbol(s).detail, SymbolDetail::Global(_)) =>
                    {
                        Some((s, e.displacement))
                    }
                    _ => None,
                })
            }
            _ => None,
        };
        if let Some((s, disp)) = storage {
            let disp = i32::try_from(disp).map_err(|_| Error::Limit)?;
            if self.symbol_access(s) != super::symbol_access::SymbolAccess::Direct {
                moves.push(Transfer {
                    destination: Operand::Register(R::R11),
                    source: self.symbol_operand(s, disp),
                    ty: Type::Void,
                    source_type: None,
                    address: false,
                });
                return Ok(Operand::Memory(Memory::new(Base::Register(R::R11), 0)));
            }
            return Ok(Operand::Memory(Memory::new(Base::Symbol(s), disp)));
        }
        if let Alias::Symbol(s) = callee {
            return Ok(Operand::Symbol(s));
        }
        let target = self.operand(call.callee, Type::Ptr, R::R10)?;
        if (matches!(target, Operand::Immediate(_)) || target.is_symbol())
            || moves
                .iter()
                .any(|m| target.mask() & m.destination.mask() != 0)
        {
            moves.push(Transfer {
                destination: Operand::Register(R::R11),
                source: target,
                ty: Type::Void,
                source_type: None,
                address: false,
            });
            return Ok(Operand::Register(R::R11));
        }
        Ok(target)
    }
    pub(super) fn call(
        &mut self,
        program: &Program,
        result: Option<TempId>,
        call: &Call,
        plan: &CallPlan,
    ) -> Result<(), Error> {
        if plan.kind == CallKind::Trap {
            self.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::Trap,
                location: self.location,
            });
            return Ok(());
        }
        let stack = plan.layout.stack.wrapping_add(15) & !15;
        let saved_rsp = if plan.layout.stack_align > 16 {
            let home = self.placement.frame_alloc(8, 8)?;
            self.emit(ScalarOp::Store(
                Operand::Frame(home),
                Type::I64,
                Operand::Register(R::RSP),
            ));
            Some(home)
        } else {
            None
        };
        if stack != 0 {
            self.emit(ScalarOp::Binary(
                Binary::Sub,
                Type::I64,
                R::RSP,
                Operand::Immediate(u128::from(stack)),
            ));
        }
        if saved_rsp.is_some() {
            self.emit(ScalarOp::Binary(
                Binary::And,
                Type::I64,
                R::RSP,
                Operand::Immediate(u128::from(
                    (-i64::from(plan.layout.stack_align)).cast_unsigned(),
                )),
            ));
        }
        let mut staged = std::mem::take(&mut self.outgoing);
        self.stage_arguments(call, result, plan, &mut staged)?;
        let target = self.call_target(program, call, &mut staged.moves)?;
        let scratch = target.mask() & (1 << R::R11.code()) != 0;
        if scratch {
            for &(dst, ty, src) in &staged.constants {
                self.store_transfer(dst, ty, src, None, false);
            }
        }
        parallel_transfer::emit(self, &mut staged.moves)?;
        if !scratch {
            for &(dst, ty, src) in &staged.constants {
                self.load(R::R11, src, Type::I64);
                self.emit(ScalarOp::Store(dst, ty, Operand::Register(R::R11)));
            }
        }
        let used = result.is_some_and(|t| self.facts.intervals[t.index()].uses != 0);
        if plan.kind == CallKind::Memcpy && !used {
            self.mir.instructions.push(Instruction {
                volatile: false,
                op: Op::DynamicCopy,
                location: self.location,
            });
            self.mir.dynamic_copies += 1;
            self.outgoing = staged;
            return Ok(());
        }
        self.mir.instructions.push(Instruction {
            volatile: false,
            op: Op::Call(CallSite {
                target,
                arguments: staged.mask,
                stack: plan.layout.stack,
                variadic: plan.variadic,
                no_unwind: plan.no_unwind,
                no_return: plan.no_return,
                landing: None,
            }),
            location: self.location,
        });
        if let Some(home) = saved_rsp {
            self.emit(ScalarOp::Load(R::RSP, Type::I64, Operand::Frame(home)));
        } else if stack != 0 {
            self.emit(ScalarOp::Binary(
                Binary::Add,
                Type::I64,
                R::RSP,
                Operand::Immediate(u128::from(stack)),
            ));
        }
        if plan.layout.x87_result() || plan.layout.x87_pair_result() {
            let destination = if let Some(result) = result.filter(|_| used) {
                Some(Operand::Frame(self.placement.home(
                    self.body,
                    result,
                    &mut self.mir,
                )?))
            } else {
                None
            };
            self.floating(super::floating_mir::FloatingOp::Pop(destination));
            if plan.layout.x87_pair_result() {
                self.floating(super::floating_mir::FloatingOp::Pop(
                    destination.map(|value| value.displaced(16)),
                ));
            }
        } else if let Some(result) = result.filter(|_| used) {
            if super::aggregate::memory_value(call.result) {
                self.aggregate_call_result(result, &plan.layout)?;
            } else {
                let own = self.facts.level >= 1
                    && matches!(self.facts.resolve(self.body, call.callee), Alias::Symbol(symbol)
                        if program.function(program.symbol(symbol)).is_some_and(|function|
                            function.body.is_some() && function.metadata.binding != Some(crate::lowir::vocabulary::Binding::Weak)
                                && function.metadata.flags & 64 == 0));
                self.call_result(result, call.result, staged.mask, own)?;
            }
        }
        self.outgoing = staged;
        Ok(())
    }
    fn call_result(&mut self, temp: TempId, ty: Type, args: u32, own: bool) -> Result<(), Error> {
        if matches!(ty, Type::F32 | Type::F64) {
            let destination = self.result(temp, [Some(R::XMM0), None], 0);
            self.load(destination, Operand::Register(R::XMM0), ty);
            return self.finish(temp, destination);
        }
        let interval = self.facts.intervals[temp.index()];
        let general = ((1 << R::R8.code())
            | (1 << R::R9.code())
            | (1 << R::RDI.code())
            | (1 << R::RSI.code()))
            & !self.placement.excluded();
        let consumer = interval
            .consumer
            .map(|p| self.body.instruction(self.facts.flow.instructions[p]));
        if !self.facts.global[temp.index()]
            && interval.forbid & CALLER_SAVED != CALLER_SAVED
            && general & !args == 0
            && interval.uses == 1
            && consumer.is_some_and(|i| matches!(i.kind, I::Binary(..)))
        {
            self.normalize(R::RAX, ty);
            self.placement.force_home(temp);
            return self.finish(temp, R::RAX);
        }
        let stored=interval.uses==1&&interval.consumer==Some(self.pos+1)&&consumer.is_some_and(|inst|matches!(inst.kind,I::Store(_,t,v,address) if t==ty&&matches!(self.facts.resolve(self.body,v),Alias::Temporary(t) if t==temp)&&!matches!(self.facts.resolve(self.body,address),Alias::Temporary(t) if t==temp)));
        let terminal = stored
            || interval.uses == 1
                && interval.consumer == Some(self.pos + 1)
                && consumer.is_some_and(|i| matches!(i.kind,I::Return(t,_) if t==ty));
        let preference =
            (self.facts.level >= 1 || self.body.layout.len() == 1 || terminal).then_some(R::RAX);
        let destination = self.result(temp, [preference, None], 0);
        self.load(destination, Operand::Register(R::RAX), Type::I64);
        if !stored && !own {
            self.normalize(destination, ty);
        }
        self.finish(temp, destination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowir::{
        builder::FunctionBuilder,
        model::{Facts, Program},
    };
    fn target(program: &mut Program) -> Result<Value, Error> {
        let mut b = FunctionBuilder::new(
            program,
            "target",
            &[("a", Type::I64), ("b", Type::I64)],
            Type::I64,
        )?;
        b.block("entry")?;
        let a = b.parameter(0)?;
        b.effect(I::Return(Type::I64, Some(a)))?;
        b.finish()?;
        Ok(Value::Symbol(
            program
                .find_symbol(program.functions()[0].name)
                .ok_or(Error::Limit)?,
        ))
    }
    fn selected(program: &Program) -> Result<super::super::mir::Function, Error> {
        let f = &program.functions()[1];
        super::super::value_selection::select(
            program,
            program.find_symbol(f.name).ok_or(Error::Limit)?,
            f,
            0,
        )
    }
    #[test]
    fn swapped_arguments_save_the_first_destination_before_the_cycle() -> Result<(), Error> {
        let mut program = Program::default();
        let callee = target(&mut program)?;
        let mut b = FunctionBuilder::new(
            &mut program,
            "f",
            &[("a", Type::I64), ("b", Type::I64)],
            Type::I64,
        )?;
        b.block("entry")?;
        let a = b.parameter(0)?;
        let other = b.parameter(1)?;
        let arguments = b.arguments(&[other, a])?;
        let value = b.value(
            I::Call(Call {
                result: Type::I64,
                callee,
                arguments,
                copy_elision: false,
                argument_classes: Vec::new(),
                signature: None,
            }),
            "r",
        )?;
        b.effect(I::Return(Type::I64, Some(value)))?;
        b.finish()?;
        let mir = selected(&program)?;
        let transfers: Vec<_> = mir
            .instructions
            .iter()
            .filter_map(|i| {
                if let Op::Scalar(ScalarOp::Move(to, Operand::Register(from))) = i.op {
                    Some((to, from))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            transfers,
            [(R::R10, R::RDI), (R::RDI, R::RSI), (R::RSI, R::R10)]
        );
        Ok(())
    }
    #[test]
    fn a_call_result_survives_the_next_call_in_a_preserved_register() -> Result<(), Error> {
        let mut program = Program::default();
        let callee = target(&mut program)?;
        let mut b = FunctionBuilder::new(&mut program, "f", &[], Type::I64)?;
        b.block("entry")?;
        let zero = b.integer(0)?;
        let arguments = b.arguments(&[zero, zero])?;
        let first = b.value(
            I::Call(Call {
                result: Type::I64,
                callee,
                arguments,
                copy_elision: false,
                argument_classes: Vec::new(),
                signature: None,
            }),
            "first",
        )?;
        let second = b.value(
            I::Call(Call {
                result: Type::I64,
                callee,
                arguments,
                copy_elision: false,
                argument_classes: Vec::new(),
                signature: None,
            }),
            "second",
        )?;
        let sum = b.value(
            I::Binary(Binary::Add, Facts::default(), Type::I64, first, second),
            "sum",
        )?;
        b.effect(I::Return(Type::I64, Some(sum)))?;
        b.finish()?;
        let mir = selected(&program)?;
        let calls: Vec<_> = mir
            .instructions
            .iter()
            .enumerate()
            .filter_map(|(i, inst)| matches!(inst.op, Op::Call(_)).then_some(i))
            .collect();
        assert_eq!(calls.len(), 2);
        assert!(mir.preserve & (1 << R::RBX.code()) != 0);
        assert!(
            mir.instructions[calls[0] + 1..calls[1]]
                .iter()
                .any(|inst| matches!(
                    inst.op,
                    Op::Scalar(ScalarOp::Move(R::RBX, Operand::Register(R::RAX)))
                ))
        );
        Ok(())
    }
    #[test]
    fn by_address_narrow_alias_reads_its_storage_width_from_a_wider_home() -> Result<(), Error> {
        let mut program = Program::default();
        let mut b = FunctionBuilder::new(
            &mut program,
            "target",
            &[("address", Type::Ptr)],
            Type::Void,
        )?;
        b.block("entry")?;
        b.effect(I::Return(Type::Void, None))?;
        b.finish()?;
        let signature = program.functions()[0].signature;
        program.signature_mut(signature).parameters[0].pass = Some(Pass::ByAddress);
        let callee = Value::Symbol(
            program
                .find_symbol(program.functions()[0].name)
                .ok_or(Error::Limit)?,
        );
        let mut b = FunctionBuilder::new(&mut program, "f", &[("p", Type::I64)], Type::I64)?;
        b.block("entry")?;
        let p = b.parameter(0)?;
        let mut live = Vec::new();
        for offset in 1_u128..=5 {
            let amount = b.integer(offset)?;
            live.push(b.value(
                I::Binary(Binary::Add, Facts::default(), Type::I64, p, amount),
                &format!("v{offset}"),
            )?);
        }
        let zero = b.integer(0)?;
        let condition = b.value(
            I::Compare(crate::lowir::vocabulary::Compare::Ne, Type::I64, p, zero),
            "condition",
        )?;
        let narrow = b.value(I::Copy(Type::I8, condition), "narrow")?;
        let arguments = b.arguments(&[narrow])?;
        b.effect(I::Call(Call {
            result: Type::Void,
            callee,
            arguments,
            copy_elision: false,
            argument_classes: Vec::new(),
            signature: None,
        }))?;
        let mut sum = condition;
        for (index, value) in live.into_iter().enumerate() {
            sum = b.value(
                I::Binary(Binary::Add, Facts::default(), Type::I64, sum, value),
                &format!("sum{index}"),
            )?;
        }
        b.effect(I::Return(Type::I64, Some(sum)))?;
        b.finish()?;
        let mir = selected(&program)?;
        let home = mir
            .bindings
            .iter()
            .find(|binding| {
                binding.ty == Type::I64
                    && binding
                        .name
                        .is_some_and(|name| program.names.get(name) == "condition")
            })
            .ok_or(Error::Limit)?
            .offset;
        assert!(mir.instructions.windows(2).any(|pair|matches!(pair[0].op,Op::Scalar(ScalarOp::Load(R::R11,Type::I8,Operand::Frame(offset))) if offset==home)&&matches!(pair[1].op,Op::Scalar(ScalarOp::Store(Operand::Frame(_),Type::I8,Operand::Register(R::R11))))));
        Ok(())
    }
}
