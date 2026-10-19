// Translated from lower/symbol_lowering.cpp::returned_directly and
// expression_lowering.cpp::copy_result, statement_lowering.cpp::class_return.
// See NOTICE.
//! Class results use either their bytes or a caller-owned destination.
use super::{Error, function::Function, types};
use crate::{
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::{ObjectClasses, Type},
    },
    sema::lowering::{TypeId, TypeView},
    syntax::ast,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum InitializationContext {
    Ordinary,
    Return,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum ResultRegion {
    #[default]
    Operands,
    Construction,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum LandingPlacement {
    #[default]
    Immediate,
    Deferred,
}
#[derive(Clone, Copy)]
pub(super) struct ResultBoundary {
    pub ty: Type,
    pub indirect: bool,
    pub classes: Option<ObjectClasses>,
}
pub(super) struct FallbackReturn {
    position: crate::lowir::builder::Position,
    ty: Type,
    end_guard: bool,
}
impl Function<'_, '_, '_, '_> {
    // exception_lowering.cpp::zero_return/patch_fallback_slots: a runtime
    // no-return continuation's object holder follows every ordinary slot.
    pub(super) fn fallback_return(&mut self, source: TypeId, ty: Type) -> Result<(), Error> {
        if matches!(ty, Type::Object { .. }) {
            if self.lifetime.full_expression {
                self.close_lifetime_region()?;
            }
            self.fallback_returns.push(FallbackReturn {
                position: self.builder.position()?,
                ty,
                end_guard: self.lifetime.function_guard.is_some(),
            });
        } else {
            let value = self.zero_return(source)?;
            self.effect(I::Return(ty, value))?;
        }
        self.terminated = true;
        Ok(())
    }
    pub(super) fn finish_fallback_returns(&mut self) -> Result<(), Error> {
        for fallback in std::mem::take(&mut self.fallback_returns) {
            let Type::Object { bytes, align } = fallback.ty else {
                return Err(crate::sema::Error::Invariant("scalar fallback object slot").into());
            };
            let slot = self.generated_slot("retobj", fallback.ty)?;
            let mut effects = vec![I::ZeroInit(bytes, align, slot)];
            if fallback.end_guard {
                effects.push(I::Runtime(crate::lowir::model::RuntimeInstruction::EhEnd));
            }
            effects.push(I::Return(fallback.ty, Some(slot)));
            self.builder
                .insert_effects(fallback.position, effects.into_iter())?;
        }
        Ok(())
    }
    pub(super) fn zero_return(&mut self, source: TypeId) -> Result<Option<Value>, Error> {
        match self.symbols.result_boundary(source)?.ty {
            Type::Void => Ok(None),
            ty @ Type::Object { bytes, align } => {
                let slot = self.class_result_storage(ty)?;
                self.effect(I::ZeroInit(bytes, align, slot))?;
                Ok(Some(slot))
            }
            _ => Ok(Some(self.zero(source)?.value)),
        }
    }
    fn class_result_storage(&mut self, ty: Type) -> Result<Value, Error> {
        if let Some(slot) = self.result_storage {
            return Ok(slot);
        }
        let slot = self.generated_slot("retobj", ty)?;
        self.result_storage = Some(slot);
        Ok(slot)
    }
    pub(super) fn find_returned_local(
        &mut self,
        body: ast::StmtId,
        result: TypeId,
    ) -> Result<(), Error> {
        let mut candidate = None;
        let mut single = true;
        self.scan_returned_local(body, result, &mut candidate, &mut single)?;
        self.returned_local = candidate.filter(|_| single);
        Ok(())
    }
    fn scan_returned_local(
        &self,
        id: ast::StmtId,
        result: TypeId,
        candidate: &mut Option<crate::sema::lowering::EntityId>,
        single: &mut bool,
    ) -> Result<(), Error> {
        use ast::StmtKind as S;
        self.check_stack()?;
        match self.tree.statements.get(id).kind {
            S::Compound(items) => {
                for &item in self.tree.stmt_lists.get(items) {
                    self.scan_returned_local(item, result, candidate, single)?;
                }
            }
            S::Ambiguous { .. } => self.scan_returned_local(
                self.graph.lower_statement(id)?,
                result,
                candidate,
                single,
            )?,
            S::Return(value) => {
                let named = value
                    .and_then(|value| {
                        if let ast::ExprKind::Id(name) = self
                            .tree
                            .expressions
                            .get(self.without_parentheses(value))
                            .kind
                        {
                            Some(self.graph.lower_name(name))
                        } else {
                            None
                        }
                    })
                    .transpose()?;
                let usable = named.is_some_and(|named| {
                    self.entity.is_some_and(|function| {
                        self.graph.lower_result_local(named, function, result)
                    })
                });
                *single &= usable && (candidate.is_none() || *candidate == named);
                *candidate = named;
            }
            S::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.scan_returned_local(then_branch, result, candidate, single)?;
                if let Some(branch) = else_branch {
                    self.scan_returned_local(branch, result, candidate, single)?;
                }
            }
            S::ConditionalLoop { body, .. }
            | S::Do { body, .. }
            | S::For { body, .. }
            | S::RangeFor { body, .. }
            | S::Case { body, .. }
            | S::Default(body)
            | S::Label { body, .. } => {
                self.scan_returned_local(body, result, candidate, single)?;
            }
            _ => (),
        }
        Ok(())
    }
    pub(super) fn finish_result_call(
        &mut self,
        call: I,
        source: TypeId,
        destination: Option<Value>,
        throws: bool,
    ) -> Result<super::function::RValue, Error> {
        let ty = call.result_type();
        let value = if ty == Type::Void {
            self.effect(call)?;
            super::function::RValue {
                value: self.builder.integer(0)?,
                ty,
                source,
                literal: false,
                bits: None,
            }
        } else {
            self.emit(call, source)?
        };
        let value = self.finish_lifetime_call(value, throws)?;
        if let Some(destination) = destination
            && matches!(self.graph.lower_type(source)?, TypeView::Class(_))
            && ty != Type::Void
        {
            self.copy_call_result(value, source, destination)?;
        }
        Ok(value)
    }
    pub(super) fn copy_call_result(
        &mut self,
        value: super::function::RValue,
        source: TypeId,
        object: Value,
    ) -> Result<(), Error> {
        let TypeView::Class(class) = self.graph.lower_type(source)? else {
            return Err(crate::sema::Error::Invariant("class result without class").into());
        };
        if !self.graph.lower_class_empty(class)? {
            let (bytes, align) = types::layout(self.graph, source)?;
            self.effect(I::CopyObject(bytes, align, value.value, object))?;
        }
        Ok(())
    }
    pub(super) fn class_return(&mut self, id: ast::ExprId, result: TypeId) -> Result<(), Error> {
        let boundary = self.symbols.result_boundary(result)?;
        let first = self.lifetime.cleanups.len();
        let began = self
            .begin_full_expression(Some(id), super::temporary_scan::ExpressionUse::ClassResult)?;
        let temporaries = self.lifetime.temporaries_made;
        let mut throwing_local = false;
        if self.result_destination.is_some() {
            for index in self.handler_floor()..self.lifetime.cleanups.len() {
                throwing_local |= self.destructor_throws(self.lifetime.cleanups[index].ty)?;
            }
        }
        let source = self.without_parentheses(id);
        let at_call = self.result_constructed_after_operands(source)?;
        let saved_here = std::mem::replace(
            &mut self.lifetime.result_region,
            if began && throwing_local && at_call {
                ResultRegion::Construction
            } else {
                ResultRegion::Operands
            },
        );
        let (object, value) = if let Some(object) = self.result_destination {
            (object, None)
        } else {
            let slot = self.class_result_storage(boundary.ty)?;
            (self.emit(I::Addr(slot), result)?.value, Some(slot))
        };
        let elided = self.result_destination.is_some()
            && matches!(self.tree.expressions.get(source).kind, ast::ExprKind::Id(name)
                if self.returned_local == Some(self.graph.lower_name(name)?));
        let empty = matches!(self.graph.lower_type(result)?, TypeView::Class(class)
            if self.graph.lower_class_empty(class)? && self.graph.lower_class_bases(class).next().is_none());
        let empty_value = empty
            && matches!(self.tree.expressions.get(source).kind,
                ast::ExprKind::FunctionalCast { arguments, braced: None, .. }
                if self.tree.expr_lists.get(arguments).is_empty());
        let saved = std::mem::replace(
            &mut self.empty_value_zeroing,
            if empty_value {
                super::construction::EmptyValueZeroing::Omit
            } else {
                super::construction::EmptyValueZeroing::Emit
            },
        );
        let returning = std::mem::replace(
            &mut self.initialization_context,
            InitializationContext::Return,
        );
        let initialized = if elided {
            Ok(())
        } else {
            self.initialize_class_result(id, result, object, self.graph.lower_class_copy(id)?, None)
        };
        self.empty_value_zeroing = saved;
        self.initialization_context = returning;
        if let Err(error) = initialized {
            self.lifetime.result_region = saved_here;
            return Err(error);
        }
        self.destroy_temporaries(first)?;
        let defer = throwing_local
            && self.lifetime.temporaries_made == temporaries
            && !matches!(
                self.tree.expressions.get(source).kind,
                ast::ExprKind::Conditional { .. } | ast::ExprKind::Call { .. }
            );
        let saved_defer = std::mem::replace(
            &mut self.lifetime.landing_placement,
            if defer {
                LandingPlacement::Deferred
            } else {
                LandingPlacement::Immediate
            },
        );
        let ended = self.end_lifetime_expression(began);
        self.lifetime.landing_placement = saved_defer;
        self.lifetime.result_region = saved_here;
        ended?;
        // CWG 2176: once its own full-expression finishes, a completed
        // result is destroyed first if a local destructor throws on return.
        let returned = self.lexical.returned;
        if !elided && self.symbols.storage_destructor(result)?.is_some() {
            let mut cleanup = super::lifetime::Cleanup::temporary(result, object);
            if let Some(slot) = value {
                cleanup.place = super::lifetime::CleanupPlace::Storage(slot);
            }
            self.lexical.returned = Some(cleanup);
        }
        let finished = self.cleanup_return(boundary.ty, value, false);
        self.lexical.returned = returned;
        finished?;
        // The result's pending landings precede lexical landings unless
        // an enclosing return already deferred placement.
        if self.terminated && saved_defer == LandingPlacement::Immediate {
            self.place_result_cleanup_chains()?;
        }
        Ok(())
    }
    fn result_constructed_after_operands(&mut self, source: ast::ExprId) -> Result<bool, Error> {
        match self.tree.expressions.get(source).kind {
            ast::ExprKind::Id(_) => Ok(true),
            ast::ExprKind::FunctionalCast {
                arguments,
                braced: None,
                ..
            } => {
                for &argument in self.tree.expr_lists.get(arguments) {
                    if self.expression_temporaries(argument)?.made {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    fn initialize_class_result(
        &mut self,
        id: ast::ExprId,
        result: TypeId,
        object: Value,
        action: Option<crate::sema::lowering::CopyAction>,
        projection: Option<u64>,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let root = self.without_parentheses(id);
        let facts = self.graph.lower_expression(root)?;
        if let ast::ExprKind::Conditional {
            condition,
            then_expr,
            else_expr,
        } = self.tree.expressions.get(root).kind
            && (facts.is_place
                || matches!((self.graph.lower_type(facts.ty)?, self.graph.lower_type(result)?), (TypeView::Class(a), TypeView::Class(b)) if a == b))
        {
            let left = self.new_block("condobj_then")?;
            let right = self.new_block("condobj_else")?;
            let end = self.new_block("condobj_end")?;
            let condition = self.truth(condition)?;
            self.effect(I::Branch(condition.value, left, right))?;
            // C++ initialize_result passes the whole glvalue's selected
            // argument binding to either branch. Its base projection belongs
            // to that binding, not to a new branch-specific conversion.
            let projection = if facts.is_place {
                projection.or(self.graph.lower_base_conversion_offset(root)?)
            } else {
                None
            };
            for (block, operand) in [(left, then_expr), (right, else_expr)] {
                self.place(block)?;
                let first = self.lifetime.cleanups.len();
                let own = if facts.is_place {
                    action
                } else {
                    self.graph.lower_class_copy(operand)?
                };
                let throwing = matches!(
                    self.tree
                        .expressions
                        .get(self.without_parentheses(operand))
                        .kind,
                    ast::ExprKind::Throw(_)
                );
                if throwing {
                    self.discard(operand)?;
                } else {
                    self.initialize_class_result(operand, result, object, own, projection)?;
                }
                if !self.lifetime.tracking.active {
                    self.destroy_temporaries(first)?;
                }
                self.jump(end)?;
            }
            self.place(end)
        } else if let (Some(offset), Some(action)) = (projection, action) {
            self.materialize_projected_copy(id, result, object, action, offset)
        } else if self.materialize_copy(id, result, object, action)? {
            Ok(())
        } else {
            self.materialize_class(id, result, object)
        }
    }
    fn materialize_projected_copy(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
        action: crate::sema::lowering::CopyAction,
        offset: u64,
    ) -> Result<(), Error> {
        use crate::sema::lowering::CopyAction;
        let source = self.address(id)?;
        let source = self.base_projection(source, ty, offset)?;
        match action {
            CopyAction::Constructor { constructor, .. } => self.call_copy_constructor(
                constructor,
                super::construction::Variant::Complete,
                object,
                source,
            ),
            CopyAction::Storage { .. } => {
                let TypeView::Class(class) = self.graph.lower_type(ty)? else {
                    return Err(
                        crate::sema::Error::Invariant("projected copy without class").into(),
                    );
                };
                if !self.graph.lower_class_empty(class)? {
                    let (bytes, align) = types::layout(self.graph, ty)?;
                    let bytes = bytes.min(self.graph.lower_nonvirtual_bytes(class)?);
                    self.effect(I::CopyObject(bytes, align, source, object))?;
                }
                Ok(())
            }
        }
    }
}
