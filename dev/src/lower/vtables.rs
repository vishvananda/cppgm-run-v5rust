// Translated from lower/vtable_lowering.cpp and rtti_lowering.cpp. See NOTICE.
//! Demanded primary tables and readonly class RTTI, keyed by semantic class identity.
#[path = "construction_tables.rs"]
mod construction_tables;
use super::Symbols;
use crate::{
    abi::encoder::{Encoder, Target},
    lex::vocabulary::Keyword,
    lower::{Error, construction::Entry, function, types},
    lowir::{
        literals::Constant,
        model::{
            DataItem, Global, GlobalInit, Metadata, Program, Returns, SignatureId, SymbolId, Unwind,
        },
        vocabulary::{Binding, Role, Storage, Type},
    },
    sema::lowering::{Emission, EntityId, SlotKind},
};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) enum InformationKind {
    Class,
    SingleInheritance,
    MultipleInheritance,
    Fundamental,
    Pointer,
    MemberPointer,
    Function,
    Array,
    Enum,
}
#[derive(Default)]
pub(super) struct Tables {
    objects: HashMap<EntityId, SymbolId>,
    typeinfos: HashMap<EntityId, (SymbolId, Metadata)>,
    pending: std::collections::VecDeque<(SymbolId, Metadata)>,
    names: HashMap<EntityId, SymbolId>,
    signatures: HashMap<EntityId, SignatureId>,
    pure_virtual: Option<SymbolId>,
    views: HashMap<EntityId, Vec<Option<SymbolId>>>,
    vtts: HashMap<EntityId, SymbolId>,
    offsets: HashMap<EntityId, Vec<u64>>,
    address_points: HashMap<EntityId, Vec<u64>>,
    thunks: Vec<Thunk>,
    pending_thunk: usize,
    thunk_families: HashMap<(EntityId, ThisAdjustment), crate::lower::definition_order::FamilyId>,
    thunk_symbols: HashMap<ThunkKey, usize>,
    definition_classes: std::collections::HashSet<EntityId>,
    definitions_indexed: bool,
    construction_sensitive: std::collections::HashSet<EntityId>,
    definition_targets: DefinitionTargets,
    own_entry_thunks: HashMap<(EntityId, usize), bool>,
    contexts: construction_tables::Contexts,
    return_adjustments: HashMap<(EntityId, EntityId, EntityId), Option<ResultAdjustment>>,
}
/// Immutable ABI query context; complete and construction tables share row recipes.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct TableContext {
    pub complete: EntityId,
    pub class: EntityId,
    pub offset: u64,
    pub construction: bool,
}
impl TableContext {
    pub fn complete(class: EntityId) -> Self {
        Self {
            complete: class,
            class,
            offset: 0,
            construction: false,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ResultAdjustment {
    // Canonical route identities remain part of the thunk key, even if two
    // distinct returned layouts happen to use the same negative row number.
    from: EntityId,
    via: Option<EntityId>,
    offset: i64,
    reference: bool,
}

#[cfg(test)]
pub(crate) fn assert_covariant_routes(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    classes: &[EntityId],
    reference: bool,
) {
    use crate::support::stack::StackBudget;
    let (class, dormant) = (classes[0], classes[1]);
    assert!(graph.lower_class_layout(dormant).is_err());
    let slots = graph.lower_virtual_slots(class).unwrap();
    let (old, own) = (slots[0].unwrap(), slots[1].unwrap());
    assert_eq!(old.function, own.function);
    let mut table = super::SymbolTable::default();
    let mut symbols = Symbols::new(graph, tree, cursor, StackBudget::default(), &mut table);
    let result = symbols
        .result_adjustment(own.function, old.kind, old.adjusted, class)
        .unwrap()
        .unwrap();
    assert_eq!(result.reference, reference);
    assert_eq!(result.offset, 12);
    assert!(result.via.is_some());
    let counts = symbols.vtables.return_adjustments.len();
    for _ in 0..3 {
        assert!(
            symbols
                .result_adjustment(own.function, old.kind, old.adjusted, class)
                .unwrap()
                == Some(result)
        );
    }
    assert_eq!(counts, symbols.vtables.return_adjustments.len());
    assert!(
        symbols
            .result_adjustment(own.function, own.kind, own.adjusted, class)
            .unwrap()
            .is_none()
    );
    assert!(symbols.vtables.objects.is_empty());
    assert!(symbols.vtables.typeinfos.is_empty());
    assert!(symbols.vtables.thunks.is_empty());
    assert_eq!(symbols.pending.len(), 0);
    assert!(graph.lower_class_layout(dormant).is_err());
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash)]
struct ThisAdjustment {
    offset: i64,
    vcall: Option<i64>,
    unused: bool,
}
impl ThisAdjustment {
    fn fixed(offset: i64) -> Self {
        Self {
            offset,
            ..Self::default()
        }
    }
    fn is_zero(self) -> bool {
        self.offset == 0 && self.vcall.is_none()
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ThunkKey {
    entry: Entry,
    adjustment: ThisAdjustment,
    result: Option<ResultAdjustment>,
    internal: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct TableTarget {
    function: EntityId,
    kind: SlotKind,
    adjustment: ThisAdjustment,
    adjusted: EntityId,
    through: EntityId,
}
// One flat unit-owned store. Each function's order is most-recent-first;
// seeing a complete key again moves its existing node to the head.
#[derive(Default)]
struct DefinitionTargets {
    nodes: Vec<DefinitionTarget>,
    seen: HashMap<TableTarget, usize>,
    heads: HashMap<EntityId, usize>,
}
struct DefinitionTarget {
    target: TableTarget,
    previous: Option<usize>,
    next: Option<usize>,
}
impl DefinitionTargets {
    fn note(&mut self, target: TableTarget) {
        // DefinitionKey excludes the unused-entry sentinel. A repeated key
        // moves the original node, retaining its first observed adjustment.
        let key = TableTarget {
            adjustment: ThisAdjustment {
                unused: false,
                ..target.adjustment
            },
            ..target
        };
        let head = self.heads.get(&target.function).copied();
        let index = if let Some(&index) = self.seen.get(&key) {
            if head == Some(index) {
                return;
            }
            let node = &self.nodes[index];
            let (previous, next) = (node.previous, node.next);
            if let Some(previous) = previous {
                self.nodes[previous].next = next;
            }
            if let Some(next) = next {
                self.nodes[next].previous = previous;
            }
            index
        } else {
            let index = self.nodes.len();
            self.nodes.push(DefinitionTarget {
                target,
                previous: None,
                next: None,
            });
            self.seen.insert(key, index);
            index
        };
        self.nodes[index].next = head;
        self.nodes[index].previous = None;
        if let Some(head) = head {
            self.nodes[head].previous = Some(index);
        }
        self.heads.insert(target.function, index);
    }
}
struct Thunk {
    symbol: SymbolId,
    target: SymbolId,
    signature: SignatureId,
    this_index: usize,
    adjustment: ThisAdjustment,
    result: Option<ResultAdjustment>,
    result_row: Option<i64>,
    jump: bool,
    family: Option<(
        EntityId,
        Option<crate::lowir::model::NameId>,
        crate::lower::construction::Variant,
    )>,
    metadata: Metadata,
}
#[derive(Clone, Copy)]
enum ClassObject {
    Table,
    Vtt,
    Typeinfo,
    Name,
}
pub(super) fn integer(ty: Type, value: u64) -> DataItem {
    DataItem::Constant(ty, Constant::Integer(u128::from(value)))
}
pub(super) fn address(symbol: SymbolId, offset: i64) -> DataItem {
    DataItem::Address(Type::Ptr, symbol, offset)
}
fn defined(program: &Program, symbol: SymbolId) -> bool {
    program
        .global(program.symbol(symbol))
        .is_some_and(|global| matches!(global.init, GlobalInit::Data(_)))
}
pub(super) fn define(
    program: &mut Program,
    symbol: SymbolId,
    mut metadata: Metadata,
    items: Vec<DataItem>,
) -> Result<(), Error> {
    metadata.storage = Some(Storage::ReadOnly);
    program.add_global(
        symbol,
        Global {
            name: program.symbol(symbol).name,
            ty: None,
            metadata,
            init: GlobalInit::Data(items),
        },
    )?;
    Ok(())
}
impl Symbols<'_, '_> {
    pub(in crate::lower) fn finish_class_declarations(
        &mut self,
        program: &mut Program,
        before: Option<SymbolId>,
    ) -> Result<(), Error> {
        while self
            .vtables
            .pending
            .front()
            .is_some_and(|(symbol, _)| before.is_none_or(|limit| symbol.index() < limit.index()))
        {
            let (symbol, metadata) = self
                .vtables
                .pending
                .pop_front()
                .ok_or(crate::sema::Error::Invariant("pending RTTI declaration"))?;
            if program.symbol(symbol).declared() {
                continue;
            }
            program.add_global(
                symbol,
                Global {
                    name: program.symbol(symbol).name,
                    ty: None,
                    metadata: Metadata {
                        binding: Some(if metadata.binding == Some(Binding::Internal) {
                            Binding::Internal
                        } else {
                            Binding::Strong
                        }),
                        ..metadata
                    },
                    init: GlobalInit::Declaration,
                },
            )?;
            self.note_deferred_global(symbol);
        }
        Ok(())
    }
    fn class_encoding(
        &mut self,
        class: EntityId,
        kind: ClassObject,
    ) -> Result<(String, String), Error> {
        let node = self.abi_type(self.graph.lower_entity_type(class)?)?;
        let mut encoded = Vec::new();
        Encoder::encode(&self.abi, Target::Type(node), &mut encoded, self.budget)?;
        let name = String::from_utf8(encoded)
            .map_err(|_| crate::sema::Error::Invariant("non-UTF8 ABI class name"))?;
        let mut object = Vec::new();
        self.table.work.mangled_names += 1;
        Encoder::encode(
            &self.abi,
            match kind {
                ClassObject::Table => Target::Vtable(node),
                ClassObject::Vtt => Target::Vtt(node),
                ClassObject::Typeinfo => Target::Typeinfo(node),
                ClassObject::Name => Target::TypeinfoName(node),
            },
            &mut object,
            self.budget,
        )?;
        Ok((
            name,
            String::from_utf8(object)
                .map_err(|_| crate::sema::Error::Invariant("non-UTF8 ABI class object"))?,
        ))
    }
    fn class_global(
        &mut self,
        program: &mut Program,
        class: EntityId,
        kind: ClassObject,
    ) -> Result<(SymbolId, Metadata), Error> {
        let (encoding, object) = self.class_encoding(class, kind)?;
        let (mut internal, local, key, scope) = self.graph.lower_virtual_class_identity(class)?;
        if self.table.object_mode {
            internal = self
                .graph
                .lower_object_type_unit_local(self.graph.lower_entity_type(class)?)?;
        }
        internal |= matches!(kind, ClassObject::Typeinfo)
            && self.host_objects()
            && !self.graph.lower_class_complete(class);
        let prefix = match kind {
            ClassObject::Table => "__vtable_",
            ClassObject::Vtt => "__vtt_",
            ClassObject::Typeinfo => "__rtti_",
            ClassObject::Name => "__typeinfo_name_",
        };
        let spelling = if matches!(kind, ClassObject::Vtt) {
            format!("{}____vtt", self.class_presentation(class)?)
        } else if internal && local {
            format!("{prefix}type_{encoding}")
        } else {
            let mut components = Vec::new();
            if self.graph.lower_scope_class(scope) != Some(class) {
                components.push(self.specialized_class_label(class)?);
            }
            let mut current = scope;
            while let (Some(parent), name) = self.graph.lower_presentation_scope(current) {
                components.push(self.presentation_scope_label(current, name)?);
                current = parent;
            }
            components.reverse();
            let label = components.join("__");
            if matches!(kind, ClassObject::Table | ClassObject::Vtt) {
                format!(
                    "{label}{}",
                    if matches!(kind, ClassObject::Vtt) {
                        "____vtt"
                    } else {
                        "__vtable"
                    }
                )
            } else {
                let key = match key {
                    Keyword::Class => "class",
                    Keyword::Union => "union",
                    _ => "struct",
                };
                format!(
                    "{prefix}{}{key}_{label}",
                    if matches!(kind, ClassObject::Name) {
                        "_"
                    } else {
                        ""
                    }
                )
            }
        };
        let object = program.names.intern(&object)?;
        // local_object: only the first internal symbol with this ABI spelling
        // owns it. Later definitions use their unique LowIR symbol name.
        let object_name =
            (!internal || !self.table.objects.contains_key(&object)).then_some(object);
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), internal)?;
        let metadata = Metadata {
            binding: Some(if internal {
                Binding::Internal
            } else {
                Binding::Weak
            }),
            object: object_name,
            ..Metadata::default()
        };
        if !program.symbol(symbol).declared() {
            program.set_forward_global(
                symbol,
                crate::lowir::model::ForwardGlobal {
                    binding: Some(if internal {
                        Binding::Internal
                    } else {
                        Binding::Strong
                    }),
                    storage: None,
                    defined: false,
                },
            );
            self.vtables.pending.push_back((symbol, metadata));
        }
        Ok((symbol, metadata))
    }
    pub(in crate::lower) fn construction_table(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<(), Error> {
        if self.graph.lower_dynamic_class(class) && self.graph.lower_emits_vtable(class)? {
            self.vtable_symbol(program, class)?;
        }
        Ok(())
    }
    pub(in crate::lower) fn vtable_symbol(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<SymbolId, Error> {
        self.check_stack()?;
        if let Some(&symbol) = self.vtables.objects.get(&class) {
            return Ok(symbol);
        }
        let facts = self.graph.lower_virtual_class(class)?;
        let key = facts.key;
        let (symbol, mut metadata) = self.class_global(program, class, ClassObject::Table)?;
        self.vtables.objects.insert(class, symbol);
        if defined(program, symbol) || !self.graph.lower_emits_vtable(class)? {
            return Ok(symbol);
        }
        if let Some(key) = key
            && self.function_facts(key)?.emission == Emission::Always
            && metadata.binding != Some(Binding::Internal)
        {
            metadata.binding = Some(Binding::Strong);
        }
        let typeinfo = self.typeinfo_symbol(program, class)?;
        let slots = self.graph.lower_virtual_slots(class)?;
        let mut items = self.table_rows(class, 0)?;
        items.reserve(slots.len() + 2);
        items.push(integer(Type::I64, 0));
        items.push(address(typeinfo, 0));
        let group = self.graph.lower_vtable_group(class);
        for (index, slot) in slots.iter().enumerate() {
            let slot = slot.ok_or(crate::sema::Error::Invariant("unfilled virtual slot"))?;
            let (function, kind) = group.map_or((slot.function, slot.kind), |group| {
                let entry = group.entries[index];
                (entry.function, entry.kind)
            });
            let adjustment = self.entry_adjustment(class, 0, index)?;
            items.push(self.table_target(
                program,
                TableTarget {
                    function,
                    kind,
                    adjustment,
                    adjusted: slot.adjusted,
                    through: class,
                },
                metadata.binding,
            )?);
        }
        let one_object = self.table.object_mode && group.is_some();
        if one_object {
            self.append_group_items(program, class, metadata.binding, &mut items)?;
        }
        define(program, symbol, metadata, items)?;
        self.table.work.vtables += 1;
        if group.is_some() && !one_object {
            self.define_views(program, class, symbol, metadata.binding)?;
        }
        if !self.graph.lower_virtual_bases(class).is_empty() {
            self.vtt_symbol(program, class)?;
        }
        if !self.host_objects() && self.graph.lower_vtable_group(class).is_some() {
            for (base, _) in self.graph.lower_inheritance_edges(class) {
                self.construction_table(program, base)?;
            }
        }
        Ok(symbol)
    }
    fn table_target(
        &mut self,
        program: &mut Program,
        target: TableTarget,
        binding: Option<Binding>,
    ) -> Result<DataItem, Error> {
        Ok(self
            .table_entry_symbol(program, target, binding, false)?
            .map_or_else(|| integer(Type::Ptr, 0), |symbol| address(symbol, 0)))
    }
    fn table_entry_symbol(
        &mut self,
        program: &mut Program,
        target: TableTarget,
        binding: Option<Binding>,
        strong: bool,
    ) -> Result<Option<SymbolId>, Error> {
        let TableTarget {
            function,
            kind,
            adjustment,
            adjusted,
            through,
        } = target;
        if adjustment.unused {
            return Ok(None);
        }
        if self.graph.lower_virtual_pure(function) {
            return Ok(Some(self.pure_virtual_symbol(program, function)?));
        }
        let entry = if matches!(kind, SlotKind::DeletingDestructor) {
            Entry::DestructorDeleting(function)
        } else {
            Entry::Function(function)
        };
        let result = self.result_adjustment(function, kind, adjusted, through)?;
        if matches!(kind, SlotKind::CompleteDestructor) {
            // Both table data and definition-owned thunks request the pair,
            // even when this unit never emits the class's own table.
            self.demand_entry(Entry::DestructorDeleting(function));
            self.demand_entry(Entry::Function(function));
        } else if matches!(kind, SlotKind::Function) {
            self.demand(function);
        }
        let symbol = if !adjustment.is_zero() || result.is_some() {
            self.thunk_symbol(program, entry, adjustment, result, binding, strong)?
        } else {
            self.entry_symbol(program, entry)?
        };
        Ok(Some(symbol))
    }
    fn result_adjustment(
        &mut self,
        function: EntityId,
        kind: SlotKind,
        adjusted: EntityId,
        through: EntityId,
    ) -> Result<Option<ResultAdjustment>, Error> {
        if !matches!(kind, SlotKind::Function) || function == adjusted {
            return Ok(None);
        }
        let key = (function, adjusted, through);
        if let Some(&known) = self.vtables.return_adjustments.get(&key) {
            return Ok(known);
        }
        let Some(classes) = self
            .graph
            .lower_covariant_result(function, adjusted, through)?
        else {
            self.vtables.return_adjustments.insert(key, None);
            return Ok(None);
        };
        let route = if let Some(through) = classes.through {
            let first = self.base_route(classes.from, through)?;
            let second = self.base_route(through, classes.to)?;
            if second.via.is_none() {
                super::virtual_bases::Route {
                    via: first.via,
                    offset: first.offset.wrapping_add(second.offset),
                }
            } else {
                self.base_route(classes.from, classes.to)?
            }
        } else {
            self.base_route(classes.from, classes.to)?
        };
        let adjustment = (route.via.is_some() || route.offset != 0).then_some(ResultAdjustment {
            from: classes.from,
            via: route.via,
            offset: i64::from_ne_bytes(route.offset.to_ne_bytes()),
            reference: classes.reference,
        });
        self.vtables.return_adjustments.insert(key, adjustment);
        Ok(adjustment)
    }
    pub(super) fn segment_offsets(&mut self, class: EntityId) -> Result<&[u64], Error> {
        if !self.vtables.offsets.contains_key(&class) {
            self.vtables
                .offsets
                .insert(class, self.graph.lower_segment_offsets(class)?);
        }
        Ok(&self.vtables.offsets[&class])
    }
    fn class_presentation(&mut self, class: EntityId) -> Result<String, Error> {
        let (_, _, _, scope) = self.graph.lower_virtual_class_identity(class)?;
        let mut components = Vec::new();
        if self.graph.lower_scope_class(scope) != Some(class) {
            components.push(self.specialized_class_label(class)?);
        }
        let mut current = scope;
        while let (Some(parent), name) = self.graph.lower_presentation_scope(current) {
            components.push(self.presentation_scope_label(current, name)?);
            current = parent;
        }
        components.reverse();
        Ok(components.join("__"))
    }
    fn define_views(
        &mut self,
        program: &mut Program,
        class: EntityId,
        primary: SymbolId,
        binding: Option<Binding>,
    ) -> Result<(), Error> {
        let graph = self.graph;
        let group = graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("missing vtable group"))?;
        self.segment_offsets(class)?;
        let mut views = vec![Some(primary)];
        for (index, segment) in group.segments.iter().enumerate().skip(1) {
            if segment.shared {
                views.push(None);
                continue;
            }
            let base = self.class_presentation(segment.class)?;
            let top = self.class_presentation(class)?;
            let spelling = format!(
                "{top}____view__{base}__{}__vtable",
                self.vtables.offsets[&class][index]
            );
            views.push(Some(self.table.allocate(program, &spelling, None, true)?));
        }
        self.vtables.views.insert(class, views);
        let typeinfo = self.typeinfo_symbol(program, class)?;
        for index in 1..group.segments.len() {
            let Some(symbol) = self.vtables.views[&class][index] else {
                continue;
            };
            let mut items = self.table_rows(class, index)?;
            items.extend([
                integer(
                    Type::I64,
                    self.vtables.offsets[&class][index].wrapping_neg(),
                ),
                address(typeinfo, 0),
            ]);
            self.append_view_targets(program, class, index, binding, &mut items)?;
            self.table.work.vtables += 1;
            define(
                program,
                symbol,
                Metadata {
                    binding: Some(Binding::Internal),
                    ..Metadata::default()
                },
                items,
            )?;
        }
        Ok(())
    }
    fn append_view_targets(
        &mut self,
        program: &mut Program,
        class: EntityId,
        segment: usize,
        binding: Option<Binding>,
        targets: &mut Vec<DataItem>,
    ) -> Result<(), Error> {
        let graph = self.graph;
        let group = graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("missing vtable group"))?;
        let part = &group.segments[segment];
        let own = graph.lower_virtual_slots(part.class)?;
        for (slot, entry) in group.entries[part.entries.clone()].iter().enumerate() {
            let adjustment = self.entry_adjustment(class, segment, slot)?;
            targets.push(
                self.table_target(
                    program,
                    TableTarget {
                        function: entry.function,
                        kind: entry.kind,
                        adjustment,
                        adjusted: own[slot]
                            .ok_or(crate::sema::Error::Invariant("unfilled view slot"))?
                            .adjusted,
                        through: part.class,
                    },
                    binding,
                )?,
            );
        }
        Ok(())
    }
    fn append_group_items(
        &mut self,
        program: &mut Program,
        class: EntityId,
        binding: Option<Binding>,
        items: &mut Vec<DataItem>,
    ) -> Result<(), Error> {
        let graph = self.graph;
        let group = graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("missing vtable group"))?;
        // Resolve every slot before publishing the one packed definition.
        // Target demand can publish other globals; no view symbol is allocated.
        let mut targets = Vec::new();
        for (index, part) in group.segments.iter().enumerate().skip(1) {
            if !part.shared {
                self.append_view_targets(program, class, index, binding, &mut targets)?;
            }
        }
        self.segment_offsets(class)?;
        let typeinfo = self.typeinfo_symbol(program, class)?;
        let mut targets = targets.into_iter();
        for (index, part) in group.segments.iter().enumerate().skip(1) {
            if part.shared {
                continue;
            }
            items.extend(self.table_rows(class, index)?);
            items.extend([
                integer(
                    Type::I64,
                    self.vtables.offsets[&class][index].wrapping_neg(),
                ),
                address(typeinfo, 0),
            ]);
            items.extend(targets.by_ref().take(part.entries.len()));
        }
        Ok(())
    }
    pub(in crate::lower) fn table_address(
        &mut self,
        program: &mut Program,
        class: EntityId,
        mut segment: usize,
    ) -> Result<(SymbolId, u64), Error> {
        let symbol = self.vtable_symbol(program, class)?;
        let Some(group) = self.graph.lower_vtable_group(class) else {
            return Ok((symbol, self.table_header_bytes(class, 0)?));
        };
        while segment != 0 && group.segments[segment].shared {
            segment = group.segments[segment].parent.index();
        }
        if segment == 0 {
            return Ok((symbol, self.table_header_bytes(class, 0)?));
        }
        if let Some(views) = self.vtables.views.get(&class) {
            return Ok((
                views[segment].ok_or(crate::sema::Error::Invariant("missing view symbol"))?,
                self.table_header_bytes(class, segment)?,
            ));
        }
        if !self.vtables.address_points.contains_key(&class) {
            let primary = self.table_header_bytes(class, 0)?;
            let mut points = vec![primary];
            let mut position =
                primary.wrapping_add(u64::from(self.graph.lower_virtual_class(class)?.size) * 8);
            for (index, part) in group.segments.iter().enumerate().skip(1) {
                if part.shared {
                    points.push(0);
                    continue;
                }
                let point = position.wrapping_add(self.table_header_bytes(class, index)?);
                points.push(point);
                position = point
                    .wrapping_add(u64::from(self.graph.lower_virtual_class(part.class)?.size) * 8);
            }
            self.vtables.address_points.insert(class, points);
        }
        Ok((symbol, self.vtables.address_points[&class][segment]))
    }
    fn thunk_symbol(
        &mut self,
        program: &mut Program,
        entry: Entry,
        adjustment: ThisAdjustment,
        result: Option<ResultAdjustment>,
        binding: Option<Binding>,
        strong: bool,
    ) -> Result<SymbolId, Error> {
        use crate::abi::encoder::CallOffset;
        let internal = binding == Some(Binding::Internal);
        let key = ThunkKey {
            entry,
            adjustment,
            result,
            internal,
        };
        if let Some(&index) = self.vtables.thunk_symbols.get(&key) {
            if strong && !internal {
                self.vtables.thunks[index].metadata.binding = Some(Binding::Strong);
            }
            return Ok(self.vtables.thunks[index].symbol);
        }
        let facts = self.entry_facts(entry)?;
        let target = self.entry_symbol(program, entry)?;
        let variant = if matches!(entry, Entry::DestructorDeleting(_)) {
            super::super::construction::Variant::Deleting
        } else {
            super::super::construction::Variant::Complete
        };
        let function = self.encode_function(&facts, variant)?;
        let result_row = result
            .and_then(|r| r.via.map(|via| (r.from, via)))
            .map(|(from, via)| self.vbase_row_offset(from, via))
            .transpose()?;
        let mut object = Vec::new();
        self.table.work.mangled_names += 1;
        Encoder::encode(
            &self.abi,
            Target::Thunk {
                function,
                this: CallOffset {
                    offset: adjustment.offset,
                    vcall: adjustment.vcall,
                },
                result: result.map(|r| CallOffset {
                    offset: r.offset,
                    vcall: result_row,
                }),
            },
            &mut object,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&object)?;
        let returned = result.map_or(0, |r| r.offset);
        let mut spelling = format!(
            "_{}__vtable_return_adjust__this_{}{}__return_{}{}",
            program.names.get(program.symbol(target).name),
            if adjustment.offset < 0 { "neg" } else { "pos" },
            adjustment.offset.unsigned_abs(),
            if returned < 0 { "neg" } else { "pos" },
            returned.unsigned_abs()
        );
        if let Some(vcall) = adjustment.vcall {
            use std::fmt::Write;
            write!(
                spelling,
                "__vcall_{}{}",
                if vcall < 0 { "n" } else { "p" },
                vcall.unsigned_abs()
            )
            .map_err(|_| crate::sema::Error::Invariant("thunk spelling"))?;
        }
        if let Some(row) = result_row {
            use std::fmt::Write;
            write!(
                spelling,
                "__virtual_{}{}",
                if row < 0 { "n" } else { "p" },
                row.unsigned_abs()
            )
            .map_err(|_| crate::sema::Error::Invariant("thunk result spelling"))?;
        }
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), internal)?;
        let entity = entry
            .entity(self.graph)
            .ok_or(crate::sema::Error::Invariant("thunk without function"))?;
        let original = self.virtual_signature(program, entity)?;
        let original = program.signature(original);
        let this_index = usize::from(
            original
                .parameters
                .first()
                .is_some_and(|p| p.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)),
        );
        let parameters = original
            .parameters
            .iter()
            .enumerate()
            .map(|(index, p)| crate::lowir::model::Parameter {
                name: p.name,
                ty: p.ty,
                pass: p.pass,
                object_bytes: if index == this_index {
                    0
                } else {
                    p.object_bytes
                },
                no_alias: p.no_alias,
                classes: p.classes,
            })
            .collect();
        let signature = crate::lowir::model::Signature {
            parameters,
            result: original.result,
            boundary: crate::lowir::model::Boundary {
                arity: original.boundary.arity,
                effects: original.boundary.effects,
                returns: original.boundary.returns,
                result_classes: original.boundary.result_classes,
                unwind: if matches!(entry, Entry::DestructorDeleting(_)) {
                    Unwind::No
                } else {
                    original.boundary.unwind
                },
            },
        };
        let variadic = signature.boundary.arity == crate::lowir::model::Arity::Variadic;
        if self.table.object_mode && variadic && result.is_some() {
            return Err(crate::sema::Error::Diagnostic {
                location: crate::support::source::SourceOffset(0),
                message: "a covariant thunk of a variadic function",
            }
            .into());
        }
        let jump =
            self.table.object_mode && (variadic || self.table.host_objects && result.is_none());
        let signature = program.add_signature(signature)?;
        self.vtables
            .thunk_symbols
            .insert(key, self.vtables.thunks.len());
        let family = if matches!(
            facts.name,
            crate::sema::lowering::FunctionName::Destructor(_)
        ) {
            let object = if internal {
                None
            } else {
                let function =
                    self.encode_function(&facts, crate::lower::construction::Variant::Base)?;
                let mut object = Vec::new();
                // Object-family aliases are backend observations. The C++
                // lowering neither mangles nor interns these extra aliases.
                Encoder::encode(
                    &self.abi,
                    Target::Thunk {
                        function,
                        this: CallOffset {
                            offset: adjustment.offset,
                            vcall: adjustment.vcall,
                        },
                        result: None,
                    },
                    &mut object,
                    self.budget,
                )?;
                Some(program.names.intern_view_bytes(&object)?)
            };
            Some((entity, object, variant))
        } else {
            None
        };
        self.vtables.thunks.push(Thunk {
            symbol,
            target,
            signature,
            this_index,
            adjustment,
            result,
            result_row,
            jump,
            family,
            metadata: Metadata {
                object: Some(object),
                binding: Some(if internal {
                    Binding::Internal
                } else if strong {
                    Binding::Strong
                } else {
                    Binding::Weak
                }),
                ..Metadata::default()
            },
        });
        Ok(symbol)
    }
    pub(in crate::lower) fn emit_thunks(
        &mut self,
        program: &mut Program,
        definitions: &mut crate::lower::definition_order::Definitions,
    ) -> Result<(), Error> {
        use crate::lowir::{
            builder::FunctionBuilder,
            model::{Boundary, Call, InstructionKind as I, Value},
        };
        for thunk in &self.vtables.thunks[self.vtables.pending_thunk..] {
            if program.function_defined(thunk.symbol) {
                continue;
            }
            if !program.function_defined(thunk.target) {
                program.add(
                    thunk.symbol,
                    crate::lowir::model::Function {
                        name: program.symbol(thunk.symbol).name,
                        signature: thunk.signature,
                        metadata: Metadata {
                            binding: Some(if thunk.metadata.binding == Some(Binding::Internal) {
                                Binding::Internal
                            } else {
                                Binding::Strong
                            }),
                            ..thunk.metadata
                        },
                        body: None,
                        location: None,
                    },
                )?;
                continue;
            }
            if thunk.jump {
                program.set_jump_thunk(
                    thunk.symbol,
                    crate::lowir::model::JumpThunk {
                        target: thunk.target,
                        this_index: u32::try_from(thunk.this_index)
                            .map_err(|_| crate::lowir::Error::Limit)?,
                        adjustment: thunk.adjustment.offset,
                        vcall_offset: thunk.adjustment.vcall.unwrap_or(0),
                    },
                );
                if program.sink_reads_jump_thunks() {
                    program.add(
                        thunk.symbol,
                        crate::lowir::model::Function {
                            name: program.symbol(thunk.symbol).name,
                            signature: thunk.signature,
                            metadata: thunk.metadata,
                            body: None,
                            location: None,
                        },
                    )?;
                    definitions.stream(program, None)?;
                    continue;
                }
            }
            let signature = program.signature(thunk.signature);
            let params: Vec<_> = signature
                .parameters
                .iter()
                .map(|p| {
                    (
                        program.names.get(p.name).to_owned(),
                        p.ty,
                        p.pass,
                        p.object_bytes,
                        p.no_alias,
                        p.classes,
                    )
                })
                .collect();
            let parameters: Vec<_> = params.iter().map(|p| (p.0.as_str(), p.1)).collect();
            let result = signature.result;
            let boundary = Boundary {
                arity: signature.boundary.arity,
                effects: signature.boundary.effects,
                unwind: signature.boundary.unwind,
                returns: signature.boundary.returns,
                result_classes: signature.boundary.result_classes,
            };
            let name = program
                .names
                .get(program.symbol(thunk.symbol).name)
                .to_owned();
            let mut builder = FunctionBuilder::new(program, &name, &parameters, result)?;
            builder.metadata(thunk.metadata);
            builder.boundary(boundary);
            for (index, p) in params.iter().enumerate() {
                builder.parameter_boundary(index, p.2, p.3, p.4)?;
                builder.parameter_classes(index, p.5)?;
            }
            builder.block("entry")?;
            let mut arguments = Vec::with_capacity(params.len());
            for index in 0..params.len() {
                let mut argument = builder.parameter(index)?;
                if index == thunk.this_index {
                    if thunk.adjustment.offset != 0 || thunk.adjustment.vcall.is_none() {
                        let offset = builder.integer(u128::from_ne_bytes(
                            i128::from(thunk.adjustment.offset).to_ne_bytes(),
                        ))?;
                        argument = builder.generated(I::Index(Type::I8, None, argument, offset))?;
                    }
                    if let Some(vcall) = thunk.adjustment.vcall {
                        let facts = crate::lowir::model::Facts::memory(false, true);
                        let table = builder.generated(I::Load(facts, Type::Ptr, argument))?;
                        let offset = builder
                            .integer(u128::from_ne_bytes(i128::from(vcall).to_ne_bytes()))?;
                        let row = builder.generated(I::Index(Type::I8, None, table, offset))?;
                        let offset = builder.generated(I::Load(facts, Type::I64, row))?;
                        argument = builder.generated(I::Index(Type::I8, None, argument, offset))?;
                    }
                }
                arguments.push(argument);
            }
            let arguments = builder.arguments(&arguments)?;
            let call = I::Call(Call {
                result,
                callee: Value::Symbol(thunk.target),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: Vec::new(),
            });
            let returned = if result == Type::Void {
                builder.effect(call)?;
                None
            } else {
                Some(builder.generated(call)?)
            };
            let returned = if let Some(adjustment) = thunk.result {
                let mut value =
                    returned.ok_or(crate::sema::Error::Invariant("void covariant thunk"))?;
                if !adjustment.reference {
                    let null_block = builder.generated_block("return_null_1", "return_null")?;
                    let adjust_block =
                        builder.generated_block("adjust_return_2", "adjust_return")?;
                    let zero = builder.integer(0)?;
                    let is_null = builder.generated(I::Compare(
                        crate::lowir::vocabulary::Compare::Eq,
                        Type::Ptr,
                        value,
                        zero,
                    ))?;
                    builder.effect(I::Branch(is_null, null_block, adjust_block))?;
                    builder.place_block(null_block)?;
                    builder.effect(I::Return(result, Some(value)))?;
                    builder.place_block(adjust_block)?;
                }
                if let Some(row) = thunk.result_row {
                    let facts = crate::lowir::model::Facts::memory(false, true);
                    let table = builder.generated(I::Load(facts, Type::Ptr, value))?;
                    let offset =
                        builder.integer(u128::from_ne_bytes(i128::from(row).to_ne_bytes()))?;
                    let row = builder.generated(I::Index(Type::I8, None, table, offset))?;
                    let offset = builder.generated(I::Load(facts, Type::I64, row))?;
                    value = builder.generated(I::Index(Type::I8, None, value, offset))?;
                }
                if adjustment.offset != 0 {
                    let offset = builder.integer(u128::from_ne_bytes(
                        i128::from(adjustment.offset).to_ne_bytes(),
                    ))?;
                    value = builder.generated(I::Index(Type::I8, None, value, offset))?;
                }
                Some(value)
            } else {
                returned
            };
            builder.effect(I::Return(result, returned))?;
            builder.finish()?;
            let family = if let Some((entity, object, variant)) =
                thunk.family.filter(|_| !self.table.object_mode)
            {
                let family = if let Some(&family) =
                    self.vtables.thunk_families.get(&(entity, thunk.adjustment))
                {
                    family
                } else {
                    let family = definitions.family(object)?;
                    self.vtables
                        .thunk_families
                        .insert((entity, thunk.adjustment), family);
                    family
                };
                Some((family, variant))
            } else {
                None
            };
            definitions.stream(program, family)?;
        }
        self.vtables.pending_thunk = self.vtables.thunks.len();
        Ok(())
    }
    pub(super) fn rtti_vtable(
        &mut self,
        program: &mut Program,
        kind: InformationKind,
    ) -> Result<SymbolId, Error> {
        use InformationKind as K;
        let (name, object, role) = match kind {
            K::Class => (
                "__class_type_info",
                "_ZTVN10__cxxabiv117__class_type_infoE",
                Role::RttiClass,
            ),
            K::SingleInheritance => (
                "__si_class_type_info",
                "_ZTVN10__cxxabiv120__si_class_type_infoE",
                Role::RttiSi,
            ),
            K::MultipleInheritance => (
                "__vmi_class_type_info",
                "_ZTVN10__cxxabiv121__vmi_class_type_infoE",
                Role::RttiVmi,
            ),
            K::Fundamental => (
                "__fundamental_type_info",
                "_ZTVN10__cxxabiv123__fundamental_type_infoE",
                Role::RttiData,
            ),
            K::Pointer => (
                "__pointer_type_info",
                "_ZTVN10__cxxabiv119__pointer_type_infoE",
                Role::RttiPointer,
            ),
            K::MemberPointer => (
                "__pointer_to_member_type_info",
                "_ZTVN10__cxxabiv129__pointer_to_member_type_infoE",
                Role::RttiData,
            ),
            K::Function => (
                "__function_type_info",
                "_ZTVN10__cxxabiv120__function_type_infoE",
                Role::RttiData,
            ),
            K::Array => (
                "__array_type_info",
                "_ZTVN10__cxxabiv117__array_type_infoE",
                Role::RttiData,
            ),
            K::Enum => (
                "__enum_type_info",
                "_ZTVN10__cxxabiv116__enum_type_infoE",
                Role::RttiData,
            ),
        };
        let object = program.names.intern(object)?;
        let symbol = self.table.allocate(
            program,
            &format!("__external_rtti_vtable__{name}"),
            Some(object),
            false,
        )?;
        if !program.symbol(symbol).declared() {
            program.add_global(
                symbol,
                Global {
                    name: program.symbol(symbol).name,
                    ty: None,
                    metadata: Metadata {
                        binding: Some(Binding::Strong),
                        object: Some(object),
                        role: Some(role),
                        ..Metadata::default()
                    },
                    init: GlobalInit::Declaration,
                },
            )?;
        }
        Ok(symbol)
    }
    fn typeinfo_name(&mut self, program: &mut Program, class: EntityId) -> Result<SymbolId, Error> {
        if let Some(&symbol) = self.vtables.names.get(&class) {
            return Ok(symbol);
        }
        let (symbol, metadata) = self.class_global(program, class, ClassObject::Name)?;
        self.vtables.names.insert(class, symbol);
        if !defined(program, symbol) {
            let (mut name, _) = self.class_encoding(class, ClassObject::Name)?;
            if self.table.object_mode
                && self
                    .graph
                    .lower_private_type_name(self.graph.lower_entity_type(class)?)?
            {
                name.insert(0, '*');
            }
            let items = name
                .bytes()
                .chain(std::iter::once(0))
                .map(|byte| integer(Type::I8, u64::from(byte)))
                .collect();
            define(program, symbol, metadata, items)?;
        }
        Ok(symbol)
    }
    pub(in crate::lower) fn typeinfo_symbol(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<SymbolId, Error> {
        self.class_typeinfo(program, class, false)
    }
    fn class_typeinfo(
        &mut self,
        program: &mut Program,
        class: EntityId,
        derived: bool,
    ) -> Result<SymbolId, Error> {
        self.check_stack()?;
        let (symbol, metadata) = if let Some(&known) = self.vtables.typeinfos.get(&class) {
            known
        } else {
            let result = self.class_global(program, class, ClassObject::Typeinfo)?;
            self.vtables.typeinfos.insert(class, result);
            result
        };
        // A primary class object's information follows its vtable owner;
        // a derived descriptor may also define an otherwise external base.
        if defined(program, symbol)
            || self.graph.lower_polymorphic(class)
                && !(derived && !self.graph.lower_class_instantiated_elsewhere(class)
                    || self.graph.lower_emits_vtable(class)?)
        {
            return Ok(symbol);
        }
        let bases = self
            .graph
            .lower_rtti_bases(class)
            .collect::<Result<Vec<_>, _>>()?;
        let single = matches!(bases.as_slice(), [(_, true, false, 0)]);
        let kind = self.rtti_vtable(
            program,
            if bases.is_empty() {
                InformationKind::Class
            } else if single {
                InformationKind::SingleInheritance
            } else {
                InformationKind::MultipleInheritance
            },
        )?;
        let name = self.typeinfo_name(program, class)?;
        let mut targets = Vec::with_capacity(bases.len());
        for &(base, _, _, _) in &bases {
            targets.push(self.class_typeinfo(program, base, true)?);
        }
        let mut items = vec![address(kind, 16), address(name, 0)];
        if single {
            items.push(address(targets[0], 0));
        } else if !bases.is_empty() {
            items.push(integer(Type::I32, u64::from(self.inheritance_flags(class))));
            items.push(integer(
                Type::I32,
                u64::try_from(bases.len()).map_err(|_| crate::lowir::Error::Limit)?,
            ));
            for (&(base, public, is_virtual, offset), target) in bases.iter().zip(targets) {
                let offset = if is_virtual {
                    u64::from_ne_bytes(self.vbase_row_offset(class, base)?.to_ne_bytes())
                } else {
                    offset
                };
                items.push(address(target, 0));
                items.push(integer(
                    Type::I64,
                    offset.wrapping_shl(8) | if public { 2 } else { 0 } | u64::from(is_virtual),
                ));
            }
        }
        define(program, symbol, metadata, items)?;
        Ok(symbol)
    }
    fn inheritance_flags(&self, class: EntityId) -> u32 {
        // vtable_lowering.cpp::inheritance_flags: count actual nonvirtual
        // occurrences, but descend through each shared virtual base only once.
        // The bounded walk's repeat flag disables the runtime's shortcuts.
        let mut work = vec![class];
        let mut marks = HashMap::<EntityId, u8>::new();
        let mut flags = 0;
        let mut visits = 0_u32;
        while let Some(class) = work.pop() {
            for (next, shared) in self.graph.lower_inheritance_edges(class) {
                visits += 1;
                if visits > 1 << 16 {
                    return 1;
                }
                let mark = marks.entry(next).or_default();
                if shared {
                    flags |= u32::from(*mark & 1 != 0);
                    if *mark & 2 != 0 {
                        flags |= 2;
                        continue;
                    }
                    *mark |= 2;
                } else {
                    flags |= u32::from(*mark != 0);
                    *mark |= 1;
                }
                work.push(next);
            }
        }
        flags
    }
    pub(in crate::lower) fn virtual_signature(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SignatureId, Error> {
        if let Some(&signature) = self.vtables.signatures.get(&entity) {
            return Ok(signature);
        }
        let facts = self.function_facts(entity)?;
        let signature = function::signature(program, self, self.graph, facts.ty, facts.nothrow)?;
        function::declared_effects(
            &mut program.signature_mut(signature).boundary,
            self.graph,
            Some(entity),
        );
        let class = facts
            .object
            .ok_or(crate::sema::Error::Invariant("virtual without class"))?;
        let first = usize::from(
            program
                .signature(signature)
                .parameters
                .first()
                .is_some_and(|p| p.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)),
        );
        let name = program.names.intern("arg0")?;
        program.signature_mut(signature).parameters.insert(
            first,
            crate::lowir::model::Parameter {
                name,
                ty: Type::Ptr,
                pass: None,
                object_bytes: types::layout(self.graph, self.graph.lower_entity_type(class)?)?.0,
                no_alias: false,
                classes: None,
            },
        );
        for index in 0..program.signature(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{index}"))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
        self.vtables.signatures.insert(entity, signature);
        Ok(signature)
    }
    fn pure_virtual_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        if let Some(symbol) = self.vtables.pure_virtual {
            return Ok(symbol);
        }
        let symbol = self
            .table
            .allocate(program, "__cxa_pure_virtual", None, true)?;
        let signature = self.virtual_signature(program, entity)?;
        // A handler has its own boundary, never mutate a call site's signature.
        let original = program.signature(signature);
        let mut parameters = Vec::with_capacity(original.parameters.len());
        for p in &original.parameters {
            parameters.push(crate::lowir::model::Parameter {
                name: p.name,
                ty: p.ty,
                pass: p.pass,
                object_bytes: p.object_bytes,
                no_alias: p.no_alias,
                classes: p.classes,
            });
        }
        let boundary = crate::lowir::model::Boundary {
            unwind: Unwind::No,
            returns: Returns::NoReturn,
            arity: original.boundary.arity,
            effects: original.boundary.effects,
            result_classes: original.boundary.result_classes,
        };
        let result = original.result;
        let signature = program.add_signature(crate::lowir::model::Signature {
            parameters,
            result,
            boundary,
        })?;
        program.add(
            symbol,
            crate::lowir::model::Function {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    role: Some(Role::PureVirtual),
                    binding: Some(Binding::Strong),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        program.publish_runtime_declaration(symbol);
        self.vtables.pure_virtual = Some(symbol);
        Ok(symbol)
    }
    fn prepare_definition_thunks(&mut self, class: EntityId) -> Result<(), Error> {
        if self.vtables.definition_classes.contains(&class) {
            return Ok(());
        }
        if !self.vtables.definitions_indexed {
            self.vtables.definitions_indexed = true;
            // Completion order puts each base before its derived classes.
            for &grouped in self.graph.lower_grouped_classes() {
                let group = self
                    .graph
                    .lower_vtable_group(grouped)
                    .ok_or(crate::sema::Error::Invariant("missing completed group"))?;
                let sensitive =
                    group.segments.iter().enumerate().skip(1).any(|(i, s)| {
                        s.shared && s.within.is_some_and(|within| within.index() == i)
                    }) || self
                        .graph
                        .lower_inheritance_edges(grouped)
                        .any(|(base, _)| self.vtables.construction_sensitive.contains(&base))
                        || self
                            .graph
                            .lower_virtual_bases(grouped)
                            .iter()
                            .any(|(base, _)| self.vtables.construction_sensitive.contains(base));
                if sensitive {
                    self.vtables.construction_sensitive.insert(grouped);
                }
            }
            for &grouped in self.graph.lower_grouped_classes() {
                if self.vtables.definition_classes.insert(grouped) {
                    self.index_definition_thunks(grouped)?;
                }
            }
        }
        if self.vtables.definition_classes.insert(class) {
            self.index_definition_thunks(class)?;
        }
        Ok(())
    }
    fn index_construction_thunks(&mut self, context: TableContext, top: bool) -> Result<(), Error> {
        self.check_stack()?;
        if !self.vtables.construction_sensitive.contains(&context.class) {
            return Ok(());
        }
        let graph = self.graph;
        if !top {
            let group =
                graph
                    .lower_vtable_group(context.class)
                    .ok_or(crate::sema::Error::Invariant(
                        "construction thunk without group",
                    ))?;
            for segment in std::iter::once(0).chain(self.vtt_pointer_order(context.class)?) {
                let segment = self.construction_owner(context, segment)?;
                let part = &group.segments[segment];
                for (slot, own) in graph.lower_virtual_slots(part.class)?.iter().enumerate() {
                    let own = own.ok_or(crate::sema::Error::Invariant(
                        "unfilled construction thunk slot",
                    ))?;
                    let entry = group.entries[part.entries.start + slot];
                    let adjustment = self.construction_adjustment(context, segment, slot)?;
                    if !graph.lower_virtual_pure(entry.function) {
                        self.note_definition_target(TableTarget {
                            function: entry.function,
                            kind: entry.kind,
                            adjustment,
                            adjusted: own.adjusted,
                            through: part.class,
                        })?;
                    }
                }
            }
        }
        for (i, (base, shared)) in graph.lower_inheritance_edges(context.class).enumerate() {
            if !shared && !graph.lower_virtual_bases(base).is_empty() {
                let (_, _, offset) = graph.lower_inheritance_edge(context.class, i).ok_or(
                    crate::sema::Error::Invariant("missing construction thunk base"),
                )?;
                self.index_construction_thunks(
                    TableContext {
                        complete: context.complete,
                        class: base,
                        offset: context.offset.wrapping_add(offset.ok_or(
                            crate::sema::Error::Invariant("construction thunk before layout"),
                        )?),
                        construction: true,
                    },
                    false,
                )?;
            }
        }
        if top {
            for &(base, offset) in graph.lower_virtual_bases(context.complete) {
                if !graph.lower_virtual_bases(base).is_empty() {
                    self.index_construction_thunks(
                        TableContext {
                            complete: context.complete,
                            class: base,
                            offset,
                            construction: true,
                        },
                        false,
                    )?;
                }
            }
        }
        Ok(())
    }
    fn index_definition_thunks(&mut self, class: EntityId) -> Result<(), Error> {
        if !self.graph.lower_virtual_bases(class).is_empty() {
            self.index_construction_thunks(TableContext::complete(class), true)?;
        }
        let graph = self.graph;
        let group = graph.lower_vtable_group(class);
        let count = group.map_or(1, |group| group.segments.len());
        for index in 0..count {
            if group.is_some_and(|g| g.segments[index].shared) {
                continue;
            }
            let through = group.map_or(class, |g| g.segments[index].class);
            let own = graph.lower_virtual_slots(through)?;
            for (slot, entry) in own.iter().enumerate() {
                let own = entry.ok_or(crate::sema::Error::Invariant("unfilled definition slot"))?;
                let (function, kind) = if let Some(group) = group {
                    let entry = group.entries[group.segments[index].entries.start + slot];
                    (entry.function, entry.kind)
                } else {
                    (own.function, own.kind)
                };
                if graph.lower_virtual_pure(function) {
                    continue;
                }
                let adjustment = self.entry_adjustment(class, index, slot)?;
                self.note_definition_target(TableTarget {
                    function,
                    kind,
                    adjustment,
                    adjusted: own.adjusted,
                    through,
                })?;
                if let Some(level) = self.virtual_thunk_level(function, through, slot)? {
                    let adjustment = ThisAdjustment {
                        offset: 0,
                        vcall: Some(self.vcall_row_offset(level, function)?),
                        unused: false,
                    };
                    self.note_definition_target(TableTarget {
                        function,
                        kind,
                        adjustment,
                        adjusted: own.adjusted,
                        through,
                    })?;
                }
            }
        }
        Ok(())
    }
    fn note_definition_target(&mut self, target: TableTarget) -> Result<(), Error> {
        let result = self.result_adjustment(
            target.function,
            target.kind,
            target.adjusted,
            target.through,
        )?;
        if target.adjustment.is_zero() && result.is_none() {
            return Ok(());
        }
        self.vtables.definition_targets.note(target);
        Ok(())
    }
    fn virtual_thunk_level(
        &self,
        function: EntityId,
        table: EntityId,
        slot: usize,
    ) -> Result<Option<EntityId>, Error> {
        let owner = self.function_facts(function)?.object;
        let mut below = true;
        let mut link = Some(table);
        while let Some(class) = link {
            if Some(class) == owner {
                below = false;
                break;
            }
            link = self.graph.lower_virtual_class(class)?.primary;
        }
        let mut level = None;
        link = Some(table);
        while let Some(class) = link {
            let facts = self.graph.lower_virtual_class(class)?;
            if slot
                >= usize::try_from(facts.size)
                    .map_err(|_| crate::sema::Error::Limit("virtual slot limit"))?
            {
                break;
            }
            below |= Some(class) == owner;
            let Some((next, shared)) = self.graph.lower_itanium_primary(class) else {
                break;
            };
            if below
                && shared
                && slot
                    < usize::try_from(self.graph.lower_virtual_class(next)?.size)
                        .map_err(|_| crate::sema::Error::Limit("virtual slot limit"))?
            {
                level = Some(next);
            }
            link = Some(next);
        }
        Ok(level)
    }
    pub(in crate::lower) fn key_function_table(
        &mut self,
        program: &mut Program,
        entry: Entry,
    ) -> Result<(), Error> {
        if let Entry::Function(entity) = entry
            && self.graph.lower_virtual_slot(entity).is_some()
            && !self.function_facts(entity)?.internal
            && !self.graph.lower_virtual_pure(entity)
            && let Some(class) = self.function_facts(entity)?.object
        {
            self.prepare_definition_thunks(class)?;
            let symbol = self.entry_symbol(program, entry)?;
            let binding = program
                .function(program.symbol(symbol))
                .ok_or(crate::sema::Error::Invariant(
                    "virtual definition missing function",
                ))?
                .metadata
                .binding;
            let mut next = self.vtables.definition_targets.heads.get(&entity).copied();
            while let Some(index) = next {
                let node = &self.vtables.definition_targets.nodes[index];
                let target = node.target;
                next = node.next;
                self.table_entry_symbol(
                    program,
                    target,
                    binding,
                    binding == Some(Binding::Strong),
                )?;
            }
        }
        if let Entry::Function(entity) = entry
            && let Some(class) = self.graph.lower_virtual_key_owner(entity)
        {
            self.vtable_symbol(program, class)?;
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) fn assert_definition_destructor_targets(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    function: EntityId,
) {
    let through = graph.lower_function(function).unwrap().object.unwrap();
    let mut table = crate::lower::symbols::SymbolTable {
        object_mode: true,
        host_objects: true,
        ..crate::lower::symbols::SymbolTable::default()
    };
    let mut symbols = Symbols::new(
        graph,
        tree,
        cursor,
        crate::support::stack::StackBudget::default(),
        &mut table,
    );
    let mut program = Program::default();
    let target = TableTarget {
        function,
        kind: SlotKind::CompleteDestructor,
        adjustment: ThisAdjustment {
            vcall: Some(-24),
            ..ThisAdjustment::default()
        },
        adjusted: function,
        through,
    };
    let unused = TableTarget {
        adjustment: ThisAdjustment {
            unused: true,
            ..target.adjustment
        },
        ..target
    };
    assert!(
        symbols
            .table_entry_symbol(&mut program, unused, Some(Binding::Weak), false)
            .unwrap()
            .is_none()
    );
    assert_eq!(symbols.pending.len(), 0);
    assert!(program.symbols().is_empty());

    let first = symbols
        .table_entry_symbol(&mut program, target, Some(Binding::Weak), false)
        .unwrap();
    assert!(first.is_some());
    assert!(matches!(
        symbols.pending.next(),
        Some(Entry::DestructorDeleting(entity)) if entity == function
    ));
    assert!(matches!(
        symbols.pending.next(),
        Some(Entry::Function(entity)) if entity == function
    ));
    assert!(symbols.pending.next().is_none());
    assert_eq!(
        first,
        symbols
            .table_entry_symbol(&mut program, target, Some(Binding::Strong), true)
            .unwrap()
    );
    assert!(symbols.pending.next().is_none());
    assert_eq!(symbols.vtables.thunks.len(), 1);
    assert_eq!(
        symbols.vtables.thunks[0].metadata.binding,
        Some(Binding::Strong)
    );
    assert!(symbols.vtables.objects.is_empty());
    assert!(symbols.vtables.typeinfos.is_empty());
    assert!(program.globals().is_empty());
}

#[cfg(test)]
pub(crate) fn assert_table_demand(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    classes: &[EntityId],
) {
    let (a, b, unused) = (classes[0], classes[1], classes[2]);
    assert_eq!(graph.lower_virtual_class(a).unwrap().size, 2);
    assert_eq!(graph.lower_virtual_class(b).unwrap().size, 2);
    assert_eq!(graph.lower_virtual_class(b).unwrap().entries.len(), 1);
    assert_eq!(
        graph.lower_virtual_class(b).unwrap().entries[0]
            .slot
            .index()
            .unwrap(),
        1
    );
    assert!(graph.lower_class_layout(unused).is_err());
    assert!(graph.lower_virtual_slots(unused).is_err());
    let slots = graph.lower_virtual_slots(b).unwrap();
    assert_eq!(slots.len(), 2);
    assert_eq!(
        slots[0].unwrap().function,
        graph.lower_virtual_class(a).unwrap().entries[0].function
    );
    assert_eq!(
        slots[1].unwrap().function,
        graph.lower_virtual_class(b).unwrap().entries[0].function
    );
    let mut program = Program::default();
    let mut table = super::SymbolTable::default();
    let mut symbols = Symbols::new(
        graph,
        tree,
        cursor,
        crate::support::stack::StackBudget::default(),
        &mut table,
    );
    let first = symbols.vtable_symbol(&mut program, b).unwrap();
    let counts = (
        program.globals().len(),
        program.symbols().len(),
        symbols.pending.len(),
    );
    assert_eq!(symbols.vtable_symbol(&mut program, b).unwrap(), first);
    assert_eq!(
        counts,
        (
            program.globals().len(),
            program.symbols().len(),
            symbols.pending.len()
        )
    );
    assert!(!symbols.vtables.objects.contains_key(&a));
    assert!(!symbols.vtables.objects.contains_key(&unused));
    assert!(!symbols.vtables.typeinfos.contains_key(&unused));
    symbols.vtable_symbol(&mut program, a).unwrap();
    assert!(symbols.vtables.pure_virtual.is_some());
}

#[cfg(test)]
pub(crate) fn assert_hierarchy_flags(
    graph: &crate::sema::Graph,
    tree: &crate::syntax::ast::Tree,
    cursor: &crate::syntax::cursor::Cursor<'_>,
    parameters: &[crate::sema::lowering::TypeId],
) {
    use crate::{sema::lowering::TypeView, support::stack::StackBudget};
    let program = Program::default();
    let mut table = super::SymbolTable::default();
    let symbols = Symbols::new(graph, tree, cursor, StackBudget::default(), &mut table);
    assert_eq!(parameters.len(), 6);
    for (((&ty, expected), slots), layout) in parameters
        .iter()
        .zip([0, 1, 2, 3, 3, 0])
        .zip([true, true, true, true, true, false])
        .zip([true, false, true, true, true, false])
    {
        let TypeView::Pointer(pointee) = graph.lower_type(ty).unwrap() else {
            panic!("anchor pointer");
        };
        let TypeView::Class(class) = graph.lower_type(pointee).unwrap() else {
            panic!("anchor class");
        };
        assert_eq!(symbols.inheritance_flags(class), expected);
        assert_eq!(graph.lower_class_layout(class).is_ok(), layout);
        // Class completion projects virtual primary pointers where they may
        // share. This readonly hierarchy query adds no layout/slot demand.
        assert_eq!(graph.lower_virtual_slots(class).is_ok(), slots);
    }
    assert_eq!(symbols.pending.len(), 0);
    assert!(symbols.vtables.objects.is_empty());
    assert!(symbols.vtables.typeinfos.is_empty());
    assert!(program.globals().is_empty());
}

impl Symbols<'_, '_> {
    fn table_header_bytes(&mut self, class: EntityId, segment: usize) -> Result<u64, Error> {
        let rows = u64::try_from(self.table_row_count(class, segment)?)
            .map_err(|_| crate::sema::Error::Limit("vtable row limit"))?;
        rows.checked_mul(8)
            .and_then(|bytes| bytes.checked_add(16))
            .ok_or_else(|| crate::sema::Error::Limit("vtable row limit").into())
    }
    pub(in crate::lower) fn vtt_symbol(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<SymbolId, Error> {
        if let Some(&symbol) = self.vtables.vtts.get(&class) {
            return Ok(symbol);
        }
        // Preserve the oracle's demand order: a first VTT request publishes
        // its class table, which can recursively publish a VTT of its own.
        // External ABI identity coalesces the two requests; internal requests
        // each allocate a definition, and the outer one becomes the memo.
        self.vtable_symbol(program, class)?;
        let (symbol, mut metadata) = self.class_global(program, class, ClassObject::Vtt)?;
        self.vtables.vtts.insert(class, symbol);
        if defined(program, symbol) || !self.graph.lower_emits_vtable(class)? {
            return Ok(symbol);
        }
        let items = self.vtt_entries(program, class)?;
        metadata.flags |= 4;
        define(program, symbol, metadata, items)?;
        Ok(symbol)
    }
}

impl Symbols<'_, '_> {
    fn entry_adjustment(
        &mut self,
        class: EntityId,
        segment: usize,
        slot: usize,
    ) -> Result<ThisAdjustment, Error> {
        self.context_adjustment(TableContext::complete(class), segment, slot)
    }
    fn context_adjustment(
        &mut self,
        context: TableContext,
        segment: usize,
        slot: usize,
    ) -> Result<ThisAdjustment, Error> {
        self.check_stack()?;
        let class = context.class;
        let Some(group) = self.graph.lower_vtable_group(class) else {
            return Ok(ThisAdjustment::default());
        };
        let part = &group.segments[segment];
        let entry = group.entries[part.entries.start + slot];
        let overrider_at = self.context_at(context, entry.overrider.index())?;
        let own = self.graph.lower_virtual_slots(part.class)?[slot]
            .ok_or(crate::sema::Error::Invariant("unfilled adjustment slot"))?;
        let (anchor, unused) = self.slot_anchor(context, segment, slot)?;
        if unused {
            return Ok(ThisAdjustment {
                unused: true,
                ..ThisAdjustment::default()
            });
        }
        let (place, within) = if let Some(anchor) = anchor {
            let place =
                self.group_segment(class, self.virtual_base_offset(class, anchor)?, anchor)?;
            (place, Some(place))
        } else {
            (
                segment,
                part.within.map(crate::sema::lowering::SegmentId::index),
            )
        };
        let level_at = self.context_at(context, place)?;
        if level_at == overrider_at
            && !self.graph.lower_virtual_pure(entry.function)
            && self
                .result_adjustment(entry.function, entry.kind, own.adjusted, part.class)?
                .is_none()
        {
            return Ok(ThisAdjustment::default());
        }
        if let Some(within) = within
            && group.segments[entry.overrider.index()]
                .within
                .map(crate::sema::lowering::SegmentId::index)
                != Some(within)
        {
            return Ok(ThisAdjustment {
                offset: i64::from_ne_bytes(
                    self.context_at(context, within)?
                        .wrapping_sub(level_at)
                        .to_ne_bytes(),
                ),
                vcall: Some(self.vcall_row_offset(group.segments[within].class, own.function)?),
                unused: false,
            });
        }
        Ok(ThisAdjustment::fixed(i64::from_ne_bytes(
            overrider_at.wrapping_sub(level_at).to_ne_bytes(),
        )))
    }
    fn slot_anchor(
        &mut self,
        context: TableContext,
        segment: usize,
        slot: usize,
    ) -> Result<(Option<EntityId>, bool), Error> {
        let class = context.class;
        let graph = self.graph;
        let group = graph
            .lower_vtable_group(class)
            .ok_or(crate::sema::Error::Invariant("anchor without group"))?;
        let part = &group.segments[segment];
        let entry = group.entries[part.entries.start + slot];
        let mut chain = Vec::new();
        let mut link = Some(part.class);
        while let Some(class) = link {
            let facts = graph.lower_virtual_class(class)?;
            if slot
                >= usize::try_from(facts.size)
                    .map_err(|_| crate::sema::Error::Limit("virtual slot limit"))?
            {
                break;
            }
            chain.push(class);
            link = facts.primary;
        }
        if chain.is_empty() {
            return Ok((None, false));
        }
        let mut lost = vec![false; chain.len()];
        let mut place = self.context_at(context, segment)?;
        for index in 1..chain.len() {
            if graph
                .lower_itanium_primary(chain[index - 1])
                .is_some_and(|(_, shared)| shared)
            {
                let at = self.virtual_base_offset(context.complete, chain[index])?;
                lost[index] = at != place;
                place = at;
            }
        }
        let mut parent = chain.len() - 1;
        let mut unused = false;
        for (index, &class) in chain.iter().enumerate() {
            let function = graph.lower_virtual_slots(class)?[slot]
                .ok_or(crate::sema::Error::Invariant("unfilled anchor slot"))?
                .function;
            if graph.lower_function(function)?.object == Some(class) {
                parent = index;
                break;
            }
            unused |= lost.get(index + 1).copied().unwrap_or(false);
        }
        let adjusted = graph.lower_virtual_slots(part.class)?[slot]
            .ok_or(crate::sema::Error::Invariant("unfilled result slot"))?
            .adjusted;
        if !graph.lower_virtual_pure(entry.function)
            && matches!(entry.kind, SlotKind::Function)
            && entry.function != adjusted
            && self
                .result_adjustment(entry.function, entry.kind, adjusted, part.class)?
                .is_some()
        {
            if graph.lower_function(entry.function)?.object == Some(chain[parent])
                && parent + 1 < chain.len()
            {
                parent += 1;
            }
            while parent + 1 < chain.len() && self.own_entry_thunk(chain[parent], slot)? {
                unused |= lost[parent + 1];
                parent += 1;
            }
        }
        let mut anchor = None;
        for index in 1..=parent {
            if graph
                .lower_itanium_primary(chain[index - 1])
                .is_some_and(|(_, shared)| shared)
            {
                anchor = Some(chain[index]);
            }
        }
        Ok((anchor, unused))
    }
    fn own_entry_thunk(&mut self, class: EntityId, slot: usize) -> Result<bool, Error> {
        if let Some(&known) = self.vtables.own_entry_thunks.get(&(class, slot)) {
            return Ok(known);
        }
        let own = self.graph.lower_virtual_slots(class)?[slot]
            .ok_or(crate::sema::Error::Invariant("unfilled own thunk slot"))?;
        let group = self.graph.lower_vtable_group(class);
        let (function, kind) = group.map_or((own.function, own.kind), |group| {
            let entry = group.entries[slot];
            (entry.function, entry.kind)
        });
        let result = !self.graph.lower_virtual_pure(function)
            && (self
                .result_adjustment(function, kind, own.adjusted, class)?
                .is_some()
                || group.is_some() && !self.entry_adjustment(class, 0, slot)?.is_zero());
        self.vtables.own_entry_thunks.insert((class, slot), result);
        Ok(result)
    }
}

#[cfg(test)]
pub(crate) use construction_tables::assert_construction_context;
