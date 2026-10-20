// Translated from lower/virtual_base_lowering.cpp and itanium_layout.cpp. See NOTICE.
//! Query-owned routes and class-owned negative-row recipes, without emission demand.
use super::{Error, Symbols};
use crate::lower::function::Function;
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::{EntityId, TypeId, VcallSignature},
};
use std::collections::{HashMap, HashSet};

pub(super) use crate::sema::lowering::BaseRoute as Route;
#[derive(Clone, Copy)]
enum Row {
    Base(EntityId),
    Call {
        signature: VcallSignature,
        function: EntityId,
        holder: EntityId,
        level: EntityId,
    },
}
#[derive(Default)]
pub(super) struct Routes {
    vtt_orders: HashMap<EntityId, Vec<usize>>,
    construction_orders: HashMap<EntityId, Vec<EntityId>>,
    projections: crate::sema::lowering::BaseRoutes,
    rows: HashMap<EntityId, Vec<Row>>,
    full_rows: HashMap<EntityId, Vec<Row>>,
    segments: HashMap<EntityId, HashMap<(u64, EntityId), usize>>,
    positions: HashMap<(EntityId, EntityId), i64>,
    vcall_positions: HashMap<(EntityId, VcallSignature), i64>,
}
impl Symbols<'_, '_> {
    pub(in crate::lower) fn base_route(
        &mut self,
        from: EntityId,
        to: EntityId,
    ) -> Result<Route, Error> {
        Ok(self.base_routes.projections.get(self.graph, from, to)?)
    }
    pub(in crate::lower) fn vbase_row_offset(
        &mut self,
        class: EntityId,
        base: EntityId,
    ) -> Result<i64, Error> {
        self.base_rows(class)?;
        self.base_routes
            .positions
            .get(&(class, base))
            .copied()
            .ok_or_else(|| crate::sema::Error::Invariant("virtual base without a table row").into())
    }
    fn base_rows(&mut self, root: EntityId) -> Result<(), Error> {
        if self.base_routes.rows.contains_key(&root) {
            return Ok(());
        }
        let mut chain = Vec::new();
        let mut next = Some((root, false));
        while let Some((class, full)) = next {
            if self.base_routes.rows.contains_key(&class) {
                if full {
                    self.full_base_rows(class)?;
                }
                break;
            }
            chain.push((class, full));
            next = if self.graph.lower_virtual_bases(class).is_empty() {
                None
            } else {
                self.graph.lower_itanium_primary(class)
            };
        }
        for (class, full) in chain.into_iter().rev() {
            let mut rows = Vec::new();
            let mut listed = HashSet::new();
            if !self.graph.lower_virtual_bases(class).is_empty() {
                if let Some((primary, virtual_edge)) = self.graph.lower_itanium_primary(class) {
                    let inherited = if virtual_edge {
                        self.base_routes.full_rows.get(&primary)
                    } else {
                        self.base_routes.rows.get(&primary)
                    };
                    let inherited = inherited.ok_or(crate::sema::Error::Invariant(
                        "inherited rows before primary recipe",
                    ))?;
                    rows.extend_from_slice(inherited);
                }
                for row in &rows {
                    if let Row::Base(base) = row {
                        listed.insert(*base);
                    }
                }
                for &(base, _) in self.graph.lower_virtual_bases(class) {
                    if listed.insert(base) {
                        rows.push(Row::Base(base));
                    }
                }
            }
            for (index, row) in rows.iter().enumerate() {
                if let Row::Base(base) = row {
                    let index = i64::try_from(index)
                        .map_err(|_| crate::sema::Error::Limit("vtable row limit"))?;
                    let offset = index
                        .checked_add(3)
                        .and_then(|n| n.checked_mul(-8))
                        .ok_or(crate::sema::Error::Limit("vtable row limit"))?;
                    self.base_routes.positions.insert((class, *base), offset);
                }
            }
            self.base_routes.rows.insert(class, rows);
            if full {
                self.full_base_rows(class)?;
            }
        }
        Ok(())
    }
    fn full_base_rows(&mut self, class: EntityId) -> Result<(), Error> {
        if self.base_routes.full_rows.contains_key(&class) {
            return Ok(());
        }
        let mut rows = self
            .base_routes
            .rows
            .get(&class)
            .cloned()
            .ok_or(crate::sema::Error::Invariant("full rows before base rows"))?;
        let mut signatures: HashSet<_> = rows
            .iter()
            .filter_map(|row| match row {
                Row::Call { signature, .. } => Some(*signature),
                Row::Base(_) => None,
            })
            .collect();
        let mut work = vec![(class, false)];
        let mut visited = HashSet::new();
        while let Some((holder, own)) = work.pop() {
            let primary = self
                .graph
                .lower_itanium_primary(holder)
                .filter(|&(_, shared)| !shared)
                .map(|(base, _)| base);
            if !own {
                if !visited.insert(holder) {
                    continue;
                }
                work.push((holder, true));
                if let Some(primary) = primary {
                    work.push((primary, false));
                }
                continue;
            }
            for function in self.graph.lower_vcall_functions(holder)? {
                let signature = self.graph.lower_vcall_signature(function)?;
                if signatures.insert(signature) {
                    rows.push(Row::Call {
                        signature,
                        function,
                        holder,
                        level: class,
                    });
                }
            }
            for base in self.graph.lower_nonvirtual_bases(holder).rev() {
                if Some(base) != primary && self.graph.lower_dynamic(base) {
                    work.push((base, false));
                }
            }
        }
        for (index, row) in rows.iter().enumerate() {
            if let Row::Call { signature, .. } = row {
                let index = i64::try_from(index)
                    .map_err(|_| crate::sema::Error::Limit("vcall row limit"))?;
                let position = index
                    .checked_add(3)
                    .and_then(|n| n.checked_mul(-8))
                    .ok_or(crate::sema::Error::Limit("vcall row limit"))?;
                self.base_routes
                    .vcall_positions
                    .insert((class, *signature), position);
            }
        }
        self.base_routes.full_rows.insert(class, rows);
        Ok(())
    }
}
impl Function<'_, '_, '_, '_> {
    pub(in crate::lower) fn virtual_base_address(
        &mut self,
        object: Value,
        source: TypeId,
        class: EntityId,
        base: EntityId,
    ) -> Result<Value, Error> {
        if (self.virtual_context.pending || self.virtual_context.vtt.is_some())
            && self.virtual_context.this_values.contains(&object)
            && self.virtual_context.class == Some(class)
        {
            let displacement = if self.virtual_context.vtt.is_some() {
                self.construction_row(source, class, base)?
            } else {
                self.builder
                    .integer(u128::from(self.symbols.virtual_base_offset(class, base)?))?
            };
            return Ok(self
                .emit(I::Index(Type::I8, None, object, displacement), source)?
                .value);
        }
        let row_offset = self.symbols.vbase_row_offset(class, base)?;
        let vptr = self
            .emit(
                I::Load(Facts::memory(false, true), Type::Ptr, object),
                source,
            )?
            .value;
        let offset = self
            .builder
            .integer(u128::from_le_bytes(i128::from(row_offset).to_le_bytes()))?;
        let row = self
            .emit(I::Index(Type::I8, None, vptr, offset), source)?
            .value;
        let displacement = self
            .emit(I::Load(Facts::memory(false, true), Type::I64, row), source)?
            .value;
        Ok(self
            .emit(I::Index(Type::I8, None, object, displacement), source)?
            .value)
    }
}

