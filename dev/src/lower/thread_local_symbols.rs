// Translated from lower/global_lowering.cpp TLS symbol ownership. See NOTICE.
use super::{EntityId, Error, HashMap, Metadata, Program, SymbolId, Symbols, Value, abi};
use crate::lowir::{
    model::{Function, Global, GlobalInit, Signature},
    vocabulary::{Binding, Storage, Type},
};
#[derive(Default)]
pub(super) struct State {
    wrappers: HashMap<EntityId, SymbolId>,
    used: std::collections::HashSet<EntityId>,
    unit: Option<Unit>,
}
pub(in crate::lower) struct Unit {
    pub init: SymbolId,
    pub guard: SymbolId,
    pub initializers: Vec<SymbolId>,
}
impl Symbols<'_, '_> {
    pub(in crate::lower) fn tls_wrapper(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        global: SymbolId,
    ) -> Result<Option<SymbolId>, Error> {
        if let Some(&wrapper) = self.tls.wrappers.get(&entity) {
            return Ok((!self.object_mode() || self.tls.used.contains(&entity)).then_some(wrapper));
        }
        let facts = self.graph.lower_variable(entity)?;
        if facts.definition.is_some() {
            // A definition not published yet needs its future initializer's
            // wrapper. A completed static definition is addressed directly.
            if !program.symbol(global).declared() {
                return self.tls_wrapper_symbol(program, entity, global).map(Some);
            }
        } else if self.object_mode() && facts.locality != crate::sema::lowering::Locality::Unit {
            let wrapper = self.tls_wrapper_symbol(program, entity, global)?;
            let init = self.tls_init_entry(program, entity, false)?;
            self.emit_tls_wrapper(program, entity, Some(init))?;
            program.stream_last_definition(&mut self.generated_definitions)?;
            return Ok(Some(wrapper));
        }
        Ok(None)
    }

