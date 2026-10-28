// Translated from lower/symbol_lowering.cpp::call_signature/member_call_signature. See NOTICE.
//! Canonical callable types key reusable indirect call boundaries, including adjusted objects.
use super::{EntityId, Error, Program, Symbols, TypeId};
use crate::lowir::model::{SymbolId, Value};

impl Symbols<'_, '_> {
    pub(in crate::lower) fn call_signature(
        &mut self,
        program: &mut Program,
        ty: TypeId,
        nothrow: bool,
    ) -> Result<crate::lowir::model::SignatureId, Error> {
        if let Some(&signature) = self.state.signatures.get(&(ty, nothrow)) {
            return Ok(signature);
        }
        let signature = crate::lower::function::signature(program, self, self.graph, ty, nothrow)?;
        let crate::sema::lowering::TypeView::Function { parameters, .. } =
            self.graph.lower_type(ty)?
        else {
            return Err(crate::sema::Error::Invariant("indirect call signature type").into());
        };
        let stored = &program.signature(signature).parameters;
        let first = usize::from(stored.first().is_some_and(|parameter| {
            parameter.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)
        }));
        // A later demand can complete a reference object's boundary. Preserve
        // C++'s uncached signature while the parameter states no object size.
        for (index, &parameter) in parameters.iter().enumerate() {
            if let crate::sema::lowering::TypeView::Reference { element, .. } =
                self.graph.lower_type(parameter)?
                && !matches!(
                    self.graph.lower_type(element)?,
                    crate::sema::lowering::TypeView::Function { .. }
                )
                && stored[first + index].object_bytes == 0
            {
                return Ok(signature);
            }
        }
        self.state.signatures.insert((ty, nothrow), signature);
        Ok(signature)
    }

    pub(in crate::lower) fn member_call_signature(
        &mut self,
        program: &mut Program,
        pointer: TypeId,
        nothrow: bool,
    ) -> Result<crate::lowir::model::SignatureId, Error> {
        use crate::lower::types;
        use crate::lowir::vocabulary::Type;
        if let Some(&signature) = self.state.signatures.get(&(pointer, nothrow)) {
            return Ok(signature);
        }
        let graph = self.graph;
        let (owner, function) = graph
            .lower_member_pointer_type(pointer)
            .ok_or(crate::sema::Error::Invariant("member call signature type"))?;
        let signature = crate::lower::function::signature(program, self, graph, function, nothrow)?;
        let first = usize::from(
            program
                .signature(signature)
                .parameters
                .first()
                .is_some_and(|p| p.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)),
        );
        let crate::sema::lowering::TypeView::Class(class) = graph.lower_type(owner)? else {
            return Err(crate::sema::Error::Invariant("member call signature owner").into());
        };
        let object_bytes = if graph.lower_class_bases(class).next().is_none() {
            types::layout(graph, owner)?.0
        } else {
            0
        };
        let name = program.names.intern("arg0")?;
        program.signature_mut(signature).parameters.insert(
            first,
            crate::lowir::model::Parameter {
                name,
                ty: Type::Ptr,
                pass: None,
                object_bytes,
                no_alias: false,
                classes: None,
            },
        );
        for index in 0..program.signature(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{index}"))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
        self.state.signatures.insert((pointer, nothrow), signature);
        Ok(signature)
    }
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn function_facts(
        &self,
        entity: EntityId,
    ) -> Result<crate::sema::lowering::FunctionFacts, Error> {
        self.entry_facts(crate::lower::construction::Entry::Function(entity))
    }
    pub(in crate::lower) fn entry_facts(
        &self,
        entry: crate::lower::construction::Entry,
    ) -> Result<crate::sema::lowering::FunctionFacts, Error> {
        entry.facts_in_mode(self.graph, self.object_mode())
    }
    pub(in crate::lower) fn callee(
        &mut self,
        entity: EntityId,
        builder: &mut crate::lowir::builder::FunctionBuilder<'_>,
    ) -> Result<crate::lowir::model::Value, Error> {
        if let Some(&value) = self.state.callees.get(&entity) {
            return Ok(value);
        }
        let symbol = self.function_symbol(builder.program(), entity)?;
        let value = Value::Symbol(symbol);
        self.state.callees.insert(entity, value);
        Ok(value)
    }
    pub(in crate::lower) fn function_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(Value::Symbol(symbol)) = self.state.callees.get(&entity) {
            return Ok(*symbol);
        }
        let name = self.name(entity)?;
        let facts = self.function_facts(entity)?;
        let mut metadata = self.metadata(program, entity, &name)?;
        let object = if facts.builtin.is_none()
            && let Some(label) = self.graph.lower_asm_label(entity)
        {
            Some(program.names.intern_bytes(label)?)
        } else if facts.builtin.is_some() {
            metadata.object.or(Some(program.names.intern(&name)?))
        } else if facts.c_linkage && !facts.internal
            || metadata.role == Some(crate::lowir::vocabulary::Role::Entry)
        {
            Some(program.names.intern(&name)?)
        } else {
            metadata.object
        };
        let local_object = if facts.internal
            && (facts.c_linkage
                || object.is_some_and(|object| self.state.table.objects.contains_key(&object)))
        {
            None
        } else {
            metadata.object
        };
        let symbol = self
            .state
            .table
            .allocate(program, &name, object, facts.internal)?;
        let local_object = if facts.c_linkage && !facts.internal {
            local_object.or(object.filter(|&object| {
                !program.names.get(object).is_empty() && program.symbol(symbol).name != object
            }))
        } else {
            local_object
        };
        if facts.c_linkage && !facts.internal && local_object.is_some() {
            metadata.object = local_object;
        }
        self.state.objects.insert(entity, local_object);
        self.state.callees.insert(entity, Value::Symbol(symbol));
        if let Some(
            role @ (crate::lowir::vocabulary::Role::AllocateMemory
            | crate::lowir::vocabulary::Role::FreeMemory
            | crate::lowir::vocabulary::Role::Terminate),
        ) = metadata.role
        {
            self.state
                .table
                .runtime_roles
                .push(crate::lower::runtime_roles::RoleSymbol {
                    symbol,
                    role,
                    origin: if facts.allocation.is_some_and(|allocation| allocation.array)
                        || matches!(
                            facts.builtin,
                            Some(crate::sema::lowering::BuiltinFunction::Abort)
                        ) {
                        crate::lower::runtime_roles::Origin::Secondary
                    } else {
                        crate::lower::runtime_roles::Origin::Primary
                    },
                });
        }
        if !program.symbol(symbol).declared() {
            self.state.declarations.reserve(
                symbol,
                crate::lower::construction::Entry::Function(entity),
                metadata,
            );
        }
        Ok(symbol)
    }
}
