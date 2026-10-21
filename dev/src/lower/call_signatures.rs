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
        if let Some(&signature) = self.signatures.get(&(ty, nothrow)) {
            return Ok(signature);
        }
        let signature = crate::lower::function::signature(program, self, self.graph, ty, nothrow)?;
        self.signatures.insert((ty, nothrow), signature);
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
        if let Some(&signature) = self.signatures.get(&(pointer, nothrow)) {
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
        self.signatures.insert((pointer, nothrow), signature);
        Ok(signature)
    }
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn callee(
        &mut self,
        entity: EntityId,
        builder: &mut crate::lowir::builder::FunctionBuilder<'_>,
    ) -> Result<crate::lowir::model::Value, Error> {
        if let Some(&value) = self.callees.get(&entity) {
            return Ok(value);
        }
        let symbol = self.function_symbol(builder.program(), entity)?;
        let value = Value::Symbol(symbol);
        self.callees.insert(entity, value);
        Ok(value)
    }
    pub(in crate::lower) fn function_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<SymbolId, Error> {
        let entity = self.graph.lower_function_entity(entity);
        if let Some(Value::Symbol(symbol)) = self.callees.get(&entity) {
            return Ok(*symbol);
        }
        let name = self.name(entity)?;
        let facts = self.graph.lower_function(entity)?;
        let metadata = self.metadata(program, entity, &name)?;
        let object = if facts.builtin.is_some() {
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
                || object.is_some_and(|object| self.table.objects.contains_key(&object)))
        {
            None
        } else {
            metadata.object
        };
        let symbol = self
            .table
            .allocate(program, &name, object, facts.internal)?;
        self.objects.insert(entity, local_object);
        self.callees.insert(entity, Value::Symbol(symbol));
        if let Some(
            role @ (crate::lowir::vocabulary::Role::AllocateMemory
            | crate::lowir::vocabulary::Role::FreeMemory
            | crate::lowir::vocabulary::Role::Terminate),
        ) = metadata.role
        {
            self.table
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
        if self.table.object_mode && !program.symbol(symbol).declared() {
            let signature = crate::lower::function::declaration_signature(
                program,
                self,
                self.graph,
                crate::lower::construction::Entry::Function(entity),
            )?;
            program.set_forward_boundary(symbol, signature, metadata);
        }
        Ok(symbol)
    }
}
