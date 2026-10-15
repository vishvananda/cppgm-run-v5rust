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

#[derive(Clone, Copy)]
pub(super) struct ResultBoundary {
    pub ty: Type,
    pub indirect: bool,
    pub classes: Option<ObjectClasses>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn zero_return(&mut self, source: TypeId) -> Result<Option<Value>, Error> {
        match self.symbols.result_boundary(source)?.ty {
            Type::Void => Ok(None),
            ty @ Type::Object { bytes, align } => {
                let slot = self.generated_slot("retobj", ty)?;
                self.effect(I::ZeroInit(bytes, align, slot))?;
                Ok(Some(slot))
            }
            _ => Ok(Some(self.zero(source)?.value)),
        }
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
        let began = self.begin_full_expression(Some(id))?;
        let (object, value) = if let Some(object) = self.result_destination {
            (object, None)
        } else {
            let slot = if let Some(slot) = self.result_storage {
                slot
            } else {
                let slot = self.generated_slot("retobj", boundary.ty)?;
                self.result_storage = Some(slot);
                slot
            };
            (self.emit(I::Addr(slot), result)?.value, Some(slot))
        };
        let source = self.without_parentheses(id);
        if let ast::ExprKind::Id(name) = self.tree.expressions.get(source).kind
            && self.returned_local == Some(self.graph.lower_name(name)?)
        {
            return self.cleanup_return(boundary.ty, value, began);
        }
        let empty = matches!(self.graph.lower_type(result)?, TypeView::Class(class)
            if self.graph.lower_class_empty(class)? && self.graph.lower_class_bases(class).next().is_none());
        if empty
            && matches!(self.tree.expressions.get(source).kind,
                ast::ExprKind::FunctionalCast { arguments, braced: None, .. }
                if self.tree.expr_lists.get(arguments).is_empty())
        {
            self.materialize_empty_argument(source, result, object)?;
        } else {
            self.materialize_class(id, result, object)?;
        }
        self.cleanup_return(boundary.ty, value, began)
    }
}
