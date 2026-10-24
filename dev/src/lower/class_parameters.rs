// Translated from lower/symbol_lowering.cpp::course_class_quarters/course_classes.
// See NOTICE.
//! Course object parameter classes are memoized by canonical class per unit.
use super::{Error, types};
use crate::{
    lowir::vocabulary::{EightbyteClass as Eightbyte, ObjectClasses, Type},
    sema::{
        Graph,
        lowering::{EntityId, TypeId, TypeView},
    },
    support::stack::StackBudget,
};
use std::collections::HashMap;

const MEMORY: u16 = 0x100;
const UNMODELED: u16 = 0x200;
#[derive(Default)]
pub(super) struct Classes {
    classes: HashMap<EntityId, u16>,
}
impl Classes {
    pub(super) fn result_memory(
        &mut self,
        graph: &Graph,
        class: EntityId,
        budget: StackBudget,
    ) -> Result<bool, Error> {
        let marker = 0u8;
        Ok(
            self.class_quarters(graph, class, budget, std::ptr::from_ref(&marker).addr())? & MEMORY
                != 0,
        )
    }
    pub(super) fn parameter(
        &mut self,
        graph: &Graph,
        ty: TypeId,
        budget: StackBudget,
    ) -> Result<Option<ObjectClasses>, Error> {
        let TypeView::Class(class) = graph.lower_type(ty)? else {
            return Ok(None);
        };
        let (size, _) = types::layout(graph, ty)?;
        if size == 0 || size > 16 {
            return Ok(None);
        }
        let marker = 0u8;
        let quarters =
            self.class_quarters(graph, class, budget, std::ptr::from_ref(&marker).addr())?;
        if quarters & (MEMORY | UNMODELED) != 0 {
            return Ok(Some(ObjectClasses::Memory));
        }
        let first = eightbyte(quarters);
        let second = (size > 8).then(|| eightbyte(quarters >> 4));
        if matches!(first, Eightbyte::Integer)
            && second.is_none_or(|second| matches!(second, Eightbyte::Integer))
        {
            return Ok(None);
        }
        Ok(Some(ObjectClasses::Direct(first, second)))
    }
    fn class_quarters(
        &mut self,
        graph: &Graph,
        class: EntityId,
        budget: StackBudget,
        base: usize,
    ) -> Result<u16, Error> {
        let marker = 0u8;
        if budget.exceeded(base, std::ptr::from_ref(&marker).addr()) {
            return Err(crate::sema::Error::Limit("constructs nested too deeply").into());
        }
        if let Some(&known) = self.classes.get(&class) {
            return Ok(known);
        }
        let mut quarters = 0;
        if graph.lower_class_layout(class)?.0 > 16 {
            quarters |= MEMORY;
        }
        for edge in graph.lower_destructor_bases(class) {
            if quarters & MEMORY != 0 {
                break;
            }
            let (class, offset) = edge?;
            self.member_quarters(
                graph,
                graph.lower_entity_type(class)?,
                offset,
                &mut quarters,
                budget,
                base,
            )?;
        }
        for &member in graph.lower_class_member_entities(class) {
            if quarters & MEMORY != 0 {
                break;
            }
            let ty = graph.lower_entity_type(member)?;
            if let Some(field) = graph.lower_bit_field(member)? {
                let (size, _) = types::layout(graph, ty)?;
                if field.width != 0 && size != 0 {
                    let first = field.bit / (8 * size) * size;
                    merge(&mut quarters, first, first.wrapping_add(size), 1);
                }
            } else {
                self.member_quarters(
                    graph,
                    ty,
                    graph.lower_member_offset(member)?,
                    &mut quarters,
                    budget,
                    base,
                )?;
            }
        }
        if quarters & MEMORY != 0 {
            quarters = MEMORY;
        } else if quarters & UNMODELED != 0 {
            quarters = UNMODELED;
        }
        self.classes.insert(class, quarters);
        Ok(quarters)
    }
    fn member_quarters(
        &mut self,
        graph: &Graph,
        ty: TypeId,
        offset: u64,
        quarters: &mut u16,
        budget: StackBudget,
        base: usize,
    ) -> Result<(), Error> {
        let marker = 0u8;
        if budget.exceeded(base, std::ptr::from_ref(&marker).addr()) {
            return Err(crate::sema::Error::Limit("constructs nested too deeply").into());
        }
        let (size, align) = types::layout(graph, ty)?;
        if size == 0 {
            return Ok(());
        }
        if offset.wrapping_add(size) > 16 || !offset.is_multiple_of(1u64 << align) {
            *quarters |= MEMORY;
            return Ok(());
        }
        match graph.lower_type(ty)? {
            TypeView::Complex(element) => {
                let (size, _) = types::layout(graph, element)?;
                for at in [offset, offset.wrapping_add(size)] {
                    self.member_quarters(graph, element, at, quarters, budget, base)?;
                }
            }
            TypeView::Array { element, bound } => {
                let (size, _) = types::layout(graph, element)?;
                if size != 0 {
                    for index in 0..bound {
                        self.member_quarters(
                            graph,
                            element,
                            offset.wrapping_add(index.wrapping_mul(size)),
                            quarters,
                            budget,
                            base,
                        )?;
                    }
                }
            }
            TypeView::Class(class) => {
                let inner = self.class_quarters(graph, class, budget, base)?;
                *quarters |= inner & (MEMORY | UNMODELED);
                for quarter in 0..4u32 {
                    let class = (inner >> (quarter * 2)) & 3;
                    if class != 0 {
                        let first = offset.wrapping_add(u64::from(quarter) * 4);
                        merge(quarters, first, first.wrapping_add(4), class);
                    }
                }
            }
            _ => {
                if let Some(kind) = super::soft_float::kind(graph, ty)? {
                    if kind == super::soft_float::Kind::Quad {
                        *quarters |= UNMODELED;
                    } else {
                        merge(quarters, offset, offset.wrapping_add(size), 2);
                    }
                    return Ok(());
                }
                let class = match types::storage(graph, ty)? {
                    Type::F32 | Type::F64 => 2,
                    Type::F80 => {
                        *quarters |= UNMODELED;
                        return Ok(());
                    }
                    _ => 1,
                };
                merge(quarters, offset, offset.wrapping_add(size), class);
            }
        }
        Ok(())
    }
}
fn merge(quarters: &mut u16, first: u64, last: u64, class: u16) {
    for quarter in 0..4u32 {
        let at = u64::from(quarter);
        if at < first / 4 || at * 4 >= last {
            continue;
        }
        let shift = quarter * 2;
        let current = (*quarters >> shift) & 3;
        let merged = if current == 1 || class == 1 {
            1
        } else if current == 0 {
            class
        } else {
            current
        };
        *quarters = (*quarters & !(3 << shift)) | merged << shift;
    }
}
fn eightbyte(quarters: u16) -> Eightbyte {
    let low = quarters & 3;
    let high = (quarters >> 2) & 3;
    if low == 1 || high == 1 {
        Eightbyte::Integer
    } else if low == 2 || high == 2 {
        Eightbyte::Sse
    } else {
        Eightbyte::None
    }
}