    pub(in crate::lower) fn tls_wrapper_symbol(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        global: SymbolId,
    ) -> Result<SymbolId, Error> {
        self.tls.used.insert(entity);
        self.tls_wrapper_symbol_in(program, entity, global)
    }
    fn tls_wrapper_symbol_in(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        global: SymbolId,
    ) -> Result<SymbolId, Error> {
        if let Some(&wrapper) = self.tls.wrappers.get(&entity) {
            return Ok(wrapper);
        }
        let spelling = format!(
            "__cppgm_tls_wrapper__{}",
            program.names.get(program.symbol(global).name)
        );
        let name = self.variable_abi_name(entity)?;
        let mut bytes = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::TlsWrapper(name),
            &mut bytes,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&bytes)?;
        let internal =
            self.graph.lower_variable(entity)?.locality == crate::sema::lowering::Locality::Unit;
        let local_object =
            (!internal || !self.table.objects.contains_key(&object)).then_some(object);
        let wrapper = self
            .table
            .allocate(program, &spelling, Some(object), internal)?;
        let object = local_object;
        self.tls.wrappers.insert(entity, wrapper);
        // The wrapper's ABI identity is retained on its own symbol declaration.
        let signature = program.add_signature(crate::lowir::model::Signature {
            parameters: Vec::new(),
            result: crate::lowir::vocabulary::Type::Ptr,
            boundary: crate::lowir::model::Boundary::default(),
        })?;
        let name = program.symbol(wrapper).name;
        program.add(
            wrapper,
            crate::lowir::model::Function {
                name,
                signature,
                metadata: Metadata {
                    binding: Some(if internal {
                        crate::lowir::vocabulary::Binding::Internal
                    } else {
                        crate::lowir::vocabulary::Binding::Weak
                    }),
                    object,
                    tls_for: Some(global),
                    ..Metadata::default()
                },
                body: None,
                location: None,
            },
        )?;
        let function =
            program
                .function(program.symbol(wrapper))
                .ok_or(crate::sema::Error::Invariant(
                    "TLS wrapper declaration missing",
                ))?;
        let boundary = (function.signature, function.metadata);
        program.set_forward_boundary(wrapper, boundary.0, boundary.1);
        Ok(wrapper)
    }
    pub(in crate::lower) fn tls_initialization_symbols(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<(SymbolId, SymbolId), Error> {
        use crate::lowir::{
            model::{Global, GlobalInit},
            vocabulary::{Binding, Storage},
        };
        let global = self.variable_symbol(program, entity)?;
        let base = program.names.get(program.symbol(global).name).to_owned();
        let guard =
            self.table
                .allocate(program, &format!("__cppgm_tls_guard__{base}"), None, true)?;
        program.add_global(
            guard,
            Global {
                name: program.symbol(guard).name,
                ty: Some(crate::lowir::vocabulary::Type::I64),
                init: GlobalInit::Zero,
                metadata: Metadata {
                    binding: Some(Binding::Internal),
                    storage: Some(Storage::ThreadLocal),
                    ..Metadata::default()
                },
            },
        )?;
        let init =
            self.table
                .allocate(program, &format!("__cppgm_tls_init__{base}"), None, true)?;
        Self::tls_function(
            program,
            init,
            Metadata {
                binding: Some(Binding::Internal),
                ..Metadata::default()
            },
        )?;
        Ok((guard, init))
    }
    pub(in crate::lower) fn emit_tls_wrapper(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        initializer: Option<SymbolId>,
    ) -> Result<bool, Error> {
        let Some(&wrapper) = self.tls.wrappers.get(&entity) else {
            return Ok(false);
        };
        let global = self.variable_symbol(program, entity)?;
        let optional = initializer.is_some_and(|init| {
            !program.function_defined(init)
                && program
                    .function(program.symbol(init))
                    .is_some_and(|f| f.metadata.binding == Some(Binding::Weak))
        });
        let mut builder =
            crate::lowir::builder::FunctionBuilder::from_forward_boundary(program, wrapper)?;
        builder.block("entry")?;
        if let Some(initializer) = initializer {
            let done = if optional {
                let run =
                    builder.generated_block("local_static_ctor_run1", "local_static_ctor_run")?;
                let done =
                    builder.generated_block("local_static_ctor_done2", "local_static_ctor_done")?;
                let zero = builder.integer(0)?;
                let test = builder.generated(crate::lowir::model::InstructionKind::Compare(
                    crate::lowir::vocabulary::Compare::Ne,
                    Type::Ptr,
                    Value::Symbol(initializer),
                    zero,
                ))?;
                builder.effect(crate::lowir::model::InstructionKind::Branch(
                    test, run, done,
                ))?;
                builder.place_block(run)?;
                Some(done)
            } else {
                None
            };
            let arguments = builder.arguments(&[])?;
            builder.effect(crate::lowir::model::InstructionKind::Call(
                crate::lowir::model::Call {
                    result: crate::lowir::vocabulary::Type::Void,
                    callee: Value::Symbol(initializer),
                    arguments,
                    signature: None,
                    copy_elision: false,
                    argument_classes: Vec::new(),
                },
            ))?;
            if let Some(done) = done {
                builder.effect(crate::lowir::model::InstructionKind::Jump(done))?;
                builder.place_block(done)?;
            }
        }
        let address = builder.generated(crate::lowir::model::InstructionKind::Addr(
            Value::Symbol(global),
        ))?;
        builder.effect(crate::lowir::model::InstructionKind::Return(
            crate::lowir::vocabulary::Type::Ptr,
            Some(address),
        ))?;
        builder.finish()?;
        Ok(true)
    }
    pub(in crate::lower) fn tls_init_entry(
        &mut self,
        program: &mut Program,
        entity: EntityId,
        define: bool,
    ) -> Result<SymbolId, Error> {
        let global = self.variable_symbol(program, entity)?;
        let name = self.variable_abi_name(entity)?;
        let mut bytes = Vec::new();
        abi::encoder::Encoder::encode(
            &self.abi,
            abi::encoder::Target::TlsInit(name),
            &mut bytes,
            self.budget,
        )?;
        let object = program.names.intern_bytes(&bytes)?;
        let spelling = format!(
            "__cppgm_tls_init__{}",
            program.names.get(program.symbol(global).name)
        );
        let symbol = self
            .table
            .allocate(program, &spelling, Some(object), false)?;
        if program.function(program.symbol(symbol)).is_none() {
            let binding = if !define
                || program
                    .global(program.symbol(global))
                    .is_some_and(|g| g.metadata.binding == Some(Binding::Weak))
            {
                Binding::Weak
            } else {
                Binding::Strong
            };
            Self::tls_function(
                program,
                symbol,
                Metadata {
                    binding: Some(binding),
                    object: Some(object),
                    flags: if define {
                        program
                            .global(program.symbol(global))
                            .map_or(0, |g| g.metadata.flags & 64)
                    } else {
                        0
                    },
                    ..Metadata::default()
                },
            )?;
            if !define {
                // The synthetic declaration is published at first demand;
                // ordinary callable declarations remain pending until the end.
                program.publish_runtime_declaration(symbol);
                program.optional_reference(symbol);
            }
        }
        Ok(symbol)
    }
    fn tls_function(
        program: &mut Program,
        symbol: SymbolId,
        metadata: Metadata,
    ) -> Result<(), Error> {
        let signature = program.add_signature(Signature {
            parameters: Vec::new(),
            result: Type::Void,
            boundary: crate::lowir::model::Boundary::default(),
        })?;
        program.set_forward_boundary(symbol, signature, metadata);
        program.add(
            symbol,
            Function {
                name: program.symbol(symbol).name,
                signature,
                metadata,
                body: None,
                location: None,
            },
        )?;
        Ok(())
    }
    pub(in crate::lower) fn tls_initializer(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<(Option<SymbolId>, SymbolId, bool), Error> {
        if !self.object_mode() {
            let (guard, init) = self.tls_initialization_symbols(program, entity)?;
            return Ok((Some(guard), init, false));
        }
        let global = self.variable_symbol(program, entity)?;
        let weak = program
            .global(program.symbol(global))
            .is_some_and(|g| g.metadata.binding == Some(Binding::Weak));
        let guard = if weak {
            let guard = self.instance_guard(program, entity, true)?;
            Some(guard)
        } else {
            None
        };
        let unordered = self.graph.lower_unordered_variable(entity)
            && self.graph.lower_variable(entity)?.locality != crate::sema::lowering::Locality::Unit;
        let init = if unordered {
            self.tls_init_entry(program, entity, true)?
        } else {
            let spelling = format!(
                "__cppgm_tls_init__{}",
                program.names.get(program.symbol(global).name)
            );
            let init = self.table.allocate(program, &spelling, None, true)?;
            Self::tls_function(
                program,
                init,
                Metadata {
                    binding: Some(Binding::Internal),
                    ..Metadata::default()
                },
            )?;
            init
        };
        Ok((guard, init, unordered))
    }
    pub(in crate::lower) fn tls_ordered_entry(
        &mut self,
        program: &mut Program,
        init: SymbolId,
    ) -> Result<SymbolId, Error> {
        if self.tls.unit.is_none() {
            let init = self
                .table
                .allocate(program, "__cppgm_tls_init", None, true)?;
            Self::tls_function(
                program,
                init,
                Metadata {
                    binding: Some(Binding::Internal),
                    ..Metadata::default()
                },
            )?;
            let guard = self
                .table
                .allocate(program, "__cppgm_tls_guard", None, true)?;
            program.add_global(
                guard,
                Global {
                    name: program.symbol(guard).name,
                    ty: Some(Type::I8),
                    init: GlobalInit::Zero,
                    metadata: Metadata {
                        binding: Some(Binding::Internal),
                        storage: Some(Storage::ThreadLocal),
                        ..Metadata::default()
                    },
                },
            )?;
            self.tls.unit = Some(Unit {
                init,
                guard,
                initializers: Vec::new(),
            });
        }
        let unit = self.tls.unit.as_mut().ok_or(crate::sema::Error::Invariant(
            "TLS unit initialization missing",
        ))?;
        unit.initializers.push(init);
        Ok(unit.init)
    }
    pub(in crate::lower) fn take_tls_unit(&mut self) -> Option<Unit> {
        self.tls.unit.take()
    }
    pub(in crate::lower) fn tls_static_entry(
        &mut self,
        program: &mut Program,
        entity: EntityId,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_variable(entity)?;
        if !self.object_mode()
            || !facts.thread_local
            || facts.locality == crate::sema::lowering::Locality::Unit
        {
            return Ok(());
        }
        let entry = self.tls_init_entry(program, entity, true)?;
        super::super::thread_local::forwarder(program, entry, None)?;
        program.stream_last_definition(&mut self.generated_definitions)?;
        if !self.tls.wrappers.contains_key(&entity) {
            let global = self.variable_symbol(program, entity)?;
            self.tls_wrapper_symbol_in(program, entity, global)?;
            self.emit_tls_wrapper(program, entity, None)?;
            program.stream_last_definition(&mut self.generated_definitions)?;
        }
        Ok(())
    }
}
