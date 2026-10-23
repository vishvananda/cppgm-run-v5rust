// Translated from lower/program_lowering.cpp::inline_copy/emit_demanded and
// dispatch_lowering.cpp::member_callee. See NOTICE.
//! Direct calls own a unit-local definition; addresses and statics keep the
//! original entity's identity. Copies are drained before ordinary demands.
use super::{EntityId, Error, Metadata, Program, SymbolId, Symbols};
use crate::lower::construction::Entry;
use crate::lowir::{model::Value, vocabulary::Binding};

impl Symbols<'_, '_> {
    pub(in crate::lower) fn finish_inline_declarations(
        &mut self,
        program: &mut Program,
    ) -> Result<(), Error> {
        for index in 0..self.inline_static_owners.len() {
            let entity = self.inline_static_owners[index];
            crate::lower::function::declare(program, self, self.graph, Entry::Function(entity))?;
        }
        self.inline_static_owners.clear();
        Ok(())
    }

    pub(in crate::lower) fn direct_callee(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<Value, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if !self.graph.lower_always_inline(entity)
            || !self.graph.lower_inline_copy_available(entity)
        {
            self.demand(entity);
            return Ok(Value::Symbol(self.function_symbol(program, entity)?));
        }
        let entry = Entry::InlineCopy(entity);
        if let Some(&symbol) = self.entries.get(&entry) {
            return Ok(Value::Symbol(symbol));
        }
        let name = self.name(entity)?;
        // Form the ABI spelling independently of the original's local-object
        // publication: a prior unit may already own that spelling.
        let facts = self.function_facts(entity)?;
        let mut object = if facts.c_linkage {
            self.source_function_spelling(facts.name)?.into_bytes()
        } else {
            let encoding = self.function_encoding(entity)?;
            let mut bytes = Vec::new();
            crate::abi::encoder::Encoder::encode(
                &self.abi,
                crate::abi::encoder::Target::Entity(encoding),
                &mut bytes,
                self.budget,
            )?;
            bytes
        };
        object.extend_from_slice(b".always_inline");
        let object = program.names.intern_bytes(&object)?;
        let local_object = (!self.table.objects.contains_key(&object)).then_some(object);
        let symbol = self.table.allocate(
            program,
            &format!("{name}.always_inline"),
            Some(object),
            true,
        )?;
        self.entries.insert(entry, symbol);
        self.entry_objects.insert(entry, local_object);
        self.pending_copies.push(entity);
        // The boundary is available while the body is still queued. This is
        // equally required by retained views and the consuming O0 object sink.
        let signature =
            crate::lower::function::declaration_signature(program, self, self.graph, entry)?;
        let metadata = self.inline_copy_metadata(program, entity, symbol)?;
        program.set_forward_boundary(symbol, signature, metadata);
        Ok(Value::Symbol(symbol))
    }

    pub(in crate::lower) fn inline_copy_metadata(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        symbol: SymbolId,
    ) -> Result<Metadata, Error> {
        let name = program.names.get(program.symbol(symbol).name).to_owned();
        let mut metadata = self.metadata(program, entity, &name)?;
        metadata.binding = Some(Binding::Internal);
        metadata.object = self.entry_objects[&Entry::InlineCopy(entity)]
            .filter(|&object| object != program.symbol(symbol).name);
        Ok(metadata)
    }
}
