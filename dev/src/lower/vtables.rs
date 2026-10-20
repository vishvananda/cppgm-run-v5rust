// Translated from lower/vtable_lowering.cpp and rtti_lowering.cpp. See NOTICE.
//! Demanded primary tables and readonly class RTTI, keyed by semantic class identity.
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
}
#[derive(Clone, Copy)]
enum ClassObject {
    Table,
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
        Encoder::encode(
            &self.abi,
            match kind {
                ClassObject::Table => Target::Vtable(node),
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
        let (internal, local, key, scope) = self.graph.lower_virtual_class_identity(class)?;
        let prefix = match kind {
            ClassObject::Table => "__vtable_",
            ClassObject::Typeinfo => "__rtti_",
            ClassObject::Name => "__typeinfo_name_",
        };
        let spelling = if internal && local {
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
            if matches!(kind, ClassObject::Table) {
                format!("{label}__vtable")
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
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), internal)?;
        let metadata = Metadata {
            binding: Some(if internal {
                Binding::Internal
            } else {
                Binding::Weak
            }),
            object: Some(object),
            ..Metadata::default()
        };
        if !program.symbol(symbol).declared() {
            self.vtables.pending.push_back((symbol, metadata));
        }
        Ok((symbol, metadata))
    }
    pub(in crate::lower) fn construction_table(
        &mut self,
        program: &mut Program,
        class: EntityId,
    ) -> Result<(), Error> {
        if self.graph.lower_polymorphic(class) && self.graph.lower_emits_vtable(class)? {
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
            && self.graph.lower_function(key)?.emission == Emission::Always
            && metadata.binding != Some(Binding::Internal)
        {
            metadata.binding = Some(Binding::Strong);
        }
        let typeinfo = self.typeinfo_symbol(program, class)?;
        let slots = self.graph.lower_virtual_slots(class)?;
        let mut items = Vec::with_capacity(slots.len() + 2);
        items.push(integer(Type::I64, 0));
        items.push(address(typeinfo, 0));
        for slot in slots {
            let slot = slot.ok_or(crate::sema::Error::Invariant("unfilled virtual slot"))?;
            let target = if self.graph.lower_virtual_pure(slot.function) {
                self.pure_virtual_symbol(program, slot.function)?
            } else if matches!(slot.kind, SlotKind::Function) {
                self.demand(slot.function);
                self.function_symbol(program, slot.function)?
            } else {
                self.demand_entry(Entry::DestructorDeleting(slot.function));
                self.demand_entry(Entry::Function(slot.function));
                self.entry_symbol(
                    program,
                    if matches!(slot.kind, SlotKind::DeletingDestructor) {
                        Entry::DestructorDeleting(slot.function)
                    } else {
                        Entry::Function(slot.function)
                    },
                )?
            };
            items.push(address(target, 0));
        }
        define(program, symbol, metadata, items)?;
        Ok(symbol)
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
            let (name, _) = self.class_encoding(class, ClassObject::Name)?;
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
        let single = matches!(bases.as_slice(), [(_, true, 0)]);
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
        for &(base, _, _) in &bases {
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
            for (&(_, public, offset), target) in bases.iter().zip(targets) {
                items.push(address(target, 0));
                items.push(integer(
                    Type::I64,
                    offset.wrapping_shl(8) | if public { 2 } else { 0 },
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
        let facts = self.graph.lower_function(entity)?;
        let signature = function::signature(program, self, self.graph, facts.ty, facts.nothrow)?;
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
    pub(in crate::lower) fn key_function_table(
        &mut self,
        program: &mut Program,
        entry: Entry,
    ) -> Result<(), Error> {
        if let Entry::Function(entity) = entry
            && let Some(class) = self.graph.lower_virtual_key_owner(entity)
        {
            self.vtable_symbol(program, class)?;
        }
        Ok(())
    }
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
    for (&ty, expected) in parameters.iter().zip([0, 1, 2, 3, 3, 0]) {
        let TypeView::Pointer(pointee) = graph.lower_type(ty).unwrap() else {
            panic!("anchor pointer");
        };
        let TypeView::Class(class) = graph.lower_type(pointee).unwrap() else {
            panic!("anchor class");
        };
        assert_eq!(symbols.inheritance_flags(class), expected);
        assert!(graph.lower_class_layout(class).is_err());
        assert!(graph.lower_virtual_slots(class).is_err());
    }
    assert_eq!(symbols.pending.len(), 0);
    assert!(symbols.vtables.objects.is_empty());
    assert!(symbols.vtables.typeinfos.is_empty());
    assert!(program.globals().is_empty());
}
