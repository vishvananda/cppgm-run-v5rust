// Translated from lower/intrinsic_lowering.cpp::atomic_intrinsic/atomic_update. See NOTICE.
//! Atomic calls consume selected semantic operations; backend instructions stay unchanged.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{AtomicUpdate, Binary, Compare, Fence, MemoryOrder as Order, Type, Unary},
    },
    sema::lowering::{
        AtomicArithmetic as Arithmetic, AtomicIntrinsic, AtomicOperation as Op, AtomicOrder,
        TypeId, TypeView,
    },
    syntax::{ast, storage::List},
};
#[derive(Clone, Copy)]
struct AtomicObject {
    source: TypeId,
    stored: Type,
    ty: Type,
    target: Value,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn atomic_intrinsic(
        &mut self,
        id: ast::ExprId,
        arguments: List<ast::ExprId>,
        intrinsic: AtomicIntrinsic,
    ) -> Result<RValue, Error> {
        let arguments = self.tree.expr_lists.get(arguments);
        let source = self.graph.lower_expression(id)?.ty;
        let kind = intrinsic.operation;
        if matches!(kind, Op::ThreadFence | Op::SignalFence) {
            let order = self.atomic_order(intrinsic, arguments, 0)?;
            self.effect(I::Runtime(R::Fence(
                if kind == Op::ThreadFence {
                    Fence::Thread
                } else {
                    Fence::Signal
                },
                order,
            )))?;
            return self.zero(source);
        }
        let object = self.atomic_object(
            arguments[0],
            source,
            matches!(kind, Op::TestAndSet | Op::Clear),
        )?;
        let AtomicObject { ty, target, .. } = object;
        match kind {
            Op::Load(generic) => {
                let destination = if generic {
                    Some(self.operand(arguments[1], Context::Argument)?.value)
                } else {
                    None
                };
                let order = self.atomic_order(intrinsic, arguments, arguments.len() - 1)?;
                let value = self.emit(I::Runtime(R::AtomicLoad(ty, target, order)), source)?;
                if let Some(destination) = destination {
                    self.atomic_store_bits(object, value, destination)?;
                    self.zero(source)
                } else {
                    self.atomic_result(object, value)
                }
            }
            Op::Store(_) | Op::Clear => {
                let generic = matches!(kind, Op::Store(true));
                let value =
                    if kind == Op::Clear || (intrinsic.implied.is_some() && arguments.len() < 2) {
                        self.atomic_integer(object, 0)?
                    } else {
                        self.atomic_value(object, arguments[1], generic)?
                    };
                let order = self.atomic_order(intrinsic, arguments, arguments.len() - 1)?;
                self.effect(I::Runtime(R::AtomicStore(ty, value.value, target, order)))?;
                self.zero(source)
            }
            Op::Exchange(_) | Op::TestAndSet => {
                let generic = matches!(kind, Op::Exchange(true));
                let value = if kind == Op::TestAndSet {
                    self.atomic_integer(object, 1)?
                } else {
                    self.atomic_value(object, arguments[1], generic)?
                };
                let order = self.atomic_order(intrinsic, arguments, arguments.len() - 1)?;
                let previous = self.emit(
                    I::Runtime(R::AtomicUpdate(
                        AtomicUpdate::Exchange,
                        ty,
                        target,
                        value.value,
                        order,
                    )),
                    source,
                )?;
                if kind == Op::TestAndSet {
                    let zero = self.builder.integer(0)?;
                    return self.emit(I::Compare(Compare::Ne, ty, previous.value, zero), source);
                }
                if generic {
                    let destination = self.operand(arguments[2], Context::Argument)?.value;
                    self.atomic_store_bits(object, previous, destination)?;
                    self.zero(source)
                } else {
                    self.atomic_result(object, previous)
                }
            }
            Op::CompareExchange(generic) => {
                let expected = self.operand(arguments[1], Context::Argument)?.value;
                let desired = self.atomic_value(object, arguments[2], generic)?;
                self.operand(arguments[3], Context::Argument)?;
                let success = self.memory_order(arguments[4])?;
                let failure = self.memory_order(arguments[5])?;
                self.emit(
                    I::Runtime(R::AtomicCompareExchange {
                        ty,
                        address: target,
                        expected,
                        desired: desired.value,
                        success,
                        failure,
                    }),
                    source,
                )
            }
            Op::BoolCompareSwap | Op::ValCompareSwap => {
                let expected = self.generated_slot("atomic_expected", ty)?;
                let value = self.atomic_value(object, arguments[1], false)?;
                self.effect(I::Store(atomic_memory(ty), ty, value.value, expected))?;
                let desired = self.atomic_value(object, arguments[2], false)?;
                let expected_address = self.storage_address(expected, source)?;
                let result = self.emit(
                    I::Runtime(R::AtomicCompareExchange {
                        ty,
                        address: target,
                        expected: expected_address,
                        desired: desired.value,
                        success: Order::SequentiallyConsistent,
                        failure: Order::SequentiallyConsistent,
                    }),
                    source,
                )?;
                if kind == Op::BoolCompareSwap {
                    Ok(result)
                } else {
                    let value = self.emit(I::Load(atomic_memory(ty), ty, expected), source)?;
                    self.atomic_result(object, value)
                }
            }
            Op::Update(operation, fetch_old) => {
                let value = self.atomic_value(object, arguments[1], false)?;
                let order = self.atomic_order(intrinsic, arguments, 2)?;
                let value = self.atomic_update(object, operation, fetch_old, value, order)?;
                self.atomic_result(object, value)
            }
            _ => Err(crate::sema::Error::Invariant("non-atomic operation in lowering").into()),
        }
    }

