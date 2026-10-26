// Translated from sema/constant_interpreter.cpp and lower/constant_lowering.cpp.
// See NOTICE.
//! Static images consume the shared interpreter's completed cells, including table placements.
use super::{
    Error,
    analyzer::Analyzer,
    constructor_initialization::ConstructorArguments,
    expression_facts::ClassInitialization,
    graph::{EntityId, ScopeId},
    static_arrays::{Item, append_zero},
    static_initialization::Value,
    types::{Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};
pub(super) struct EvaluatedImage {
    pub image: Option<Vec<Item>>,
    pub demands: Vec<Item>,
}
impl Analyzer<'_> {
    pub(super) fn evaluated_polymorphic_object(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
    ) -> Result<Option<EvaluatedImage>, Error> {
        self.record_aggregate_constant(entity, initializer, scope, false)?;
        Ok(self
            .completed_variable_image(entity)?
            .map(|image| EvaluatedImage {
                image: Some(image),
                demands: Vec::new(),
            }))
    }
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
        if !self.static_zero_placements(target, 0, 0, &mut placements)? {
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
        depth: u32,
        out: &mut Vec<(u64, u64, Item)>,
    ) -> Result<bool, Error> {
        self.check_stack()?;
        // This projects an evaluated zero value just like constant_placements.
        // Keep its data-representation bound even when no scalar is nonzero.
        if depth > 256 {
            return Ok(false);
        }
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
                    if !self.static_zero_placements(ty, at, depth + 1, out)? {
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
                        for (offset, size) in super::evaluated_data::bit_units(
                            storage,
                            self.graph.bit_fields.width(field),
                        )? {
                            let at = origin
                                .checked_add(offset)
                                .ok_or(Error::Limit("static zero bit unit"))?;
                            out.push((at, u64::from(size), Item::BitUnit(size, 0)));
                        }
                    } else {
                        let offset = self
                            .graph
                            .entity(member)
                            .member_offset
                            .ok_or(Error::Invariant("static zero member without offset"))?;
                        let at = origin
                            .checked_add(offset)
                            .ok_or(Error::Limit("static zero member span"))?;
                        if !self.static_zero_placements(ty, at, depth + 1, out)? {
                            return Ok(false);
                        }
                    }
                    if union {
                        break;
                    }
                }
            }
            TypeKind::Array { element, bound, .. } => {
                let size = self.type_size(element, SourceOffset(0))?.0;
                for index in 0..bound {
                    let at = index
                        .checked_mul(size)
                        .and_then(|offset| origin.checked_add(offset))
                        .ok_or(Error::Limit("static zero array span"))?;
                    if !self.static_zero_placements(element, at, depth + 1, out)? {
                        return Ok(false);
                    }
                }
            }
            TypeKind::MemberPointer { .. } => out.push((
                origin,
                self.type_size(target, SourceOffset(0))?.0,
                Item::Scalar(
                    target,
                    Value::MemberPointer(super::symbolic_members::MemberPointer::null(
                        self.bare_type(target)?,
                    )),
                ),
            )),
            TypeKind::Pointer(_) | TypeKind::Scalar(Scalar::Nullptr) => {}
            TypeKind::Scalar(Scalar::Basic(ty)) if ty.floating_rank() != 0 => {
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
