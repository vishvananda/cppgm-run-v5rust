// Translated from lower/dynamic_type_lowering.cpp::runtime_cast. See NOTICE.
//! Runtime searches preserve nulls, result ownership and the reference failure edge.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Call, Facts, InstructionKind as I, Value},
        vocabulary::{Compare, Type},
    },
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn dynamic_cast_value(
        &mut self,
        id: ast::ExprId,
        operand: ast::ExprId,
    ) -> Result<RValue, Error> {
        let cast = self
            .graph
            .lower_runtime_cast(id)?
            .ok_or(crate::sema::Error::Invariant(
                "runtime cast without semantic selection",
            ))?;
        let source = self.graph.lower_expression(id)?.ty;
        let object = if cast.reference {
            // The frozen glvalue path does not materialize a class prvalue
            // merely because a runtime cast requests its address.
            let place = self.designated_place(self.without_parentheses(operand))?;
            if matches!(place, Value::Slot(_) | Value::Symbol(_)) {
                self.emit(I::Addr(place), source)?.value
            } else {
                place
            }
        } else {
            self.operand(operand, Context::Cast)?.value
        };
        let slot = self.generated_slot("dyn_cast", Type::Ptr)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::Ptr, zero, slot))?;
        let scan = self.new_block("dyn_cast_scan")?;
        let end = self.new_block("dyn_cast_end")?;
        let null = self.emit(I::Compare(Compare::Eq, Type::Ptr, object, zero), source)?;
        self.effect(I::Branch(null.value, end, scan))?;
        self.terminated = true;
        self.place(scan)?;
        self.symbols
            .construction_table(self.builder.program(), cast.from)?;
        let from = self
            .symbols
            .typeinfo_symbol(self.builder.program(), cast.from)?;
        let from = self.emit(I::Addr(Value::Symbol(from)), source)?.value;
        self.symbols
            .construction_table(self.builder.program(), cast.to)?;
        let to = self
            .symbols
            .typeinfo_symbol(self.builder.program(), cast.to)?;
        let to = self.emit(I::Addr(Value::Symbol(to)), source)?.value;
        let hint = self.symbols.dynamic_cast_hint(cast.from, cast.to)?;
        let hint = self
            .builder
            .integer(u128::from_le_bytes(i128::from(hint).to_le_bytes()))?;
        let arguments = self.builder.arguments(&[object, from, to, hint])?;
        let runtime = self.symbols.cast_runtime(self.builder.program(), false)?;
        let found = self.emit(
            I::Call(Call {
                result: Type::Ptr,
                callee: Value::Symbol(runtime),
                arguments,
                signature: None,
                copy_elision: false,
            }),
            source,
        )?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            found.value,
            slot,
        ))?;
        if cast.reference {
            let fail = self.new_block("dyn_cast_fail")?;
            let success = self.new_block("dyn_cast_found")?;
            let null = self.emit(
                I::Compare(Compare::Eq, Type::Ptr, found.value, zero),
                source,
            )?;
            self.effect(I::Branch(null.value, fail, success))?;
            self.terminated = true;
            self.place(fail)?;
            self.bad_cast_return()?;
            self.place(success)?;
            self.jump(end)?;
            let unreachable = self.new_block("block")?;
            self.place(unreachable)?;
        }
        self.jump(end)?;
        self.place(end)?;
        self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)
    }

    fn bad_cast_return(&mut self) -> Result<(), Error> {
        self.rtti_failure_return(crate::lowir::vocabulary::Role::BadCast)
    }
    pub(super) fn rtti_failure_return(
        &mut self,
        role: crate::lowir::vocabulary::Role,
    ) -> Result<(), Error> {
        let runtime = self
            .symbols
            .runtime_function(self.builder.program(), role)?;
        let arguments = self.builder.arguments(&[])?;
        self.effect(I::Call(Call {
            result: Type::Void,
            callee: Value::Symbol(runtime),
            arguments,
            signature: None,
            copy_elision: false,
        }))?;
        let Some(result) = self.result else {
            self.effect(I::Return(Type::Void, None))?;
            self.terminated = true;
            return Ok(());
        };
        let ty = self.symbols.result_boundary(result)?.ty;
        self.fallback_return(result, ty)
    }
}
