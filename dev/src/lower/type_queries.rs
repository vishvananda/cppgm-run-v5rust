// Translated from lower/dynamic_type_lowering.cpp::typeid_place/type_identity. See NOTICE.
//! Static queries designate canonical RTTI; dynamic queries load the primary table header.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lex::pp_lexer::Punctuator,
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::{Compare, Role, Type},
    },
    syntax::ast::{self, ExprKind},
};
impl Function<'_, '_, '_, '_> {
    pub(super) fn typeid_place(&mut self, id: ast::ExprId) -> Result<Value, Error> {
        let query = self
            .graph
            .lower_type_query(id)
            .ok_or(crate::sema::Error::Invariant("missing type query"))?;
        if !query.dynamic {
            return self
                .symbols
                .type_information(self.builder.program(), query.target)
                .map(Value::Symbol);
        }
        let ExprKind::TypeTrait {
            operand: Some(operand),
            ..
        } = self.tree.expressions.get(self.without_parentheses(id)).kind
        else {
            return Err(crate::sema::Error::Invariant("dynamic query without operand").into());
        };
        let inner = self.without_parentheses(operand);
        let through_pointer = matches!(self.tree.expressions.get(inner).kind,
            ExprKind::Unary { op, .. } if op.token.as_punctuator() == Some(Punctuator::Multiply))
            && !self.graph.lower_operator_call(inner)?;
        let object = self.address(operand)?;
        if let crate::sema::lowering::TypeView::Class(class) =
            self.graph.lower_type(query.target)?
        {
            self.symbols
                .construction_table(self.builder.program(), class)?;
        }
        let source = self.graph.lower_expression(id)?.ty;
        if through_pointer {
            let fail = self.new_block("typeid_fail")?;
            let scan = self.new_block("typeid_scan")?;
            let zero = self.builder.integer(0)?;
            let null = self.emit(I::Compare(Compare::Eq, Type::Ptr, object, zero), source)?;
            self.effect(I::Branch(null.value, fail, scan))?;
            self.terminated = true;
            self.place(fail)?;
            self.rtti_failure_return(Role::BadTypeId)?;
            self.place(scan)?;
        }
        let table = self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, object),
                source,
            )?
            .value;
        let offset = self
            .builder
            .integer(u128::from_le_bytes((-8_i128).to_le_bytes()))?;
        let slot = self
            .emit(I::Index(Type::I8, None, table, offset), source)?
            .value;
        Ok(self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, slot), source)?
            .value)
    }
    pub(super) fn type_identity_value(&mut self, id: ast::ExprId) -> Result<RValue, Error> {
        let id = self.without_parentheses(id);
        let ExprKind::Binary {
            op, left, right, ..
        } = self.tree.expressions.get(id).kind
        else {
            return Err(
                crate::sema::Error::Invariant("type identity without binary syntax").into(),
            );
        };
        let left = self.address(left)?;
        let right = self.address(right)?;
        let source = self.graph.lower_expression(id)?.ty;
        self.emit(
            I::Compare(
                if op.token.as_punctuator() == Some(Punctuator::Equal) {
                    Compare::Eq
                } else {
                    Compare::Ne
                },
                Type::Ptr,
                left,
                right,
            ),
            source,
        )
    }
}
