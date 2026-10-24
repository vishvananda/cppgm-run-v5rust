// Translated from lower/symbol_lowering.cpp::classify_eightbytes/object_classes.
// See NOTICE.
//! Host psABI classification, independent of the course's four-byte model.
use super::{Error, types};
use crate::{
    lowir::vocabulary::{EightbyteClass, ObjectClasses, Type},
    sema::{
        Graph,
        lowering::{EntityId, TypeId, TypeView},
    },
    support::stack::StackBudget,
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    None,
    Integer,
    Sse,
    SseUp,
    X87,
    X87Upper,
    Memory,
}
impl Class {
    fn merge(self, other: Self) -> Self {
        if self == other || other == Self::None {
            self
        } else if self == Self::None {
            other
        } else if self == Self::Memory || other == Self::Memory {
            Self::Memory
        } else if self == Self::Integer || other == Self::Integer {
            Self::Integer
        } else if matches!(self, Self::X87 | Self::X87Upper)
            || matches!(other, Self::X87 | Self::X87Upper)
        {
            Self::Memory
        } else {
            Self::Sse
        }
    }
    fn direct(self) -> Option<EightbyteClass> {
        match self {
            Self::None => Some(EightbyteClass::None),
            Self::Integer => Some(EightbyteClass::Integer),
            Self::Sse => Some(EightbyteClass::Sse),
            Self::SseUp => Some(EightbyteClass::SseUp),
            _ => None,
        }
    }
}

#[derive(Default)]
pub(super) struct Classes {
    // Completed canonical layouts are immutable for this lowering unit.
    classes: HashMap<EntityId, ObjectClasses>,
}
impl Classes {
    pub(super) fn classify(
        &mut self,
        graph: &Graph,
        ty: TypeId,
        budget: StackBudget,
    ) -> Result<Option<ObjectClasses>, Error> {
        if let Some((_, element)) = graph.lower_member_pointer_type(ty)
            && matches!(graph.lower_type(element)?, TypeView::Function { .. })
        {
            return Ok(Some(ObjectClasses::Direct(
                EightbyteClass::Integer,
                Some(EightbyteClass::Integer),
            )));
        }
        if let Some(kind) = super::soft_float::kind(graph, ty)? {
            return Ok(Some(super::soft_float::classes(kind)));
        }
        if let TypeView::Complex(part) = graph.lower_type(ty)? {
            if types::storage(graph, part)? == Type::F80 {
                return Ok(Some(ObjectClasses::ComplexX87));
            }
            let (size, _) = types::layout(graph, ty)?;
            if size > 16 {
                return Ok(Some(ObjectClasses::Memory));
            }
            let mut classes = [Class::None; 2];
            let marker = 0u8;
            classify_fields(
                graph,
                ty,
                0,
                &mut classes,
                budget,
                std::ptr::from_ref(&marker).addr(),
            )?;
            return Ok(Some(
                if let (Some(a), Some(b)) = (classes[0].direct(), classes[1].direct()) {
                    ObjectClasses::Direct(a, (size > 8).then_some(b))
                } else {
                    ObjectClasses::Memory
                },
            ));
        }
        let TypeView::Class(class) = graph.lower_type(ty)? else {
            return Ok(None);
        };
        if let Some(&known) = self.classes.get(&class) {
            return Ok(Some(known));
        }
        let (size, _) = types::layout(graph, ty)?;
        let mut classes = [Class::None; 2];
        let marker = 0u8;
        if size <= 16 {
            classify_fields(
                graph,
                ty,
                0,
                &mut classes,
                budget,
                std::ptr::from_ref(&marker).addr(),
            )?;
        }
        if classes[1] == Class::SseUp && classes[0] != Class::Sse {
            classes[1] = Class::Sse;
        }
        let result = if size > 16 {
            ObjectClasses::Memory
        } else if classes == [Class::X87, Class::X87Upper] {
            ObjectClasses::X87
        } else if let (Some(first), Some(second)) = (classes[0].direct(), classes[1].direct()) {
            // Even an all-integer host boundary states its classes. Empty
            // objects spell both eightbytes, as the frozen writer does.
            ObjectClasses::Direct(first, (size == 0 || size > 8).then_some(second))
        } else {
            ObjectClasses::Memory
        };
        self.classes.insert(class, result);
        Ok(Some(result))
    }
}

