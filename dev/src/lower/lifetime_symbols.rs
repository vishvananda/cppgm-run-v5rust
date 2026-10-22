// Translated from lower/local_static_lowering.cpp. See NOTICE.
//! Runtime destruction registration shares one typed declaration per translation unit.
use super::{Error, Metadata, Program, SymbolId, Symbols};
impl Symbols<'_, '_> {
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
        self.table.atexit = Some(symbol);
        program.publish_runtime_declaration(symbol);
        Ok(symbol)
    }
}