#[cfg(test)]
pub(crate) fn assert_base_routes(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    classes: &[EntityId],
) {
    use crate::support::stack::StackBudget;
    let (derived, branch, shared, inside, other_shared, dormant) = (
        classes[0], classes[1], classes[2], classes[3], classes[4], classes[5],
    );
    assert!(graph.lower_class_layout(dormant).is_err());
    let slot_demand: Vec<_> = classes
        .iter()
        .map(|&class| graph.lower_virtual_slots(class).is_ok())
        .collect();
    let program = crate::lowir::model::Program::default();
    let mut table = super::SymbolTable::default();
    let mut symbols = Symbols::new(graph, tree, cursor, StackBudget::default(), &mut table);
    let route = symbols.base_route(derived, inside).unwrap();
    assert_eq!(route.via, Some(shared));
    assert_eq!(
        route.offset,
        graph
            .lower_nonvirtual_offset(shared, inside)
            .unwrap()
            .unwrap()
    );
    assert_eq!(symbols.base_route(derived, branch).unwrap().via, None);
    assert_eq!(symbols.vbase_row_offset(branch, shared).unwrap(), -24);
    assert_eq!(symbols.vbase_row_offset(derived, shared).unwrap(), -24);
    assert_eq!(
        symbols.vbase_row_offset(derived, other_shared).unwrap(),
        -32
    );
    let (extended, virtual_primary, polymorphic) = (classes[6], classes[7], classes[8]);
    assert_eq!(
        symbols
            .vbase_row_offset(virtual_primary, polymorphic)
            .unwrap(),
        -48
    );
    assert_eq!(
        symbols.vbase_row_offset(extended, polymorphic).unwrap(),
        -48
    );
    assert_eq!(
        symbols.vbase_row_offset(extended, other_shared).unwrap(),
        -56
    );
    let counts = (
        symbols.base_routes.projections.len(),
        symbols.base_routes.rows.len(),
        symbols.base_routes.positions.len(),
    );
    for _ in 0..3 {
        assert_eq!(symbols.base_route(derived, inside).unwrap().via, route.via);
        assert_eq!(
            symbols.vbase_row_offset(derived, other_shared).unwrap(),
            -32
        );
    }
    assert_eq!(
        counts,
        (
            symbols.base_routes.projections.len(),
            symbols.base_routes.rows.len(),
            symbols.base_routes.positions.len()
        )
    );
    assert_eq!(
        slot_demand,
        classes
            .iter()
            .map(|&class| graph.lower_virtual_slots(class).is_ok())
            .collect::<Vec<_>>()
    );
    assert_eq!(symbols.pending.len(), 0);
    assert!(symbols.entries.is_empty());
    assert!(program.globals().is_empty());
    assert!(program.symbols().is_empty());
}

