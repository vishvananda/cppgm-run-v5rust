// Translated from lower/virtual_base_lowering.cpp's construction contexts. See NOTICE.
//! Unit-owned context facts use complete subobject identity, independently of emission.
use super::{TableContext, TableTarget, ThisAdjustment, address, define, integer};
use crate::{
    abi::encoder::{Encoder, Target},
    lower::{Error, symbols::Symbols},
    lowir::{
        model::{DataItem, Metadata, Program, SymbolId},
        vocabulary::{Binding, Type},
    },
    sema::lowering::{EntityId, SlotKind},
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Contexts {
    owners: HashMap<TableContext, Vec<usize>>,
    offsets: HashMap<TableContext, Vec<u64>>,
    tables: HashMap<TableContext, Vec<Option<(SymbolId, u64)>>>,
    sizes: HashMap<EntityId, usize>,
    positions: HashMap<EntityId, Positions>,
}
struct Positions {
    bases: Vec<usize>,
    segments: Vec<usize>,
    virtuals: HashMap<EntityId, usize>,
}
impl Symbols<'_, '_> {
    pub(in crate::lower::symbols) fn context_offsets(
        &mut self,
        context: TableContext,
    ) -> Result<&[u64], Error> {
        if !context.construction {
            return self.segment_offsets(context.class);
        }
        if self.vtables.contexts.offsets.contains_key(&context) {
            return Ok(&self.vtables.contexts.offsets[&context]);
        }
        let group =
            self.graph
                .lower_vtable_group(context.class)
                .ok_or(crate::sema::Error::Invariant(
                    "construction context without group",
                ))?;
        let own = self.graph.lower_segment_offsets(context.class)?;
        let mut offsets = Vec::with_capacity(own.len());
        for (i, &at) in own.iter().enumerate() {
            let value = if let Some(within) = group.segments[i].within {
                let within = within.index();
                self.virtual_base_offset(context.complete, group.segments[within].class)?
                    .wrapping_add(at)
                    .wrapping_sub(own[within])
            } else {
                context.offset.wrapping_add(at)
            };
            offsets.push(value);
        }
        self.vtables.contexts.offsets.insert(context, offsets);
        Ok(&self.vtables.contexts.offsets[&context])
    }
    pub(in crate::lower::symbols) fn context_at(
        &mut self,
        context: TableContext,
        segment: usize,
    ) -> Result<u64, Error> {
        if !context.construction {
            return Ok(self.segment_offsets(context.class)?[segment]);
        }
        if !self.vtables.contexts.offsets.contains_key(&context) {
            self.context_offsets(context)?;
        }
        Ok(self.vtables.contexts.offsets[&context][segment])
    }
    fn prepare_context_owners(&mut self, context: TableContext) -> Result<(), Error> {
        if self.vtables.contexts.owners.contains_key(&context) {
            return Ok(());
        }
        self.context_offsets(context)?;
        let group =
            self.graph
                .lower_vtable_group(context.class)
                .ok_or(crate::sema::Error::Invariant(
                    "construction owners without group",
                ))?;
        let count = group.segments.len();
        let at = &self.vtables.contexts.offsets[&context];
        let mut heads = HashMap::new();
        for (i, part) in group.segments.iter().enumerate().skip(1).rev() {
            if let Some((primary, true)) = self.graph.lower_itanium_primary(part.class) {
                heads.insert((primary, at[i]), i);
            }
        }
        let mut chained = HashSet::new();
        let mut link = Some(context.class);
        while let Some(class) = link {
            link = match self.graph.lower_itanium_primary(class) {
                Some((primary, false)) => Some(primary),
                Some((primary, true)) => {
                    chained.insert(primary);
                    None
                }
                None => None,
            };
        }
        let mut parents = vec![None; count];
        for (i, part) in group.segments.iter().enumerate().skip(1) {
            if part.within.is_none_or(|id| id.index() != i) {
                if part.shared {
                    parents[i] = Some(part.parent.index());
                }
                continue;
            }
            parents[i] = heads.get(&(part.class, at[i])).copied().filter(|&k| k != i);
            if parents[i].is_none() && at[i] == at[0] && chained.contains(&part.class) {
                parents[i] = Some(0);
            }
        }
        let mut owners = vec![count; count];
        let mut path = Vec::new();
        for i in 0..count {
            path.clear();
            let mut root = i;
            while owners[root] == count {
                path.push(root);
                if path.len() > count {
                    return Err(crate::sema::Error::Invariant("cyclic construction owners").into());
                }
                let Some(parent) = parents[root] else {
                    owners[root] = root;
                    break;
                };
                root = parent;
            }
            let owner = owners[root];
            for &node in &path {
                owners[node] = owner;
            }
        }
        self.vtables.contexts.owners.insert(context, owners);
        Ok(())
    }
    pub(super) fn construction_adjustment(
        &mut self,
        context: TableContext,
        segment: usize,
        slot: usize,
    ) -> Result<ThisAdjustment, Error> {
        let group =
            self.graph
                .lower_vtable_group(context.class)
                .ok_or(crate::sema::Error::Invariant(
                    "construction adjustment without group",
                ))?;
        let part = &group.segments[segment];
        if !matches!(
            group.entries[part.entries.start + slot].kind,
            SlotKind::Function
        ) {
            return Ok(ThisAdjustment {
                unused: true,
                ..ThisAdjustment::default()
            });
        }
        self.context_adjustment(
            if part.shared {
                context
            } else {
                TableContext::complete(context.class)
            },
            segment,
            slot,
        )
    }
    fn construction_address(
        &mut self,
        program: &mut Program,
        context: TableContext,
        segment: usize,
    ) -> Result<(SymbolId, u64), Error> {
        self.check_stack()?;
        self.prepare_context_owners(context)?;
        let requested = self.vtables.contexts.owners[&context][segment];
        if let Some(tables) = self.vtables.contexts.tables.get(&context) {
            return tables[requested].ok_or_else(|| {
                crate::sema::Error::Invariant("construction table recursively demanded").into()
            });
        }
        let group =
            self.graph
                .lower_vtable_group(context.class)
                .ok_or(crate::sema::Error::Invariant(
                    "construction table without group",
                ))?;
        self.vtables
            .contexts
            .tables
            .insert(context, vec![None; group.segments.len()]);
        let derived = self.abi_type(self.graph.lower_entity_type(context.complete)?)?;
        let base = self.abi_type(self.graph.lower_entity_type(context.class)?)?;
        let mut encoded = Vec::new();
        Encoder::encode(
            &self.abi,
            Target::ConstructionVtable {
                derived,
                offset: i64::from_ne_bytes(context.offset.to_ne_bytes()),
                base,
            },
            &mut encoded,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&encoded)?;
        let (internal, _, _, _) = self.graph.lower_virtual_class_identity(context.complete)?;
        let strong = self
            .graph
            .lower_virtual_class(context.complete)?
            .key
            .map(|key| {
                self.graph
                    .lower_function(key)
                    .map(|f| f.emission == crate::sema::lowering::Emission::Always)
            })
            .transpose()?
            .unwrap_or(false);
        let binding = if internal {
            Binding::Internal
        } else if strong {
            Binding::Strong
        } else {
            Binding::Weak
        };
        // Entry demand precedes RTTI and data publication, as in C++.
        let mut bodies = Vec::new();
        for s in 0..group.segments.len() {
            let owner = self.vtables.contexts.owners[&context][s];
            if s != 0 && (s != owner || !self.vtt_segment(context.class, s)?) {
                continue;
            }
            let part = &group.segments[s];
            let mut targets = Vec::new();
            for (slot, own) in self
                .graph
                .lower_virtual_slots(part.class)?
                .iter()
                .enumerate()
            {
                let own = own.ok_or(crate::sema::Error::Invariant("unfilled construction slot"))?;
                let entry = group.entries[part.entries.start + slot];
                let adjustment = self.construction_adjustment(context, s, slot)?;
                targets.push(self.table_target(
                    program,
                    TableTarget {
                        function: entry.function,
                        kind: entry.kind,
                        adjustment,
                        adjusted: own.adjusted,
                        through: part.class,
                    },
                    Some(binding),
                )?);
            }
            bodies.push((s, targets));
        }
        let information = self.typeinfo_symbol(program, context.class)?;
        let base = self.class_presentation(context.class)?;
        let top = self.class_presentation(context.complete)?;
        let prefix = format!("{top}____construction__{base}__{}", context.offset);
        let mut tables = vec![None; group.segments.len()];
        for (s, targets) in bodies {
            let mut items = self.context_rows(context, s)?;
            let point = self.table_header_bytes(context.class, s)?;
            items.push(integer(
                Type::I64,
                context.offset.wrapping_sub(self.context_at(context, s)?),
            ));
            items.push(address(information, 0));
            items.extend(targets);
            let symbol = self.table.allocate(
                program,
                &format!("{prefix}__s{s}__vtable"),
                (s == 0).then_some(object),
                true,
            )?;
            define(
                program,
                symbol,
                Metadata {
                    binding: Some(Binding::Internal),
                    object: (s == 0).then_some(object),
                    ..Metadata::default()
                },
                items,
            )?;
            tables[s] = Some((symbol, point));
        }
        for s in 0..tables.len() {
            tables[s] = tables[self.vtables.contexts.owners[&context][s]];
        }
        let result = tables[requested].ok_or(crate::sema::Error::Invariant(
            "unemitted construction owner",
        ))?;
        self.vtables.contexts.tables.insert(context, tables);
        Ok(result)
    }
    fn vtt_segment(&self, class: EntityId, index: usize) -> Result<bool, Error> {
        let group = self
            .graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("VTT segment without group"))?;
        let part = &group.segments[index];
        Ok(
            (!part.shared || part.within.is_some_and(|id| id.index() == index))
                && (!self.graph.lower_virtual_bases(part.class).is_empty()
                    || part.within.is_some()),
        )
    }
    fn vtt_size(&mut self, class: EntityId) -> Result<usize, Error> {
        if let Some(&size) = self.vtables.contexts.sizes.get(&class) {
            return Ok(size);
        }
        self.check_stack()?;
        let mut size = 1usize;
        for (base, shared) in self.graph.lower_inheritance_edges(class) {
            if !shared && !self.graph.lower_virtual_bases(base).is_empty() {
                size = size
                    .checked_add(self.vtt_size(base)?)
                    .ok_or(crate::lowir::Error::Limit)?;
            }
        }
        if let Some(group) = self.graph.lower_vtable_group(class) {
            for s in 1..group.segments.len() {
                size = size
                    .checked_add(usize::from(self.vtt_segment(class, s)?))
                    .ok_or(crate::lowir::Error::Limit)?;
            }
        }
        self.vtables.contexts.sizes.insert(class, size);
        Ok(size)
    }
    fn vtt_positions(&mut self, class: EntityId) -> Result<(), Error> {
        if self.vtables.contexts.positions.contains_key(&class) {
            return Ok(());
        }
        let mut index = 1usize;
        let mut bases = Vec::new();
        for (base, shared) in self.graph.lower_inheritance_edges(class) {
            bases.push(index);
            if !shared && !self.graph.lower_virtual_bases(base).is_empty() {
                index = index
                    .checked_add(self.vtt_size(base)?)
                    .ok_or(crate::lowir::Error::Limit)?;
            }
        }
        let count = self
            .graph
            .lower_vtable_group(class)
            .map_or(1, |g| g.segments.len());
        let mut segments = vec![index; count];
        for segment in self.vtt_pointer_order(class)? {
            segments[segment] = index;
            index = index.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        }
        let mut virtuals = HashMap::new();
        let mut index = self.vtt_size(class)?;
        for &(base, _) in self.graph.lower_virtual_bases(class) {
            virtuals.insert(base, index);
            if !self.graph.lower_virtual_bases(base).is_empty() {
                index = index
                    .checked_add(self.vtt_size(base)?)
                    .ok_or(crate::lowir::Error::Limit)?;
            }
        }
        self.vtables.contexts.positions.insert(
            class,
            Positions {
                bases,
                segments,
                virtuals,
            },
        );
        Ok(())
    }
    pub(in crate::lower) fn vtt_base_index(
        &mut self,
        class: EntityId,
        index: usize,
    ) -> Result<usize, Error> {
        self.vtt_positions(class)?;
        Ok(self.vtables.contexts.positions[&class].bases[index])
    }
    pub(in crate::lower) fn vtt_virtual_index(
        &mut self,
        class: EntityId,
        base: EntityId,
    ) -> Result<usize, Error> {
        self.vtt_positions(class)?;
        self.vtables.contexts.positions[&class]
            .virtuals
            .get(&base)
            .copied()
            .ok_or_else(|| crate::sema::Error::Invariant("VTT virtual index outside class").into())
    }
    pub(in crate::lower) fn vtt_segment_index(
        &mut self,
        class: EntityId,
        segment: usize,
    ) -> Result<usize, Error> {
        self.vtt_positions(class)?;
        Ok(self.vtables.contexts.positions[&class].segments[segment])
    }
    pub(super) fn vtt_entries(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<Vec<DataItem>, Error> {
        let mut entries = Vec::new();
        self.append_vtt_subentries(program, TableContext::complete(class), true, &mut entries)?;
        Ok(entries)
    }
    fn append_vtt_subentries(
        &mut self,
        program: &mut Program,
        context: TableContext,
        top: bool,
        entries: &mut Vec<DataItem>,
    ) -> Result<(), Error> {
        self.check_stack()?;
        let (symbol, point) = if top {
            self.table_address(program, context.class, 0)?
        } else {
            self.construction_address(program, context, 0)?
        };
        entries.push(address(symbol, i64::from_ne_bytes(point.to_ne_bytes())));
        for (i, (base, shared)) in self
            .graph
            .lower_inheritance_edges(context.class)
            .enumerate()
        {
            if !shared && !self.graph.lower_virtual_bases(base).is_empty() {
                let (_, _, step) = self
                    .graph
                    .lower_inheritance_edge(context.class, i)
                    .ok_or(crate::sema::Error::Invariant("missing VTT edge"))?;
                let offset = context
                    .offset
                    .wrapping_add(step.ok_or(crate::sema::Error::Invariant("VTT before layout"))?);
                self.append_vtt_subentries(
                    program,
                    TableContext {
                        complete: context.complete,
                        class: base,
                        offset,
                        construction: true,
                    },
                    false,
                    entries,
                )?;
            }
        }
        for segment in self.vtt_pointer_order(context.class)? {
            let (symbol, point) = if top {
                self.table_address(program, context.class, segment)?
            } else {
                self.construction_address(program, context, segment)?
            };
            entries.push(address(symbol, i64::from_ne_bytes(point.to_ne_bytes())));
        }
        if top {
            for &(base, offset) in self.graph.lower_virtual_bases(context.complete) {
                if !self.graph.lower_virtual_bases(base).is_empty() {
                    self.append_vtt_subentries(
                        program,
                        TableContext {
                            complete: context.complete,
                            class: base,
                            offset,
                            construction: true,
                        },
                        false,
                        entries,
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn assert_construction_context(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    classes: &[EntityId],
) {
    let (complete, base, shared, dormant) = (classes[0], classes[1], classes[2], classes[3]);
    let mut table = crate::lower::symbols::SymbolTable::default();
    let mut symbols = Symbols::new(
        graph,
        tree,
        cursor,
        crate::support::stack::StackBudget::default(),
        &mut table,
    );
    let offset = graph
        .lower_nonvirtual_offset(complete, base)
        .unwrap()
        .unwrap();
    let context = TableContext {
        complete,
        class: base,
        offset,
        construction: true,
    };
    let offsets = symbols.context_offsets(context).unwrap().to_vec();
    let own = graph.lower_segment_offsets(base).unwrap();
    assert_eq!(offsets[0], offset);
    let shared_at = symbols.virtual_base_offset(complete, shared).unwrap();
    assert_ne!(
        shared_at.wrapping_sub(offset),
        symbols.virtual_base_offset(base, shared).unwrap()
    );
    assert!(offsets.contains(&shared_at));
    let rows = symbols.context_rows(context, 0).unwrap();
    assert!(
        matches!(&rows[0], DataItem::Constant(Type::I64, crate::lowir::literals::Constant::Integer(n)) if *n == u128::from(shared_at - offset))
    );
    symbols.prepare_context_owners(context).unwrap();
    let owners = symbols.vtables.contexts.owners[&context].clone();
    assert_eq!(owners.len(), offsets.len());
    assert_eq!(symbols.vtt_base_index(complete, 1).unwrap(), 1);
    let before = (
        symbols.vtables.contexts.offsets.len(),
        symbols.vtables.contexts.owners.len(),
    );
    for _ in 0..3 {
        assert_eq!(symbols.context_offsets(context).unwrap(), offsets);
        symbols.prepare_context_owners(context).unwrap();
        assert_eq!(symbols.vtables.contexts.owners[&context], owners);
        assert_eq!(symbols.vtt_base_index(complete, 1).unwrap(), 1);
    }
    assert_eq!(
        before,
        (
            symbols.vtables.contexts.offsets.len(),
            symbols.vtables.contexts.owners.len()
        )
    );
    assert_ne!(own, offsets);
    assert!(symbols.vtables.contexts.tables.is_empty());
    assert!(symbols.vtables.objects.is_empty());
    assert!(symbols.vtables.typeinfos.is_empty());
    assert_eq!(symbols.pending.len(), 0);
    assert!(graph.lower_class_layout(dormant).is_err());
}
