// Translated from sema/constant_construction.cpp and lower/constant_lowering.cpp.
// See NOTICE.
//! Evaluated value zeros differ from omitted aggregate members in static data spelling.
use super::{
    Error,
    analyzer::Analyzer,
    constructor_initialization::ConstructorArguments,
    expression_facts::ClassInitialization,
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast, types::FundamentalType as F};

impl Analyzer<'_> {
    pub(super) fn static_zero_prvalue(
        &mut self,
        target: TypeId,
        expr: ast::ExprId,
        out: &mut Vec<Item>,
    ) -> Result<Option<bool>, Error> {
        self.check_stack()?;
        let root = self.graph.expression_facts.root(expr);
        let facts = self.graph.expression_facts.get(root)?;
        if let Some(constructor) = facts.constructor {
            if matches!(
                facts.class_initialization,
                Some(ClassInitialization::Constructed(
                    ConstructorArguments::Value
                ))
            ) && !self.graph.constructors.get(constructor).nontrivial()
            {
                return self.static_value_zero(target, out).map(Some);
            }
            return Ok(None);
        }
        if let Some(ClassInitialization::FromExpression(source)) = facts.class_initialization {
            return self.static_zero_prvalue(target, source, out);
        }
        Ok(None)
    }
    fn static_value_zero(&mut self, target: TypeId, out: &mut Vec<Item>) -> Result<bool, Error> {
        self.check_stack()?;
        match self.graph.types.get(target).kind {
            TypeKind::Class(class) => {
                if !self.graph.aggregate_class(class) {
                    return Ok(false);
                }
                let (size, _) = self.type_size(target, SourceOffset(0))?;
                let mut position = 0;
                let union = self.graph.entity(class).key == crate::lex::vocabulary::Keyword::Union;
                for index in 0..self.graph.entity(class).members.len() {
                    let member = self.graph.entity(class).members[index];
                    if self.graph.unnamed_bit_field(member) {
                        continue;
                    }
                    let ty = self.graph.lower_entity_type(member)?;
                    if let Some(field) = self.graph.entity(member).bit_field {
                        let storage = self.graph.bit_fields.storage(field)?;
                        if storage.shared || storage.split != 0 || storage.bytes > 8 {
                            return Ok(false);
                        }
                        if storage.offset < position {
                            continue;
                        }
                        append_zero(out, storage.offset - position)?;
                        out.push(Item::BitUnit(storage.bytes, 0));
                        position = storage
                            .offset
                            .checked_add(u64::from(storage.bytes))
                            .ok_or(Error::Limit("static zero bit unit"))?;
                    } else {
                        let offset = self
                            .graph
                            .entity(member)
                            .member_offset
                            .ok_or(Error::Invariant("static zero member without offset"))?;
                        append_zero(
                            out,
                            offset
                                .checked_sub(position)
                                .ok_or(Error::Invariant("overlapping static zero members"))?,
                        )?;
                        if !self.static_value_zero(ty, out)? {
                            return Ok(false);
                        }
                        position = offset
                            .checked_add(self.type_size(ty, SourceOffset(0))?.0)
                            .ok_or(Error::Limit("static zero member span"))?;
                    }
                    if union {
                        break;
                    }
                }
                append_zero(
                    out,
                    size.checked_sub(position)
                        .ok_or(Error::Invariant("static zero class size"))?,
                )?;
            }
            TypeKind::Array { element, bound } => {
                for _ in 0..bound {
                    if !self.static_value_zero(element, out)? {
                        return Ok(false);
                    }
                }
            }
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr) => append_zero(out, 8)?,
            TypeKind::Scalar(Scalar::Basic(ty @ (F::Float | F::Double | F::LongDouble))) => {
                out.push(Item::Scalar(target, Value::Floating(ty, 0)));
            }
            TypeKind::Scalar(_) | TypeKind::Enum(_) => {
                out.push(Item::Scalar(target, Value::Integer(0)));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
