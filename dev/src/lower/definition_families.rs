// Translated from lower/definition_order.cpp and program_lowering.cpp. See NOTICE.
//! Query-owned family identities and typed ranks shared by both `LowIR` views.
use super::{Error, Symbols};
use crate::lower::{construction::Entry, definition_order::FamilyId};
use crate::{
    abi,
    lowir::model::{NameId, Program},
    sema::lowering::{ConstructorId, EntityId, FunctionName},
};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Families {
    entries: HashMap<ConstructorId, FamilyId>,
    destructors: HashMap<EntityId, FamilyId>,
    copies: HashMap<(EntityId, bool), FamilyId>,
    aliases: HashMap<Entry, NameId>,
    classes: HashMap<EntityId, Option<NameId>>,
}
impl Families {
    pub(super) fn note_alias(&mut self, entry: Entry, object: NameId) {
        self.aliases.insert(entry, object);
    }
}
impl Symbols<'_, '_> {
    fn entry_family_object(
        &mut self,
        program: &mut Program,
        entry: crate::lower::construction::Entry,
    ) -> Result<NameId, Error> {
        use crate::lower::construction::{Entry, Variant};
        if let Some(&object) = self.families.aliases.get(&entry) {
            return Ok(object);
        }
        let base = match entry {
            Entry::Constructor(id, _) => Entry::Constructor(id, Variant::Base),
            Entry::DestructorBase(entity)
            | Entry::DestructorDeleting(entity)
            | Entry::Function(entity) => Entry::DestructorBase(entity),
            Entry::InlineCopy(_) => return Err(Error::Missing("inline copy entry family")),
        };
        if let Some(&Some(object)) = self.entry_objects.get(&base) {
            return Ok(object);
        }
        self.base_entry_object(program, entry)
    }
    pub(in crate::lower) fn stream_definition(
        &mut self,
        program: &mut Program,
        output: &mut crate::lower::definition_order::Definitions,
        entry: crate::lower::construction::Entry,
    ) -> Result<(), Error> {
        output.generated(&mut self.generated_definitions)?;
        if self.table.object_mode {
            if let crate::lower::construction::Entry::Function(entity) = entry
                && self.destructor_inert(entity)?
            {
                return output.defer_definition(program);
            }
            return output.stream(program, None);
        }
        let destructor = if matches!(self.entry_facts(entry)?.name, FunctionName::Destructor(_)) {
            entry.entity(self.graph)
        } else {
            None
        };
        let family = if let crate::lower::construction::Entry::Constructor(id, variant) = entry {
            let family = if let Some(&family) = self.families.entries.get(&id) {
                family
            } else {
                let facts = self.entry_facts(entry)?;
                let object = if facts.internal {
                    None
                } else {
                    Some(self.entry_family_object(program, entry)?)
                };
                let family = output.family(object)?;
                self.families.entries.insert(id, family);
                family
            };
            Some((family, variant))
        } else if let Some(entity) = destructor {
            let family = if let Some(&family) = self.families.destructors.get(&entity) {
                family
            } else {
                let facts = self.entry_facts(entry)?;
                let object = if facts.internal {
                    None
                } else {
                    Some(self.entry_family_object(program, entry)?)
                };
                let family = output.family(object)?;
                self.families.destructors.insert(entity, family);
                family
            };
            Some((
                family,
                match entry {
                    crate::lower::construction::Entry::DestructorBase(_) => {
                        crate::lower::construction::Variant::Base
                    }
                    crate::lower::construction::Entry::DestructorDeleting(_) => {
                        crate::lower::construction::Variant::Deleting
                    }
                    _ => crate::lower::construction::Variant::Complete,
                },
            ))
        } else {
            None
        };
        if let crate::lower::construction::Entry::Function(entity) = entry
            && self.destructor_inert(entity)?
        {
            // Later object declarations may root this destructor after its
            // source definition. Release its body now, defer only its header.
            output.defer_definition(program)
        } else {
            output.stream(program, family)?;
            if let Some(entity) = entry.entity(self.graph)
                && let Some(form) = self.graph.lower_copy_form(entity)?
            {
                let key = (form.class, form.assignment);
                let family = if let Some(&family) = self.families.copies.get(&key) {
                    family
                } else {
                    let ty = self.graph.lower_entity_type(form.class)?;
                    let object = if let Some(&object) = self.families.classes.get(&form.class) {
                        object
                    } else if form.locality == crate::sema::lowering::Locality::Unit {
                        self.families.classes.insert(form.class, None);
                        None
                    } else {
                        let node = self.abi_type(ty)?;
                        let mut bytes = Vec::new();
                        self.table.work.mangled_names += 1;
                        abi::encoder::Encoder::encode(
                            &self.abi,
                            abi::encoder::Target::Type(node),
                            &mut bytes,
                            self.budget,
                        )?;
                        let object = Some(program.names.intern_bytes(&bytes)?);
                        self.families.classes.insert(form.class, object);
                        object
                    };
                    let family = output.form_family(object, form.assignment)?;
                    self.families.copies.insert(key, family);
                    family
                };
                output.note_form(family, form.moving)?;
            }
            Ok(())
        }
    }
}
