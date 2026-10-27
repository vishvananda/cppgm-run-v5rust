// Translated from lower/function_lowering.cpp::load/store and
// lower/intrinsic_lowering.cpp::atomic_bits/begin_atomic_loop/end_atomic_loop. See NOTICE.
//! Qualified scalar accesses and per-update compare/exchange scratch.
use super::{
    Error,
    atomic_intrinsics::AtomicObject,
    function::{Function, RValue},
    types,
};
use crate::{
    lowir::{
        model::{BlockId, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{MemoryOrder as Order, Type},
    },
    sema::lowering::{AtomicArithmetic, TypeId},
};

#[derive(Clone, Copy)]
pub(super) struct AtomicLoop {
    bits: Type,
    expected: Value,
    retry: BlockId,
}

fn bits(storage: Type) -> Option<Type> {
    Some(match storage {
        Type::F32 => Type::I32,
        Type::F64 | Type::Ptr => Type::I64,
        Type::F80 => Type::I128,
        Type::Object { bytes, .. } => match bytes {
            1 => Type::I8,
            2 => Type::I16,
            4 => Type::I32,
            8 => Type::I64,
            16 => Type::I128,
            _ => return None,
        },
        ty if ty.bits() != 0 => ty,
        _ => return None,
    })
}

impl Function<'_, '_, '_, '_> {
    pub(super) fn read_atomic_place(
        &mut self,
        place: Value,
        source: TypeId,
    ) -> Result<RValue, Error> {
        let ty = types::storage(&*self.graph, source)?;
        let facts = Facts::memory(self.graph.lower_cv(source) & 2 != 0, false);
        if ty.bits() != 0 || ty == Type::Ptr {
            return self.emit(
                I::Runtime(R::AtomicLoad(
                    ty,
                    place,
                    Order::SequentiallyConsistent,
                    facts,
                )),
                source,
            );
        }
        if let Some(bits) = bits(ty).filter(|&bits| bits != Type::I128) {
            let slot = self.generated_slot("atomic_result", bits)?;
            let value = self.emit(
                I::Runtime(R::AtomicLoad(
                    bits,
                    place,
                    Order::SequentiallyConsistent,
                    facts,
                )),
                source,
            )?;
            self.effect(I::Store(Facts::default(), bits, value.value, slot))?;
            let address = self.storage_address(slot, source)?;
            return self.emit(I::Load(Facts::default(), ty, address), source);
        }
        self.emit(
            I::Load(self.place_memory(place, source)?, ty, place),
            source,
        )
    }

    pub(super) fn write_atomic_place(
        &mut self,
        place: Value,
        source: TypeId,
        value: Value,
    ) -> Result<(), Error> {
        let ty = types::storage(&*self.graph, source)?;
        if ty.bits() != 0 || ty == Type::Ptr {
            return self.effect(I::Runtime(R::AtomicStore(
                ty,
                value,
                place,
                Order::SequentiallyConsistent,
            )));
        }
        if let Some(bits) = bits(ty).filter(|&bits| bits != Type::I128) {
            let slot = self.generated_slot("atomic_arg", ty)?;
            self.effect(I::Store(Facts::default(), ty, value, slot))?;
            let address = self.storage_address(slot, source)?;
            let value = self.emit(I::Load(Facts::default(), bits, address), source)?;
            return self.effect(I::Runtime(R::AtomicStore(
                bits,
                value.value,
                place,
                Order::SequentiallyConsistent,
            )));
        }
        self.effect(I::Store(
            self.place_memory(place, source)?,
            ty,
            value,
            place,
        ))
    }

    pub(super) fn update_atomic_integer(
        &mut self,
        place: Value,
        source: TypeId,
        operation: AtomicArithmetic,
        fetch_old: bool,
        value: RValue,
    ) -> Result<RValue, Error> {
        let ty = types::storage(&*self.graph, source)?;
        self.atomic_update(
            AtomicObject {
                source,
                stored: ty,
                ty,
                target: place,
                element: source,
                object_type: ty,
            },
            operation,
            fetch_old,
            value,
            Order::SequentiallyConsistent,
        )
    }

    pub(super) fn begin_atomic_place_loop(
        &mut self,
        place: Value,
        source: TypeId,
    ) -> Result<Option<(AtomicLoop, RValue)>, Error> {
        if self.graph.lower_cv(source) & 4 == 0 {
            return Ok(None);
        }
        let storage = types::storage(&*self.graph, source)?;
        let Some(bits) = bits(storage) else {
            return Ok(None);
        };
        self.begin_atomic_bits_loop(place, source, storage, bits)
            .map(Some)
    }

    pub(super) fn begin_atomic_bits_loop(
        &mut self,
        place: Value,
        source: TypeId,
        storage: Type,
        bits: Type,
    ) -> Result<(AtomicLoop, RValue), Error> {
        let expected = self.generated_slot("atomic_expected", bits)?;
        let seen = self.emit(
            I::Runtime(R::AtomicLoad(bits, place, Order::Relaxed, Facts::default())),
            source,
        )?;
        self.effect(I::Store(plain_memory(bits), bits, seen.value, expected))?;
        let retry = self.new_block("atomic_retry")?;
        self.effect(I::Jump(retry))?;
        self.place(retry)?;
        let old = if storage == bits {
            self.emit(I::Load(plain_memory(bits), storage, expected), source)?
        } else if storage == Type::Ptr {
            let value = self.emit(I::Load(plain_memory(bits), bits, expected), source)?;
            self.emit(I::Copy(storage, value.value), source)?
        } else {
            let address = self.storage_address(expected, source)?;
            self.emit(I::Load(plain_memory(storage), storage, address), source)?
        };
        Ok((
            AtomicLoop {
                bits,
                expected,
                retry,
            },
            old,
        ))
    }

    pub(super) fn end_atomic_place_loop(
        &mut self,
        place: Value,
        source: TypeId,
        state: AtomicLoop,
        updated: RValue,
    ) -> Result<(), Error> {
        let storage = types::storage(&*self.graph, source)?;
        self.end_atomic_bits_loop(place, source, storage, state, updated)
    }

    pub(super) fn end_atomic_bits_loop(
        &mut self,
        place: Value,
        source: TypeId,
        storage: Type,
        state: AtomicLoop,
        updated: RValue,
    ) -> Result<(), Error> {
        let desired = if storage == Type::Ptr {
            self.emit(I::Copy(state.bits, updated.value), source)?
        } else if storage != state.bits {
            let fresh = self.generated_slot("atomic_expected", state.bits)?;
            let old = self.emit(
                I::Load(plain_memory(state.bits), state.bits, state.expected),
                source,
            )?;
            self.effect(I::Store(
                plain_memory(state.bits),
                state.bits,
                old.value,
                fresh,
            ))?;
            let address = self.storage_address(fresh, source)?;
            self.effect(I::Store(
                plain_memory(storage),
                storage,
                updated.value,
                address,
            ))?;
            self.emit(I::Load(plain_memory(state.bits), state.bits, fresh), source)?
        } else {
            updated
        };
        let expected = self.storage_address(state.expected, source)?;
        let exchanged = self.emit(
            I::Runtime(R::AtomicCompareExchange {
                ty: state.bits,
                address: place,
                expected,
                desired: desired.value,
                success: Order::SequentiallyConsistent,
                failure: Order::SequentiallyConsistent,
            }),
            source,
        )?;
        let done = self.new_block("atomic_done")?;
        self.effect(I::Branch(exchanged.value, done, state.retry))?;
        self.place(done)
    }
}

fn plain_memory(ty: Type) -> Facts {
    Facts::memory(
        false,
        (ty.bits() != 0 || ty.float_rank() != 0 || ty == Type::Ptr)
            && !matches!(ty, Type::I1 | Type::I8 | Type::U8),
    )
}