impl Symbols<'_, '_> {
    pub(super) fn table_row_count(
        &mut self,
        class: EntityId,
        segment: usize,
    ) -> Result<usize, Error> {
        let (holder, full) = self
            .graph
            .lower_vtable_group(class)
            .map_or((class, false), |group| {
                let part = &group.segments[segment];
                (
                    part.class,
                    part.within.is_some_and(|id| id.index() == segment),
                )
            });
        self.base_rows(holder)?;
        if full {
            self.full_base_rows(holder)?;
        }
        Ok(if full {
            self.base_routes.full_rows[&holder].len()
        } else {
            self.base_routes.rows[&holder].len()
        })
    }
    pub(super) fn table_rows(
        &mut self,
        class: EntityId,
        segment: usize,
    ) -> Result<Vec<crate::lowir::model::DataItem>, Error> {
        self.context_rows(super::vtables::TableContext::complete(class), segment)
    }
    pub(super) fn context_rows(
        &mut self,
        context: super::vtables::TableContext,
        segment: usize,
    ) -> Result<Vec<crate::lowir::model::DataItem>, Error> {
        let class = context.class;
        let count = self.table_row_count(class, segment)?;
        let group = self.graph.lower_vtable_group(class);
        let holder = group.map_or(class, |g| g.segments[segment].class);
        let full = group.is_some_and(|g| {
            g.segments[segment]
                .within
                .is_some_and(|id| id.index() == segment)
        });
        let table_at = self.context_at(context, segment)?;
        let mut values = Vec::with_capacity(count);
        for index in (0..count).rev() {
            let row = if full {
                self.base_routes.full_rows[&holder][index]
            } else {
                self.base_routes.rows[&holder][index]
            };
            let at = match row {
                Row::Base(base) => self.virtual_base_offset(context.complete, base)?,
                Row::Call {
                    function,
                    holder: declaring,
                    level,
                    ..
                } => {
                    let level = if level == holder {
                        segment
                    } else {
                        self.group_segment(class, self.virtual_base_offset(class, level)?, level)?
                    };
                    let relative = self
                        .graph
                        .lower_nonvirtual_offset(
                            group
                                .ok_or(crate::sema::Error::Invariant("vcall row without group"))?
                                .segments[level]
                                .class,
                            declaring,
                        )?
                        .ok_or(crate::sema::Error::Invariant("vcall row outside its level"))?;
                    let at = self.segment_offsets(class)?[level].wrapping_add(relative);
                    let k = self.group_segment(class, at, declaring)?;
                    let slot = self
                        .graph
                        .lower_virtual_slot(function)
                        .ok_or(crate::sema::Error::Invariant("vcall function without slot"))?
                        .index()?;
                    let group =
                        group.ok_or(crate::sema::Error::Invariant("vcall without entries"))?;
                    self.context_at(
                        context,
                        group.entries[group.segments[k].entries.start + slot]
                            .overrider
                            .index(),
                    )?
                }
            };
            values.push(super::vtables::integer(
                Type::I64,
                at.wrapping_sub(table_at),
            ));
        }
        Ok(values)
    }
    pub(super) fn virtual_base_offset(
        &self,
        class: EntityId,
        base: EntityId,
    ) -> Result<u64, Error> {
        self.graph
            .lower_virtual_base_offset(class, base)
            .ok_or_else(|| {
                crate::sema::Error::Invariant("virtual base offset before layout").into()
            })
    }
    pub(super) fn group_segment(
        &mut self,
        class: EntityId,
        offset: u64,
        base: EntityId,
    ) -> Result<usize, Error> {
        if !self.base_routes.segments.contains_key(&class) {
            let mut result = HashMap::new();
            let group = self
                .graph
                .lower_vtable_group(class)
                .ok_or(crate::sema::Error::Invariant("segment query without group"))?;
            for (index, at) in self
                .graph
                .lower_segment_offsets(class)?
                .into_iter()
                .enumerate()
                .skip(1)
            {
                result
                    .entry((at, group.segments[index].class))
                    .or_insert(index);
            }
            self.base_routes.segments.insert(class, result);
        }
        self.base_routes.segments[&class]
            .get(&(offset, base))
            .copied()
            .ok_or_else(|| {
                crate::sema::Error::Invariant("a virtual base's subobject without a table segment")
                    .into()
            })
    }
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn vtt_pointer_order(
        &mut self,
        class: EntityId,
    ) -> Result<Vec<usize>, Error> {
        if let Some(order) = self.base_routes.vtt_orders.get(&class) {
            return Ok(order.clone());
        }
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        let mut work = vec![(class, 0u64, false, 0usize)];
        while let Some((holder, offset, moral, next)) = work.last_mut() {
            if self.graph.lower_virtual_bases(*holder).is_empty() && !*moral {
                work.pop();
                continue;
            }
            let Some((base, shared, step)) = self.graph.lower_inheritance_edge(*holder, *next)
            else {
                work.pop();
                continue;
            };
            *next += 1;
            if !self.graph.lower_dynamic(base) {
                continue;
            }
            let mut at = *offset;
            let mut virtual_below = *moral;
            if shared {
                if !seen.insert(base) {
                    continue;
                }
                at = self.virtual_base_offset(class, base)?;
                virtual_below = true;
            } else {
                at = at.wrapping_add(
                    step.ok_or(crate::sema::Error::Invariant("VTT walk before base layout"))?,
                );
            }
            let primary = self.graph.lower_itanium_primary(*holder);
            if (shared || primary != Some((base, false)))
                && (virtual_below || !self.graph.lower_virtual_bases(base).is_empty())
            {
                result.push(self.group_segment(class, at, base)?);
            }
            work.push((base, at, virtual_below, 0));
        }
        let group = self
            .graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("VTT without group"))?;
        let expected = group
            .segments
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(index, segment)| {
                (!segment.shared || segment.within.is_some_and(|id| id.index() == *index))
                    && (!self.graph.lower_virtual_bases(segment.class).is_empty()
                        || segment.within.is_some())
            })
            .count();
        if result.len() != expected {
            return Err(crate::sema::Error::Invariant(
                "a VTT whose virtual pointers the inheritance graph does not list",
            )
            .into());
        }
        self.base_routes.vtt_orders.insert(class, result.clone());
        Ok(result)
    }
}