fn mark(classes: &mut [Class; 2], index: u64, class: Class) {
    let destination = match index {
        0 => &mut classes[0],
        1 => &mut classes[1],
        _ => {
            classes[0] = Class::Memory;
            return;
        }
    };
    *destination = destination.merge(class);
}

fn classify_fields(
    graph: &Graph,
    ty: TypeId,
    offset: u64,
    classes: &mut [Class; 2],
    budget: StackBudget,
    base: usize,
) -> Result<(), Error> {
    let marker = 0u8;
    if budget.exceeded(base, std::ptr::from_ref(&marker).addr()) {
        return Err(crate::sema::Error::Limit("constructs nested too deeply").into());
    }
    let view = graph.lower_type(ty)?;
    if matches!(view, TypeView::Array { bound: 0, .. }) {
        return Ok(());
    }
    let (size, align) = types::layout(graph, ty)?;
    if !offset.is_multiple_of(1u64 << align) {
        classes[0] = Class::Memory;
        return Ok(());
    }
    match view {
        TypeView::Complex(part) => {
            classify_fields(graph, part, offset, classes, budget, base)?;
            classify_fields(
                graph,
                part,
                offset.wrapping_add(size / 2),
                classes,
                budget,
                base,
            )?;
        }
        TypeView::Array { element, bound } => {
            let (bytes, _) = types::layout(graph, element)?;
            if bytes == 0 {
                classes[0] = Class::Memory;
            } else {
                for index in 0..bound {
                    classify_fields(
                        graph,
                        element,
                        offset.wrapping_add(index.wrapping_mul(bytes)),
                        classes,
                        budget,
                        base,
                    )?;
                }
            }
        }
        TypeView::Class(class) => {
            for edge in graph.lower_rtti_bases(class) {
                let (class, _, virtual_base, at) = edge?;
                if virtual_base {
                    classes[0] = Class::Memory;
                } else {
                    classify_fields(
                        graph,
                        graph.lower_entity_type(class)?,
                        offset.wrapping_add(at),
                        classes,
                        budget,
                        base,
                    )?;
                }
            }
            for &member in graph.lower_class_member_entities(class) {
                if let Some(field) = graph.lower_bit_field(member)? {
                    if field.width != 0 {
                        let first = offset.wrapping_mul(8).wrapping_add(field.bit);
                        for index in
                            first / 64..=first.wrapping_add(field.width).wrapping_sub(1) / 64
                        {
                            mark(classes, index, Class::Integer);
                        }
                    }
                } else {
                    classify_fields(
                        graph,
                        graph.lower_entity_type(member)?,
                        offset.wrapping_add(graph.lower_member_offset(member)?),
                        classes,
                        budget,
                        base,
                    )?;
                }
            }
        }
        _ if super::soft_float::kind(graph, ty)?.is_some() => {
            mark(classes, offset / 8, Class::Sse);
            if size == 16 {
                mark(classes, offset / 8 + 1, Class::SseUp);
            }
        }
        _ => match types::storage(graph, ty)? {
            Type::Void => (),
            Type::F32 | Type::F64 => mark(classes, offset / 8, Class::Sse),
            Type::F80 => {
                mark(classes, offset / 8, Class::X87);
                mark(classes, (offset / 8).wrapping_add(1), Class::X87Upper);
            }
            _ => {
                for index in offset / 8..=offset.wrapping_add(size).wrapping_sub(1) / 8 {
                    mark(classes, index, Class::Integer);
                }
            }
        },
    }
    Ok(())
}
