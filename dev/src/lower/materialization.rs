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
    pub(super) fn named_elided_list(
        &self,
        initializer: Option<ast::Initializer>,
    ) -> Result<Option<ast::ExprId>, Error> {
        let Some(ast::Initializer {
            kind: ast::InitializerKind::Assign(expr),
            ..
        }) = initializer
        else {
            return Ok(None);
        };
        let expr = self.without_parentheses(expr);
        let ExprKind::FunctionalCast {
            braced: Some(list), ..
        } = self.tree.expressions.get(expr).kind
        else {
            return Ok(None);
        };
        let ExprKind::Braced { elements, .. } = self.tree.expressions.get(list).kind else {
            return Ok(None);
        };
        if !self.tree.expr_lists.get(elements).is_empty()
            && matches!(
                self.graph.lower_prvalue_initialization(expr)?,
                Some(ClassInitialization::Aggregate(_))
            )
        {
            return Ok(Some(list));
        }
        Ok(None)
    }
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
        // A constructor member needs its own partial-construction cleanup;
        // completed named storage owns its destruction at the declaration site.
        self.symbols.require_trivial_storage(ty)?;
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
    pub(super) fn class_argument(&mut self, id: ast::ExprId, ty: TypeId) -> Result<Value, Error> {
        let TypeView::Class(class) = self.graph.lower_type(ty)? else {
            return Err(crate::sema::Error::Invariant("class argument without class").into());
        };
        if !self.graph.lower_direct_class_parameter(class)? {
            return Err(Error::Missing("indirect class parameter ownership"));
        }
        self.symbols.require_trivial_storage(ty)?;
        let slot = self.generated_slot("argobj", types::storage(self.graph, ty)?)?;
        let object = self.emit(I::Addr(slot), ty)?.value;
        let source = self.without_parentheses(id);
        let functional = matches!(
            self.tree.expressions.get(source).kind,
            ExprKind::FunctionalCast { .. }
        ) || self.graph.lower_functional_call(source)?;
        if self.graph.lower_class_empty(class)? && functional {
            self.materialize_empty_argument(source, ty, object)?;
        } else {
            self.materialize_class(id, ty, object)?;
        }
        Ok(slot)
    }
    fn materialize_empty_argument(
        &mut self,
        source: ast::ExprId,
        ty: TypeId,
        object: Value,
    ) -> Result<(), Error> {
        self.check_stack()?;
        match self.graph.lower_prvalue_initialization(source)? {
            Some(ClassInitialization::Constructed(
                crate::sema::lowering::ConstructorArguments::Value,
            )) => {
                let constructor = self.graph.lower_list_constructor(source)?.ok_or(
                    crate::sema::Error::Invariant("empty argument without constructor"),
                )?;
                if !self.graph.lower_constructor_trivial(constructor) {
                    self.call_constructor(
                        constructor,
                        super::construction::Variant::Complete,
                        object,
                        None,
                    )?;
                }
                Ok(())
            }
            Some(ClassInitialization::Aggregate(values)) if values.is_empty() => Ok(()),
            Some(ClassInitialization::Elided(inner)) => {
                self.materialize_empty_argument(inner, ty, object)
            }
            _ => self.materialize_class(source, ty, object),
        }
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
        self.symbols.storage_destructor(ty)?;
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
