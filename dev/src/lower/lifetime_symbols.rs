// Translated from lower/local_static_lowering.cpp and global_lowering.cpp. See NOTICE.
//! Runtime destruction registration shares one typed declaration per translation unit.
use super::{Error, Metadata, Program, SymbolId, Symbols};
use crate::sema::lowering::EntityId;
use std::collections::HashMap;

/// One unordered initializer owns its persistent temporaries in construction
/// order, including trivial ones that need no guard. Nested unordered
/// initializers replace this owner, matching the frozen single active context.
/// Only scratch capacity survives completion; no function or unrelated
/// variable participates in lookup.
#[derive(Default)]
pub(in crate::lower) struct OwnedTemporaries {
    owner: Option<EntityId>,
    entries: Vec<(SymbolId, Option<SymbolId>)>,
    indices: HashMap<SymbolId, usize>,
}

impl Symbols<'_, '_> {
    pub(in crate::lower) fn begin_owned_temporaries(&mut self, owner: EntityId) {
        self.owned_temporaries.owner =
            (self.host_objects() && self.graph.lower_variable_instantiated(owner)).then_some(owner);
        self.owned_temporaries.entries.clear();
        self.owned_temporaries.indices.clear();
    }

    pub(super) fn note_owned_temporary(&mut self, symbol: SymbolId) {
        let context = &mut self.owned_temporaries;
        if context.owner.is_some() {
            context.indices.insert(symbol, context.entries.len());
            context.entries.push((symbol, None));
        }
    }

    pub(in crate::lower) fn temporary_registration_guard(
        &mut self,
        program: &mut Program,
        temporary: SymbolId,
    ) -> Result<Option<SymbolId>, Error> {
        use crate::lowir::{
            model::{Global, GlobalInit},
            vocabulary::{Binding, Type},
        };
        let context = &self.owned_temporaries;
        if context.owner.is_none() {
            return Ok(None);
        }
        let Some(&index) = context.indices.get(&temporary) else {
            return Ok(None);
        };
        if let Some(guard) = context.entries[index].1 {
            return Ok(Some(guard));
        }
        let spelling = format!(
            "__cppgm_guard__{}",
            program.names.get(program.symbol(temporary).name)
        );
        let guard = self.table.allocate(program, &spelling, None, true)?;
        program.add_global(
            guard,
            Global {
                name: program.symbol(guard).name,
                ty: Some(Type::I64),
                metadata: Metadata {
                    binding: Some(Binding::Internal),
                    ..Metadata::default()
                },
                init: GlobalInit::Zero,
            },
        )?;
        self.owned_temporaries.entries[index].1 = Some(guard);
        Ok(Some(guard))
    }

    pub(in crate::lower) fn end_owned_temporaries(
        &mut self,
        program: &mut Program,
    ) -> Result<(), Error> {
        let Some(owner) = self.owned_temporaries.owner.take() else {
            return Ok(());
        };
        if self.owned_temporaries.entries.is_empty() {
            return Ok(());
        }
        let name = self.variable_abi_name(owner)?;
        for index in 0..self.owned_temporaries.entries.len() {
            let Some(guard) = self.owned_temporaries.entries[index].1 else {
                continue;
            };
            let sequence = u32::try_from(index).map_err(|_| crate::lowir::Error::Limit)?;
            let mut bytes = Vec::new();
            crate::abi::encoder::Encoder::encode(
                &self.abi,
                crate::abi::encoder::Target::TemporaryGuard {
                    owner: name,
                    sequence,
                },
                &mut bytes,
                self.budget,
            )?;
            let object = program.names.intern_bytes(&bytes)?;
            program.set_global_binding_and_object(
                guard,
                crate::lowir::vocabulary::Binding::Weak,
                object,
            )?;
        }
        self.owned_temporaries.entries.clear();
        self.owned_temporaries.indices.clear();
        Ok(())
    }

