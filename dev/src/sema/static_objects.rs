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
    pub(super) fn static_value_zero(
        &mut self,
        target: TypeId,
        out: &mut Vec<Item>,
    ) -> Result<bool, Error> {
        // Evaluated objects place base/member values at their actual offsets.
        // Base tail padding and empty bases do not occupy a second span.
        let size = self.type_size(target, SourceOffset(0))?.0;
        let mut placements = Vec::new();
        if !self.static_zero_placements(target, 0, &mut placements)? {
            return Ok(false);
        }
        placements.sort_by_key(|&(offset, _, _)| offset);
        let mut position = 0;
        for (offset, size, item) in placements {
            if offset < position {
                if matches!(item, Item::BitUnit(..)) {
                    continue;
                }
                return Ok(false);
            }
            append_zero(out, offset - position)?;
            out.push(item);
            position = offset
                .checked_add(size)
                .ok_or(Error::Limit("static zero span"))?;
        }
        append_zero(
            out,
            size.checked_sub(position)
                .ok_or(Error::Invariant("static zero object span"))?,
        )?;
        Ok(true)
    }
    fn static_zero_placements(
        &mut self,
        target: TypeId,
        origin: u64,
        out: &mut Vec<(u64, u64, Item)>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        match self.graph.types.get(target).kind {
            TypeKind::Class(class) => {
                for index in 0..self.graph.entity(class).bases.len() {
                    let base = &self.graph.entity(class).bases[index];
                    let offset = base
                        .offset
                        .ok_or(Error::Invariant("static zero base without offset"))?;
                    let ty = self.graph.lower_entity_type(base.class)?;
                    let at = origin
                        .checked_add(offset)
                        .ok_or(Error::Limit("static zero base span"))?;
                    if !self.static_zero_placements(ty, at, out)? {
                        return Ok(false);
                    }
                }
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
                        let at = origin
                            .checked_add(storage.offset)
                            .ok_or(Error::Limit("static zero bit unit"))?;
                        out.push((
                            at,
                            u64::from(storage.bytes),
                            Item::BitUnit(storage.bytes, 0),
                        ));
                    } else {
                        let offset = self
                            .graph
                            .entity(member)
                            .member_offset
                            .ok_or(Error::Invariant("static zero member without offset"))?;
                        let at = origin
                            .checked_add(offset)
                            .ok_or(Error::Limit("static zero member span"))?;
                        if !self.static_zero_placements(ty, at, out)? {
                            return Ok(false);
                        }
                    }
                    if union {
                        break;
                    }
                }
            }
            TypeKind::Array { element, bound } => {
                let size = self.type_size(element, SourceOffset(0))?.0;
                for index in 0..bound {
                    let at = index
                        .checked_mul(size)
                        .and_then(|offset| origin.checked_add(offset))
                        .ok_or(Error::Limit("static zero array span"))?;
                    if !self.static_zero_placements(element, at, out)? {
                        return Ok(false);
                    }
                }
            }
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr) => {}
            TypeKind::Scalar(Scalar::Basic(ty @ (F::Float | F::Double | F::LongDouble))) => {
                out.push((
                    origin,
                    self.type_size(target, SourceOffset(0))?.0,
                    Item::Scalar(target, Value::Floating(ty, 0)),
                ));
            }
            TypeKind::Scalar(_) | TypeKind::Enum(_) => {
                out.push((
                    origin,
                    self.type_size(target, SourceOffset(0))?.0,
                    Item::Scalar(target, Value::Integer(0)),
                ));
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
