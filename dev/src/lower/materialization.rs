// Translated from lower/object_lowering.cpp::materialize_prvalue/temporary_object.
// See NOTICE.
//! Original class expressions initialize their destination without an intermediate copy.
use super::{Error, function::Function, types};
use crate::{
    lowir::model::{InstructionKind as I, Value},
    sema::lowering::{ClassInitialization, EntityId, TypeId, TypeView},
    syntax::ast::{self, ExprKind},
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn materialize_member_expression(
        &mut self,
        member: EntityId,
        pointer: TypeId,
        initializer: ast::Initializer,
        union: bool,
    ) -> Result<bool, Error> {
        let ty = self.graph.lower_entity_type(member)?;
        if !matches!(self.graph.lower_type(ty)?, TypeView::Class(_)) {
            return Ok(false);
        }
        let expr = match initializer.kind {
            ast::InitializerKind::Assign(expr)
                if !matches!(
                    self.tree.expressions.get(expr).kind,
                    ExprKind::Braced { .. }
                ) =>
            {
                expr
            }
            ast::InitializerKind::Parenthesized(args)
                if self.tree.expr_lists.get(args).len() == 1 =>
            {
                self.tree.expr_lists.get(args)[0]
            }
            _ => return Ok(false),
        };
        if !matches!(
            self.graph
                .lower_type(self.graph.lower_expression(expr)?.ty)?,
            TypeView::Class(_)
        ) {
            return Ok(false);
        }
        let object = self.object_pointer(pointer)?.value;
        let offset = self
            .builder
            .integer(u128::from(self.graph.lower_member_offset(member)?))?;
        let address = self
            .emit(
                I::Index(
                    crate::lowir::vocabulary::Type::I8,
                    Some(crate::lowir::vocabulary::Projection::Field),
                    object,
                    offset,
                ),
                ty,
            )?
            .value;
        if union {
            self.untyped_places.insert(address);
        }
        self.materialize_class(expr, ty, address)?;
        Ok(true)
    }
    pub(super) fn class_temporary(
        &mut self,
        id: ast::ExprId,
        stem: &'static str,
    ) -> Result<Value, Error> {
        let ty = self.graph.lower_expression(id)?.ty;
        self.symbols.require_trivial_storage(ty)?;
        let slot = self.generated_slot(stem, types::storage(self.graph, ty)?)?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        self.materialize_class(id, ty, object)?;
        Ok(object)
    }
    pub(super) fn materialize_class(
        &mut self,
        id: ast::ExprId,
        ty: TypeId,
        object: Value,
    ) -> Result<(), Error> {
        self.check_stack()?;
        self.symbols.require_trivial_storage(ty)?;
        let id = self.without_parentheses(id);
        match self.graph.lower_prvalue_initialization(id)? {
            Some(ClassInitialization::Constructed(arguments)) => {
                let constructor = self
                    .graph
                    .lower_list_constructor(id)?
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "class prvalue without selected constructor",
                    )))?;
                self.construct_at(constructor, ty, object, Some(arguments))
            }
            Some(ClassInitialization::Aggregate(arguments)) => {
                self.aggregate_prvalue(ty, object, self.tree.expr_lists.get(arguments))
            }
            Some(ClassInitialization::Elided(source)) => self.materialize_class(source, ty, object),
            None => Err(Error::Missing("class call and conditional materialization")),
        }
    }
}
