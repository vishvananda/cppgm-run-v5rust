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

#[derive(Clone, Copy)]
pub(in crate::lower) struct Route {
    pub via: Option<EntityId>,
    pub offset: u64,
}
#[derive(Clone, Copy)]
enum Row {
    Base(EntityId),
    Call(VcallSignature),
}
#[derive(Default)]
pub(super) struct Routes {
    selected: HashMap<(EntityId, EntityId), Route>,
    holders: HashMap<EntityId, HashMap<EntityId, Route>>,
    rows: HashMap<EntityId, Vec<Row>>,
    full_rows: HashMap<EntityId, Vec<Row>>,
    positions: HashMap<(EntityId, EntityId), i64>,
}
impl Symbols<'_, '_> {
    pub(in crate::lower) fn base_route(
        &mut self,
        from: EntityId,
        to: EntityId,
    ) -> Result<Route, Error> {
        if let Some(&route) = self.base_routes.selected.get(&(from, to)) {
            return Ok(route);
        }
        let mut route = Route {
            via: None,
            offset: 0,
        };
        if let Some(offset) = self.graph.lower_nonvirtual_offset(from, to)? {
            route.offset = offset;
        } else {
            self.virtual_holders(from)?;
            route = self.base_routes.holders[&from].get(&to).copied().ok_or(
                crate::sema::Error::Invariant("virtual projection without a base route"),
            )?;
        }
        self.base_routes.selected.insert((from, to), route);
        Ok(route)
    }
    fn virtual_holders(&mut self, from: EntityId) -> Result<(), Error> {
        if self.base_routes.holders.contains_key(&from) {
            return Ok(());
        }
        let mut holders = HashMap::new();
        let mut work = Vec::new();
        for &(base, _) in self.graph.lower_virtual_bases(from) {
            work.push((base, 0u64));
            while let Some((class, offset)) = work.pop() {
                if holders.contains_key(&class) {
                    continue;
                }
                holders.insert(
                    class,
                    Route {
                        via: Some(base),
                        offset,
                    },
                );
                for (child, step) in self.graph.lower_nonvirtual_base_offsets(class).rev() {
                    work.push((child, offset.wrapping_add(step?)));
                }
            }
        }
        self.base_routes.holders.insert(from, holders);
        Ok(())
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
                Row::Call(signature) => Some(*signature),
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
                    rows.push(Row::Call(signature));
                }
            }
            for base in self.graph.lower_nonvirtual_bases(holder).rev() {
                if Some(base) != primary && self.graph.lower_polymorphic(base) {
                    work.push((base, false));
                }
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
        symbols.base_routes.selected.len(),
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
            symbols.base_routes.selected.len(),
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
