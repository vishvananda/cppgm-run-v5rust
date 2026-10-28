// Translated from lower/dynamic_type_lowering.cpp::runtime_cast/most_derived_object. See NOTICE.
//! Runtime casts preserve nulls, vtable header offsets and frozen search control flow.
use super::{
    Error,
    conversions::Context,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{Call, Facts, InstructionKind as I, RuntimeInstruction as R, Value},
        vocabulary::{Compare, Type},
    },
    sema::lowering::{RuntimeCast, TypeId},
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
        let (from_class, to_class, reference) = match cast {
            RuntimeCast::VoidPointer { .. } => {
                let object = self.operand(operand, Context::Cast)?.value;
                return self.most_derived_object(object, source);
            }
            RuntimeCast::Class {
                from,
                to,
                reference,
            } => (from, to, reference),
        };
        let object = if reference {
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
        if !self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .host_objects()
        {
            crate::lower::unit_frontend::Context {
                graph: self.graph,
                tree: self.tree,
                cursor: self.cursor,
                symbols: self.symbols,
            }
            .construction_table(self.builder.program(), from_class)?;
        }
        let from = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .typeinfo_symbol(self.builder.program(), from_class)?;
        let from = self.emit(I::Addr(Value::Symbol(from)), source)?.value;
        if !self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .host_objects()
        {
            crate::lower::unit_frontend::Context {
                graph: self.graph,
                tree: self.tree,
                cursor: self.cursor,
                symbols: self.symbols,
            }
            .construction_table(self.builder.program(), to_class)?;
        }
        let to = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .typeinfo_symbol(self.builder.program(), to_class)?;
        let to = self.emit(I::Addr(Value::Symbol(to)), source)?.value;
        let hint = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .dynamic_cast_hint(from_class, to_class)?;
        let hint = self
            .builder
            .integer(u128::from_le_bytes(i128::from(hint).to_le_bytes()))?;
        let arguments = self.builder.arguments(&[object, from, to, hint])?;
        let runtime = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .cast_runtime(self.builder.program(), false)?;
        let found = self.emit(
            I::Call(Call {
                result: Type::Ptr,
                callee: Value::Symbol(runtime),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: Vec::new(),
            }),
            source,
        )?;
        self.effect(I::Store(
            Facts::memory(false, true),
            Type::Ptr,
            found.value,
            slot,
        ))?;
        if reference {
            let fail = self.new_block("dyn_cast_fail")?;
            let success = self.new_block("dyn_cast_found")?;
            let null = self.emit(
                I::Compare(Compare::Eq, Type::Ptr, found.value, zero),
                source,
            )?;
            self.effect(I::Branch(null.value, fail, success))?;
            self.terminated = true;
            self.place(fail)?;
            self.rtti_failure(crate::lowir::vocabulary::Role::BadCast)?;
            self.place(success)?;
            self.jump(end)?;
            let unreachable = self.new_block("block")?;
            self.place(unreachable)?;
        }
        self.jump(end)?;
        self.place(end)?;
        self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)
    }

    fn most_derived_object(&mut self, object: Value, source: TypeId) -> Result<RValue, Error> {
        let slot = self.generated_slot("dyn_cast", Type::Ptr)?;
        let zero = self.builder.integer(0)?;
        self.effect(I::Store(Facts::memory(false, true), Type::Ptr, zero, slot))?;
        let scan = self.new_block("dyn_cast_scan")?;
        let end = self.new_block("dyn_cast_end")?;
        let null = self.emit(I::Compare(Compare::Eq, Type::Ptr, object, zero), source)?;
        self.effect(I::Branch(null.value, end, scan))?;
        self.terminated = true;
        self.place(scan)?;
        let table = self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, object),
                source,
            )?
            .value;
        let header_offset = self
            .builder
            .integer(u128::from_le_bytes((-16_i128).to_le_bytes()))?;
        let header = self
            .emit(I::Index(Type::I8, None, table, header_offset), source)?
            .value;
        let offset = self
            .emit(
                I::Load(Facts::memory(false, true), Type::I64, header),
                source,
            )?
            .value;
        let top = self
            .emit(I::Index(Type::I8, None, object, offset), source)?
            .value;
        self.effect(I::Store(Facts::memory(false, true), Type::Ptr, top, slot))?;
        self.jump(end)?;

        self.place(end)?;
        self.emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)
    }

    pub(super) fn rtti_failure(
        &mut self,
        role: crate::lowir::vocabulary::Role,
    ) -> Result<(), Error> {
        // A failure-only region must not change the sibling success path.
        if !self.lifetime.region
            && !self.lifetime_chain_empty()
            && !self.static_initializer
            && self.handlers.explicit == 0
        {
            let target = if self.has_tracked_region() {
                self.tracked_target()?
            } else {
                self.unwind_entry()?
            };
            self.effect(I::Runtime(R::EhTry(target)))?;
            if self.has_tracked_region() && !self.lifetime.tracking.made {
                self.lifetime
                    .tracking
                    .markers
                    .push(self.builder.position()?);
            }
        }
        let runtime = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .runtime_function(self.builder.program(), role)?;
        let arguments = self.builder.arguments(&[])?;
        self.effect(I::Call(Call {
            result: Type::Void,
            callee: Value::Symbol(runtime),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: Vec::new(),
        }))?;
        // Terminate the failure edge through the ordinary source-segment rules.
        self.effect(I::Unreachable)?;
        self.terminated = true;
        Ok(())
    }
}