impl Symbols<'_, '_> {
    pub(super) fn vcall_row_offset(
        &mut self,
        class: EntityId,
        function: EntityId,
    ) -> Result<i64, Error> {
        self.base_rows(class)?;
        self.full_base_rows(class)?;
        let signature = self.graph.lower_vcall_signature(function)?;
        self.base_routes
            .vcall_positions
            .get(&(class, signature))
            .copied()
            .ok_or_else(|| {
                crate::sema::Error::Invariant(
                    "a virtual base without a vcall offset for a function its table overrides",
                )
                .into()
            })
    }
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn virtual_construction_order(
        &mut self,
        class: EntityId,
    ) -> Vec<EntityId> {
        if let Some(order) = self.base_routes.construction_orders.get(&class) {
            return order.clone();
        }
        let mut pending = vec![class];
        while let Some(&top) = pending.last() {
            let before = pending.len();
            for (base, _) in self.graph.lower_inheritance_edges(top) {
                if !self.graph.lower_virtual_bases(base).is_empty()
                    && !self.base_routes.construction_orders.contains_key(&base)
                {
                    pending.push(base);
                }
            }
            if pending.len() != before {
                continue;
            }
            pending.pop();
            if self.base_routes.construction_orders.contains_key(&top) {
                continue;
            }
            let mut seen = HashSet::new();
            let mut order = Vec::new();
            for (base, shared) in self.graph.lower_inheritance_edges(top) {
                if let Some(inherited) = self.base_routes.construction_orders.get(&base) {
                    order.extend(inherited.iter().copied().filter(|&base| seen.insert(base)));
                }
                if shared && seen.insert(base) {
                    order.push(base);
                }
            }
            self.base_routes.construction_orders.insert(top, order);
        }
        self.base_routes.construction_orders[&class].clone()
    }
}