    fn atomic_object(
        &mut self,
        argument: ast::ExprId,
        source: TypeId,
        byte: bool,
    ) -> Result<AtomicObject, Error> {
        let pointer = self.graph.lower_expression(argument)?.ty;
        let (TypeView::Pointer(element) | TypeView::Array { element, .. }) =
            self.graph.lower_type(pointer)?
        else {
            return Err(crate::sema::Error::Invariant("atomic target without pointer").into());
        };
        let mut stored = if byte {
            Type::U8
        } else {
            types::storage(self.graph, element)?
        };
        if stored.bits() == 0 && stored != Type::Ptr {
            let size = types::size_of(
                self.graph,
                element,
                self.tree.expressions.get(argument).location,
            )?;
            stored = match size {
                1 => Type::I8,
                2 => Type::I16,
                4 => Type::I32,
                _ => Type::I64,
            };
        }
        let ty = if stored == Type::Ptr {
            Type::I64
        } else {
            stored
        };
        let target = self.operand(argument, Context::Argument)?.value;
        Ok(AtomicObject {
            source,
            stored,
            ty,
            target,
        })
    }
    fn atomic_value(
        &mut self,
        object: AtomicObject,
        argument: ast::ExprId,
        through_pointer: bool,
    ) -> Result<RValue, Error> {
        let source = self.graph.lower_expression(argument)?;
        let value = if through_pointer {
            let value = self.operand(argument, Context::Argument)?;
            self.emit(
                I::Load(atomic_memory(object.stored), object.stored, value.value),
                object.source,
            )?
        } else if let TypeView::Class(class) = self.graph.lower_type(source.ty)? {
            // Legacy lock_release may read its first trailing operand. The
            // frozen value_operand materializes a class there, then reads the
            // object's integer width, without argument destruction ownership.
            let slot = self.generated_slot("argobj", types::storage(self.graph, source.ty)?)?;
            let address = self.storage_address(slot, source.ty)?;
            if source.is_place && self.graph.lower_class_copy(argument)?.is_none() {
                let from = self.address(argument)?;
                if !self.graph.lower_class_empty(class)? {
                    let (size, alignment) = types::layout(self.graph, source.ty)?;
                    self.effect(I::CopyObject(size, alignment, from, address))?;
                }
            } else {
                self.materialize_class(argument, source.ty, address)?;
            }
            self.emit(
                I::Load(atomic_memory(object.stored), object.stored, slot),
                object.source,
            )?
        } else {
            self.operand(argument, Context::Argument)?
        };
        if value.ty == Type::Ptr {
            self.emit(I::Copy(object.ty, value.value), object.source)
        } else {
            Ok(value)
        }
    }
    fn atomic_result(&mut self, object: AtomicObject, value: RValue) -> Result<RValue, Error> {
        if object.stored == Type::Ptr {
            self.emit(I::Copy(object.stored, value.value), object.source)
        } else {
            Ok(value)
        }
    }
    fn atomic_store_bits(
        &mut self,
        object: AtomicObject,
        value: RValue,
        destination: Value,
    ) -> Result<(), Error> {
        let value = self.atomic_result(object, value)?;
        self.effect(I::Store(
            atomic_memory(object.stored),
            object.stored,
            value.value,
            destination,
        ))
    }
    fn atomic_integer(&mut self, object: AtomicObject, bits: u64) -> Result<RValue, Error> {
        Ok(RValue {
            value: self.builder.integer(u128::from(bits))?,
            ty: object.ty,
            source: object.source,
            literal: false,
            bits: None,
        })
    }
    fn atomic_order(
        &mut self,
        intrinsic: AtomicIntrinsic,
        arguments: &[ast::ExprId],
        index: usize,
    ) -> Result<Order, Error> {
        match intrinsic.implied {
            Some(AtomicOrder::Acquire) => Ok(Order::Acquire),
            Some(AtomicOrder::Release) => Ok(Order::Release),
            Some(AtomicOrder::SequentiallyConsistent) => Ok(Order::SequentiallyConsistent),
            None => self.memory_order(arguments[index]),
        }
    }
    fn memory_order(&mut self, argument: ast::ExprId) -> Result<Order, Error> {
        let value = self.operand(argument, Context::Argument)?;
        Ok(if let Some(bits) = value.bits.filter(|_| value.literal) {
            i64::try_from(bits)
                .ok()
                .and_then(Order::from_number)
                .unwrap_or(Order::SequentiallyConsistent)
        } else {
            Order::SequentiallyConsistent
        })
    }
    fn atomic_update(
        &mut self,
        object: AtomicObject,
        operation: Arithmetic,
        fetch_old: bool,
        value: RValue,
        order: Order,
    ) -> Result<RValue, Error> {
        let AtomicObject {
            source, ty, target, ..
        } = object;
        if matches!(operation, Arithmetic::Add | Arithmetic::Sub) {
            let delta = if operation == Arithmetic::Sub {
                self.emit(I::Unary(Unary::Neg, ty, value.value), source)?
            } else {
                value
            };
            let updated = self.emit(
                I::Runtime(R::AtomicUpdate(
                    AtomicUpdate::AddFetch,
                    ty,
                    target,
                    delta.value,
                    order,
                )),
                source,
            )?;
            return if fetch_old {
                self.emit(
                    I::Binary(
                        Binary::Sub,
                        Facts::default(),
                        ty,
                        updated.value,
                        delta.value,
                    ),
                    source,
                )
            } else {
                Ok(updated)
            };
        }
        let expected = self.generated_slot("atomic_expected", ty)?;
        let seen = self.emit(
            I::Runtime(R::AtomicLoad(ty, target, Order::Relaxed)),
            source,
        )?;
        self.effect(I::Store(atomic_memory(ty), ty, seen.value, expected))?;
        let retry = self.new_block("atomic_retry")?;
        let done = self.new_block("atomic_done")?;
        self.effect(I::Jump(retry))?;
        self.place(retry)?;
        let previous = self.emit(I::Load(atomic_memory(ty), ty, expected), source)?;
        let bitwise = match operation {
            Arithmetic::Or => Binary::Or,
            Arithmetic::Xor => Binary::Xor,
            _ => Binary::And,
        };
        let mut updated = self.emit(
            I::Binary(bitwise, Facts::default(), ty, previous.value, value.value),
            source,
        )?;
        if operation == Arithmetic::Nand {
            updated = self.emit(I::Unary(Unary::BitNot, ty, updated.value), source)?;
        }
        let failure = match order {
            Order::Release => Order::Relaxed,
            Order::AcquireRelease => Order::Acquire,
            _ => order,
        };
        let expected_address = self.storage_address(expected, source)?;
        let exchanged = self.emit(
            I::Runtime(R::AtomicCompareExchange {
                ty,
                address: target,
                expected: expected_address,
                desired: updated.value,
                success: order,
                failure,
            }),
            source,
        )?;
        self.effect(I::Branch(exchanged.value, done, retry))?;
        self.place(done)?;
        Ok(if fetch_old { previous } else { updated })
    }
}
fn atomic_memory(ty: Type) -> Facts {
    Facts::memory(false, !matches!(ty, Type::I1 | Type::I8 | Type::U8))
}