    pub(in crate::lower) fn guard_function(
        &mut self,
        program: &mut Program,
        call: super::super::local_storage::GuardCall,
    ) -> Result<SymbolId, Error> {
        use crate::lowir::{
            model::{Boundary, Function, Linkage, Parameter, Signature},
            vocabulary::{Binding, Type},
        };
        if let Some(symbol) = self.table.guard_functions[call.index()] {
            return Ok(symbol);
        }
        let stem = match call {
            super::super::local_storage::GuardCall::Acquire => "cxa_guard_acquire",
            super::super::local_storage::GuardCall::Release => "cxa_guard_release",
            super::super::local_storage::GuardCall::Abort => "cxa_guard_abort",
        };
        let symbol =
            self.table
                .allocate(program, &format!("__cppgm_runtime_{stem}"), None, true)?;
        let parameter = Parameter {
            name: program.names.intern("arg0")?,
            ty: Type::Ptr,
            pass: None,
            object_bytes: 0,
            no_alias: false,
            classes: None,
        };
        let signature = program.add_signature(Signature {
            parameters: vec![parameter],
            result: call.result(),
            boundary: Boundary::default(),
        })?;
        let object = program.names.intern(&format!("__{stem}"))?;
        program.add(
            symbol,
            Function {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        // local_static_lowering.cpp declares lifetime helpers outside its
        // lowering declaration-work counter.
        self.table.work.lifetime_declarations.insert(symbol);
        self.table.guard_functions[call.index()] = Some(symbol);
        program.publish_runtime_declaration(symbol);
        Ok(symbol)
    }
    pub(in crate::lower) fn thread_atexit_function(
        &mut self,
        program: &mut Program,
    ) -> Result<SymbolId, Error> {
        use crate::lowir::{
            model::{Boundary, Function, Linkage, Parameter, Signature},
            vocabulary::{Binding, Type},
        };
        if let Some(symbol) = self.table.thread_atexit {
            return Ok(symbol);
        }
        let symbol =
            self.table
                .allocate(program, "__cppgm_runtime_cxa_thread_atexit", None, true)?;
        let mut parameters = Vec::with_capacity(3);
        for index in 0..3 {
            parameters.push(Parameter {
                name: program.names.intern(&format!("arg{index}"))?,
                ty: Type::Ptr,
                pass: None,
                object_bytes: 0,
                no_alias: false,
                classes: None,
            });
        }
        let signature = program.add_signature(Signature {
            parameters,
            result: Type::I32,
            boundary: Boundary::default(),
        })?;
        let object = program.names.intern("__cxa_thread_atexit")?;
        program.add(
            symbol,
            Function {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        // local_static_lowering.cpp declares lifetime helpers outside its
        // lowering declaration-work counter.
        self.table.work.lifetime_declarations.insert(symbol);
        self.table.thread_atexit = Some(symbol);
        program.publish_runtime_declaration(symbol);
        Ok(symbol)
    }
    pub(in crate::lower) fn dso_handle(
        &mut self,
        program: &mut Program,
    ) -> Result<SymbolId, Error> {
        use crate::lowir::{
            model::{Global, GlobalInit, Linkage},
            vocabulary::Binding,
        };
        if let Some(symbol) = self.table.dso_handle {
            return Ok(symbol);
        }
        let symbol = self
            .table
            .allocate(program, "__cppgm_runtime_dso_handle", None, true)?;
        let object = program.names.intern("__dso_handle")?;
        program.add_global(
            symbol,
            Global {
                name: program.symbol(symbol).name,
                ty: None,
                metadata: Metadata {
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                init: GlobalInit::Declaration,
            },
        )?;
        // local_static_lowering.cpp declares lifetime helpers outside its
        // lowering declaration-work counter.
        self.table.work.lifetime_declarations.insert(symbol);
        self.table.dso_handle = Some(symbol);
        Ok(symbol)
    }
    pub(in crate::lower) fn local_destructor_symbol(
        &mut self,
        program: &mut Program,
        global: SymbolId,
    ) -> Result<SymbolId, Error> {
        let spelling = format!(
            "__cppgm_local_static_destructor__{}",
            program.names.get(program.symbol(global).name)
        );
        self.table.allocate(program, &spelling, None, true)
    }
    pub(in crate::lower) fn atexit_function(
        &mut self,
        program: &mut Program,
    ) -> Result<SymbolId, Error> {
        use crate::lowir::{
            model::{Boundary, Linkage, Parameter, Signature},
            vocabulary::{Binding, Type},
        };
        if let Some(symbol) = self.table.atexit {
            return Ok(symbol);
        }
        let object = program.names.intern("atexit")?;
        let symbol = self
            .table
            .allocate(program, "__cppgm_runtime_atexit", None, true)?;
        let parameter = Parameter {
            name: program.names.intern("arg0")?,
            ty: Type::Ptr,
            pass: None,
            object_bytes: 0,
            no_alias: false,
            classes: None,
        };
        let signature = program.add_signature(Signature {
            parameters: vec![parameter],
            result: Type::I32,
            boundary: Boundary {
                unwind: crate::lowir::model::Unwind::No,
                ..Boundary::default()
            },
        })?;
        program.add(
            symbol,
            crate::lowir::model::Function {
                name: program.symbol(symbol).name,
                signature,
                metadata: Metadata {
                    linkage: Linkage::C,
                    binding: Some(Binding::Strong),
                    object: Some(object),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        // local_static_lowering.cpp declares lifetime helpers outside its
        // lowering declaration-work counter.
        self.table.work.lifetime_declarations.insert(symbol);
        self.table.atexit = Some(symbol);
        program.publish_runtime_declaration(symbol);
        Ok(symbol)
    }
}
