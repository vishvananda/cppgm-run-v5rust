// Translated from lower/intrinsic_lowering.cpp::asm_statement. See NOTICE.
//! Each statement binds its operands once, then consumes semantic lifted operations.
use super::{
    Error,
    atomic_intrinsics::AtomicObject,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{AtomicUpdate, Binary, Fence, MemoryOrder as Order, Type, Unary},
    },
    sema::lowering::{AsmInstruction, AsmOperation as Op, AtomicArithmetic, TypeId},
    syntax::ast,
};

struct Bound {
    address: Option<Value>,
    home: Option<Value>,
    value: Option<RValue>,
    ty: Type,
    source: TypeId,
}
impl Bound {
    fn place(&self) -> Result<Value, Error> {
        self.home
            .or(self.address)
            .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                "asm operand without address",
            )))
    }
}

fn memory(ty: Type) -> Facts {
    Facts::memory(
        false,
        (ty.bits() != 0 || ty.float_rank() != 0 || ty == Type::Ptr)
            && !matches!(ty, Type::I1 | Type::I8 | Type::U8),
    )
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn asm_statement(
        &mut self,
        id: ast::StmtId,
        assembly: ast::AsmStatement,
    ) -> Result<(), Error> {
        let program = self.graph.lower_assembly(id)?.clone();
        if let Some(message) = program.unlifted {
            return Err(Error::Semantic(crate::sema::Error::Diagnostic {
                location: self.tree.statements.get(id).location,
                message,
            }));
        }
        let outputs = self.tree.asm_operands.get(assembly.outputs).len();
        let mut bound = Vec::<Bound>::with_capacity(program.registers.len());
        for list in [assembly.outputs, assembly.inputs] {
            for operand in self.tree.asm_operands.get(list).to_vec() {
                let index = bound.len();
                let output = index < outputs;
                let expr = operand.expression;
                let facts = self.graph.lower_expression(expr)?;
                let ty = types::storage(&*self.graph, facts.ty)?;
                let mut binding = Bound {
                    address: None,
                    home: None,
                    value: None,
                    ty,
                    source: facts.ty,
                };
                if !(facts.folded.is_some() && !output && !program.registers[index]) {
                    let first = self.lifetime.cleanups.len();
                    let region = self.begin_full_expression(
                        Some(expr),
                        super::temporary_scan::ExpressionUse::Value,
                    )?;
                    if output || facts.is_place {
                        binding.address = Some(self.address(expr)?);
                    } else {
                        binding.value = Some(self.operand(expr, Context::Argument)?);
                    }
                    if !output && facts.is_place && !matches!(ty, Type::Object { .. }) {
                        binding.value =
                            Some(self.emit(I::Load(memory(ty), ty, binding.place()?), facts.ty)?);
                    }
                    self.destroy_temporaries(first)?;
                    self.end_lifetime_expression(region)?;
                }
                bound.push(binding);
            }
        }
        for index in 0..outputs {
            if !program.registers[index] {
                continue;
            }
            let source = bound[index].source;
            let ty = bound[index].ty;
            let home = self.generated_slot("asm", ty)?;
            let value = if let Some(tied) = program.tied[index] {
                if let Some(value) = bound[tied].value {
                    value
                } else {
                    self.emit(
                        I::Load(memory(bound[tied].ty), bound[tied].ty, bound[tied].place()?),
                        source,
                    )?
                }
            } else {
                self.emit(I::Load(memory(ty), ty, bound[index].place()?), source)?
            };
            self.effect(I::Store(memory(value.ty), value.ty, value.value, home))?;
            bound[index].home = Some(self.storage_address(home, source)?);
        }
        for instruction in &program.instructions {
            self.asm_instruction(*instruction, outputs, &bound)?;
        }
        for operand in &bound[..outputs] {
            if let Some(home) = operand.home {
                let value = self.emit(
                    I::Load(memory(operand.ty), operand.ty, home),
                    operand.source,
                )?;
                self.effect(I::Store(
                    memory(operand.ty),
                    operand.ty,
                    value.value,
                    operand
                        .address
                        .ok_or(crate::sema::Error::Invariant("asm output address"))?,
                ))?;
            }
        }
        if program.barrier {
            self.effect(I::Runtime(R::Fence(
                Fence::Signal,
                Order::SequentiallyConsistent,
            )))?;
        }
        Ok(())
    }

    fn asm_instruction(
        &mut self,
        instruction: AsmInstruction,
        outputs: usize,
        operands: &[Bound],
    ) -> Result<(), Error> {
        let AsmInstruction {
            operation,
            locked,
            destination,
            source,
            bytes,
            immediate,
        } = instruction;
        if operation == Op::Fence {
            return self.effect(I::Runtime(R::Fence(
                Fence::Thread,
                Order::SequentiallyConsistent,
            )));
        }
        let ty = match bytes {
            1 => Type::I8,
            2 => Type::I16,
            4 => Type::I32,
            _ => Type::I64,
        };
        let source_type = operands[destination].source;
        let target = operands[destination].place()?;
        let value = if let Some(index) = source {
            let operand = &operands[index];
            if index < outputs || operand.value.is_none() {
                self.emit(I::Load(memory(ty), ty, operand.place()?), source_type)?
            } else if let Some(value) = operand.value {
                if value.ty == ty {
                    value
                } else {
                    let bits = self.generated_slot("asm", value.ty)?;
                    self.effect(I::Store(memory(value.ty), value.ty, value.value, bits))?;
                    let address = self.storage_address(bits, source_type)?;
                    self.emit(I::Load(memory(ty), ty, address), source_type)?
                }
            } else {
                return Err(Error::Semantic(crate::sema::Error::Invariant(
                    "asm input value",
                )));
            }
        } else {
            self.asm_integer(ty, immediate, source_type)?
        };
        let value = if operation == Op::Not {
            self.asm_integer(ty, u64::MAX, source_type)?
        } else {
            value
        };
        if operation == Op::Exchange {
            let previous = if locked {
                self.emit(
                    I::Runtime(R::AtomicUpdate(
                        AtomicUpdate::Exchange,
                        ty,
                        operands[destination]
                            .address
                            .ok_or(crate::sema::Error::Invariant("asm memory address"))?,
                        value.value,
                        Order::SequentiallyConsistent,
                    )),
                    source_type,
                )?
            } else {
                let previous = self.emit(I::Load(memory(ty), ty, target), source_type)?;
                self.effect(I::Store(memory(ty), ty, value.value, target))?;
                previous
            };
            return self.effect(I::Store(
                memory(ty),
                ty,
                previous.value,
                operands[source.ok_or(crate::sema::Error::Invariant("asm exchange source"))?]
                    .place()?,
            ));
        }
        if locked && operation == Op::Neg {
            let (state, old) = self.begin_atomic_bits_loop(target, source_type, ty, ty)?;
            let updated = self.emit(I::Unary(Unary::Neg, ty, old.value), source_type)?;
            return self.end_atomic_bits_loop(target, source_type, ty, state, updated);
        }
        if locked {
            let operation = match operation {
                Op::Add => AtomicArithmetic::Add,
                Op::Sub => AtomicArithmetic::Sub,
                Op::And => AtomicArithmetic::And,
                Op::Or => AtomicArithmetic::Or,
                _ => AtomicArithmetic::Xor,
            };
            self.atomic_update(
                AtomicObject {
                    source: source_type,
                    stored: ty,
                    ty,
                    target: operands[destination]
                        .address
                        .ok_or(crate::sema::Error::Invariant("asm memory address"))?,
                    element: source_type,
                    object_type: ty,
                },
                operation,
                false,
                value,
                Order::SequentiallyConsistent,
            )?;
            return Ok(());
        }
        let old = self.emit(I::Load(memory(ty), ty, target), source_type)?;
        let kind = match operation {
            Op::Neg => I::Unary(Unary::Neg, ty, old.value),
            Op::ByteSwap => I::Unary(Unary::ByteSwap, ty, old.value),
            _ => I::Binary(
                match operation {
                    Op::Add => Binary::Add,
                    Op::Sub => Binary::Sub,
                    Op::And => Binary::And,
                    Op::Or => Binary::Or,
                    _ => Binary::Xor,
                },
                Facts::default(),
                ty,
                old.value,
                value.value,
            ),
        };
        let updated = self.emit(kind, source_type)?;
        self.effect(I::Store(memory(ty), ty, updated.value, target))
    }

    fn asm_integer(&mut self, ty: Type, bits: u64, source: TypeId) -> Result<RValue, Error> {
        let value = self.builder.integer(u128::from(bits))?;
        Ok(RValue {
            value,
            ty,
            source,
            literal: false,
            bits: None,
        })
    }
}
