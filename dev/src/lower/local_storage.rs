// Translated from lower/local_static_lowering.cpp. See NOTICE.
//! Block static/TLS scalar and array storage uses one global and one first-entry guard.
use super::{Error, data, function::Function, types};
use crate::{
    lowir::{
        model::{DataItem, Facts, Global, GlobalInit, InstructionKind as I, Value},
        vocabulary::{Compare, Storage, Type},
    },
    sema::lowering::{EntityId, StaticValue, TypeView},
    syntax::ast,
};
#[derive(Clone, Copy)]
pub(super) struct StorageSymbols {
    pub object: crate::lowir::model::SymbolId,
    pub metadata: crate::lowir::model::Metadata,
    pub guard: Option<(crate::lowir::model::SymbolId, crate::lowir::model::Metadata)>,
}
pub(super) struct StaticTemporaries {
    pub reference: bool,
    pub made: Vec<super::shutdown::Finalizer>,
}
#[derive(Clone, Copy)]
pub(super) enum GuardCall {
    Acquire,
    Release,
    Abort,
}
impl GuardCall {
    pub(super) fn index(self) -> usize {
        match self {
            Self::Acquire => 0,
            Self::Release => 1,
            Self::Abort => 2,
        }
    }
    pub(super) fn result(self) -> Type {
        match self {
            Self::Acquire => Type::I32,
            Self::Release | Self::Abort => Type::Void,
        }
    }
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn guard_call(
        &mut self,
        call: GuardCall,
        guard: crate::lowir::model::SymbolId,
        ty: crate::sema::lowering::TypeId,
    ) -> Result<(), Error> {
        self.emit_guard_call(call, guard, ty).map(|_| ())
    }
    fn emit_guard_call(
        &mut self,
        call: GuardCall,
        guard: crate::lowir::model::SymbolId,
        ty: crate::sema::lowering::TypeId,
    ) -> Result<Value, Error> {
        let address = self.emit(I::Addr(Value::Symbol(guard)), ty)?.value;
        let callee = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .guard_function(self.builder.program(), call)?;
        let arguments = self.builder.arguments(&[address])?;
        self.before_lifetime_call(false)?;
        let instruction = I::Call(crate::lowir::model::Call {
            result: call.result(),
            callee: Value::Symbol(callee),
            arguments,
            signature: None,
            copy_elision: false,
            argument_classes: Vec::new(),
        });
        let value = if call.result() == Type::Void {
            self.effect(instruction)?;
            super::function::RValue {
                value: self.builder.integer(0)?,
                ty: Type::Void,
                source: ty,
                literal: false,
                bits: None,
            }
        } else {
            self.emit(instruction, ty)?
        };
        Ok(self.finish_lifetime_call(value, false)?.value)
    }
    fn local_static_has_temporaries(
        &mut self,
        entity: EntityId,
        ty: crate::sema::lowering::TypeId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(Option<ast::ExprId>, bool), Error> {
        let listed = if self.graph.lower_construction(entity).is_none()
            && !matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. })
        {
            self.list_variable(initializer)?
        } else {
            None
        };
        let temporary = listed.is_some()
            || if let Some(initializer) = initializer {
                if matches!(self.graph.lower_type(ty)?, TypeView::Reference { .. }) {
                    let expr = self.initializer_expression(initializer)?;
                    !self.graph.lower_expression(expr)?.is_place
                        || self.initializer_temporaries(Some(initializer))?
                } else {
                    self.initializer_temporaries(Some(initializer))?
                }
            } else {
                false
            };
        Ok((listed, temporary))
    }
    fn local_static_is_dynamic(
        &self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        aggregate: bool,
        destroyed: bool,
    ) -> Result<bool, Error> {
        let facts = self.graph.lower_variable(entity)?;
        let mut element = facts.ty;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        Ok(destroyed
            || if aggregate {
                self.graph.lower_array_image(entity).is_none()
                    && self.graph.lower_static_construction(entity).is_none()
                    && (initializer.is_some()
                        || matches!(self.graph.lower_type(element)?, TypeView::Class(_))
                            && !self.graph.lower_trivial_default_construction(entity)?)
            } else {
                initializer.is_some() && facts.initializer.is_none()
            })
    }
    pub(super) fn local_static(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_variable(entity)?;
        if facts.name.is_none() && self.graph.lower_decomposition(entity).is_none() {
            return self.anonymous_static(entity);
        }
        let aggregate = matches!(
            self.graph.lower_type(facts.ty)?,
            TypeView::Array { .. } | TypeView::Class(_)
        );
        let destroyed = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .storage_destructor(facts.ty)?
            .is_some();
        let dynamic = self.local_static_is_dynamic(entity, initializer, aggregate, destroyed)?;
        let (listed, temporary) =
            self.local_static_has_temporaries(entity, facts.ty, initializer)?;
        let ordinal = self.next_static;
        self.next_static = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        let StorageSymbols {
            object: symbol,
            guard,
            metadata,
        } = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .local_storage_symbols(
                self.builder.program(),
                entity,
                self.entity
                    .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                        "implicit constructor with static local",
                    )))?,
                ordinal,
                dynamic,
                tokens.filter(|_| temporary),
            )?;
        let saved = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .replace_temporary_owner(Some((entity, 0)));
        let defined = self.define_static_storage(
            entity,
            initializer,
            aggregate,
            dynamic,
            StorageSymbols {
                object: symbol,
                guard,
                metadata,
            },
        );
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .replace_temporary_owner(saved);
        defined?;
        self.locals.insert(entity, Value::Symbol(symbol));
        let Some((guard, _)) = guard else {
            if self.graph.lower_static_construction(entity).is_some()
                && !self.graph.lower_evaluated_construction(entity)
            {
                return Ok(());
            }
            if let Some(constructor) = self.graph.lower_construction(entity)
                && !self.graph.lower_constructor_trivial(constructor)
            {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .demand_entry(super::construction::Entry::Constructor(
                        constructor,
                        super::construction::Variant::Complete,
                    ));
            }
            if self.graph.lower_evaluated_construction(entity) {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .demand_constant_constructions(&*self.graph, &*self.tree, initializer);
            }
            for &constructor in self
                .graph
                .lower_initializer_constructors(entity)
                .iter()
                .rev()
            {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .demand_entry(super::construction::Entry::Constructor(
                        constructor,
                        super::construction::Variant::Complete,
                    ));
            }
            return Ok(());
        };
        if self.terminated {
            return Ok(());
        }
        let flag = Value::Symbol(guard);
        let ready = self.new_block("local_static_ready")?;
        let initialize = self.new_block("local_static_init")?;
        let thread_safe = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .host_objects()
            && !facts.thread_local;
        let aborting = self.begin_static_guard(guard, facts.ty, ready, initialize, thread_safe)?;
        let address = self.emit(I::Addr(Value::Symbol(symbol)), facts.ty)?.value;
        let mut first = self.lifetime.cleanups.len();
        let persistent = self.persistent_references;
        self.persistent_references = true;
        let previous = self.static_temporaries.replace(StaticTemporaries {
            reference: matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. }),
            made: Vec::new(),
        });
        if let Some(list) = listed
            && !self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode()
            && let Some(array) = self.graph.lower_initializer_array(list)
            && matches!(self.graph.lower_type(array)?, TypeView::Array { bound, .. } if bound != 0)
        {
            self.generated_slot("initlist", types::storage(&*self.graph, array)?)?;
        }
        let saved = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .replace_temporary_owner(Some((entity, 0)));
        let mut region = false;
        let construction = self.lifetime.construction;
        let construction_flow = self.lifetime.construction_flow;
        let construction_start = self.lifetime.construction_start;
        self.lifetime.construction |= aggregate;
        let root = initializer.and_then(|initializer| match initializer.kind {
            ast::InitializerKind::Assign(id) | ast::InitializerKind::Braced(id) => Some(id),
            ast::InitializerKind::Parenthesized(args) => {
                let arguments = self.tree.expr_lists.get(args);
                (arguments.len() == 1).then(|| arguments[0])
            }
            _ => None,
        });
        let result = self.with_full_root(root, |this| {
            if aggregate {
                let temporaries = this.local_initializer_temporaries(initializer, facts.ty)?;
                region = this.begin_local_initializer(entity, initializer, true, temporaries)?;
                if !region && !this.lifetime.full_expression {
                    this.lifetime.construction_flow = this.initializer_branches(initializer)?;
                    this.lifetime.construction_start =
                        Some((this.builder.position()?, this.lifetime.cleanups.len()));
                }
            }
            match this.graph.lower_type(facts.ty)? {
                TypeView::Array { .. } => {
                    this.local_array(entity, facts.ty, address, initializer)?;
                }
                TypeView::Class(_) => {
                    this.local_aggregate(entity, facts.ty, address, initializer)?;
                }
                TypeView::Reference { .. } => {
                    let written = initializer.ok_or(crate::sema::Error::Invariant(
                        "static reference without initializer",
                    ))?;
                    let expr = this.initializer_expression(written)?;
                    region = this.begin_full_expression(
                        Some(expr),
                        super::temporary_scan::ExpressionUse::Ordinary,
                    )?;
                    let target = this.bind_static_reference(expr, facts.ty)?;
                    this.effect(I::Store(
                        Facts::memory(false, true),
                        Type::Ptr,
                        target,
                        address,
                    ))?;
                }
                _ => {
                    let expr = initializer
                        .map(|written| this.initializer_expression(written))
                        .transpose()?;
                    region = this.begin_full_expression(
                        expr,
                        super::temporary_scan::ExpressionUse::Ordinary,
                    )?;
                    this.initialize_scalar(facts.ty, address, initializer)?;
                }
            }
            Ok::<(), Error>(())
        });
        self.persistent_references = persistent;
        self.symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .replace_temporary_owner(saved);
        let temporaries = std::mem::replace(&mut self.static_temporaries, previous).ok_or(
            crate::sema::Error::Invariant("local static lost temporary inventory"),
        )?;
        result?;
        if self.lifetime.tracking.active {
            self.finish_tracking()?;
        }
        if let Some(aborting) = aborting {
            self.complete_static_guard(aborting)?;
            first -= 1;
        }
        if !temporaries.made.is_empty() {
            self.register_static_finalizers(symbol, &temporaries.made, facts.ty)?;
        }
        if destroyed {
            let thread_local = facts.thread_local
                && self
                    .symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .object_mode();
            self.register_destruction(symbol, facts.ty, thread_local)?;
        }
        if thread_safe {
            self.guard_call(GuardCall::Release, guard, facts.ty)?;
        } else {
            let one = self.builder.integer(1)?;
            self.effect(I::Store(Facts::memory(false, true), Type::I64, one, flag))?;
        }
        // Initialization has completed before its initializer temporaries are
        // destroyed. Their failure must not retry or lose static destruction.
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(region)?;
        self.close_lifetime_region()?;
        // local_object's construction scope includes static completion: the
        // registrar's scalar result is staged while this region is active.
        self.lifetime.construction = construction;
        self.lifetime.construction_flow = construction_flow;
        self.lifetime.construction_start = construction_start;
        self.jump(ready)?;
        self.place(ready)
    }

    fn complete_static_guard(&mut self, aborting: usize) -> Result<(), Error> {
        // Retire the acquired guard before registering destruction or running
        // initializer temporaries. Later failures must not abort this guard.
        self.close_lifetime_region()?;
        self.lifetime.cleanups.remove(aborting);
        if self.lifetime.tracking.floor > aborting {
            self.lifetime.tracking.floor -= 1;
        }
        if let super::lifetime::TemporarySegment::Nothrow { floor } =
            &mut self.lifetime.temporary_segment
            && *floor > aborting
        {
            *floor -= 1;
        }
        for cleanup in self.lifetime.cleanups.iter_mut().skip(aborting) {
            cleanup.clear_unwind_memos();
        }
        Ok(())
    }

    fn begin_static_guard(
        &mut self,
        guard: crate::lowir::model::SymbolId,
        ty: crate::sema::lowering::TypeId,
        ready: crate::lowir::model::BlockId,
        initialize: crate::lowir::model::BlockId,
        thread_safe: bool,
    ) -> Result<Option<usize>, Error> {
        let flag = Value::Symbol(guard);
        if thread_safe {
            let acquire = self.new_block("local_static_ctor_run")?;
            let address = self.emit(I::Addr(flag), ty)?.value;
            let loaded = self.emit(I::Load(Facts::default(), Type::I8, address), ty)?;
            let zero = self.builder.integer(0)?;
            let test = self.emit(I::Compare(Compare::Ne, Type::I8, loaded.value, zero), ty)?;
            self.effect(I::Branch(test.value, ready, acquire))?;
            self.terminated = true;
            self.place(acquire)?;
            let acquired = self.emit_guard_call(GuardCall::Acquire, guard, ty)?;
            let zero = self.builder.integer(0)?;
            let test = self.emit(I::Compare(Compare::Ne, Type::I32, acquired, zero), ty)?;
            self.effect(I::Branch(test.value, initialize, ready))?;
            self.terminated = true;
            self.place(initialize)?;
            let aborting = self.lifetime.cleanups.len();
            let mut cleanup = super::lifetime::Cleanup::temporary(ty, flag);
            cleanup.place = super::lifetime::CleanupPlace::AbortGuard(guard);
            self.lifetime.cleanups.push(cleanup);
            let split = self.lifetime.region;
            self.close_lifetime_region()?;
            if split {
                let position = self.builder.position()?;
                self.open_temporary_region_at(Some(position))?;
            } else {
                self.lifetime.region_start = super::lifetime::RegionStart::Consumed;
                self.lifetime.storage_split = Some(self.builder.position()?);
            }
            Ok(Some(aborting))
        } else {
            let loaded = self.emit(I::Load(Facts::memory(false, true), Type::I64, flag), ty)?;
            let zero = self.builder.integer(0)?;
            let test = self.emit(I::Compare(Compare::Ne, Type::I64, loaded.value, zero), ty)?;
            self.effect(I::Branch(test.value, ready, initialize))?;
            self.terminated = true;
            self.place(initialize)?;
            Ok(None)
        }
    }

    fn define_static_storage(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        aggregate: bool,
        dynamic: bool,
        symbols: StorageSymbols,
    ) -> Result<(), Error> {
        let StorageSymbols {
            object: symbol,
            guard,
            mut metadata,
        } = symbols;
        let program = self.builder.program();
        if program.global(program.symbol(symbol)).is_some() {
            return Ok(());
        }
        let facts = self.graph.lower_variable(entity)?;
        // A static-construction attempt can publish base tables and symbolic
        // addresses even when its final image replaces them or it fails.
        // Replay that demand before defining the surviving storage image.
        let demands = self
            .graph
            .lower_static_construction_demands(entity)
            .to_vec();
        self.static_data_items(&demands)?;
        let mut init = if aggregate {
            GlobalInit::Data(
                if let Some(image) = self
                    .graph
                    .lower_static_construction(entity)
                    .or_else(|| self.graph.lower_array_image(entity))
                {
                    {
                        let image = image.to_vec();
                        self.static_data_items(&image)?
                    }
                } else {
                    vec![DataItem::Zero(
                        i64::try_from(types::layout(&*self.graph, facts.ty)?.0)
                            .map_err(|_| crate::lowir::Error::Limit)?,
                    )]
                },
            )
        } else if facts.reference_temporary.is_some() && facts.initializer.is_some() {
            GlobalInit::Address(self.static_reference_temporary(entity, false)?, 0)
        } else if let Some(value) = facts.initializer.filter(|_| initializer.is_some()) {
            match value {
                value if data::soft_item(value).is_some() => {
                    let crate::lowir::model::DataItem::Constant(_, constant) =
                        data::soft_item(value)
                            .ok_or(crate::sema::Error::Invariant("soft static local item"))?
                    else {
                        return Err(crate::sema::Error::Invariant("soft local constant").into());
                    };
                    GlobalInit::Constant(constant)
                }
                StaticValue::MemberPointer(_) => self
                    .static_data_items(&[crate::sema::lowering::StaticItem::Scalar(
                        facts.ty, value,
                    )])
                    .map(GlobalInit::Data)?,
                StaticValue::Address(..)
                | StaticValue::Literal(..)
                | StaticValue::Temporary(..)
                | StaticValue::TypeInformation(..) => {
                    let (target, offset) = self.static_data_address(value)?;
                    GlobalInit::Address(target, offset)
                }
                _ => GlobalInit::Constant(data::constant(value)?),
            }
        } else {
            GlobalInit::Zero
        };
        if let GlobalInit::Data(items) = &mut init {
            super::null_initialization::static_nulls(&*self.graph, facts.ty, items)?;
        } else if matches!(init, GlobalInit::Zero)
            && self.graph.lower_member_pointer_type(facts.ty).is_some()
            && self.graph.lower_holds_data_member_pointer(facts.ty)?
        {
            init = GlobalInit::Constant(crate::lowir::literals::Constant::Integer(u128::MAX));
        }
        if !dynamic
            && matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
            && self.graph.lower_reference_reads_storage(entity)
        {
            self.symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .demand_reference_initializer_calls(&*self.graph, &*self.tree, initializer)?;
        }
        if !dynamic
            && !facts.thread_local
            && self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .readonly_object(facts.ty)?
        {
            metadata.storage = Some(Storage::ReadOnly);
        }
        let program = self.builder.program();
        let name = program.symbol(symbol).name;
        let mut ty = if let GlobalInit::Data(items) = &init {
            super::globals::structured_type(
                &*self.graph,
                facts.ty,
                items,
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .object_mode(),
            )?
        } else {
            Some(match facts.initializer.and_then(data::soft_item) {
                Some(crate::lowir::model::DataItem::Constant(ty, _)) => ty,
                _ => types::storage(&*self.graph, facts.ty)?,
            })
        };
        super::globals::align_storage(
            &*self.graph,
            entity,
            &mut ty,
            &mut init,
            self.symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .object_mode(),
        )?;
        program.add_global(
            symbol,
            Global {
                name,
                ty,
                metadata,
                init,
            },
        )?;
        if let Some((guard, metadata)) = guard {
            let name = program.symbol(guard).name;
            program.add_global(
                guard,
                Global {
                    name,
                    ty: Some(Type::I64),
                    metadata,
                    init: GlobalInit::Zero,
                },
            )?;
        }
        Ok(())
    }

    pub(super) fn register_static_destruction(
        &mut self,
        global: crate::lowir::model::SymbolId,
        ty: crate::sema::lowering::TypeId,
    ) -> Result<(), Error> {
        self.register_destruction(global, ty, false)
    }
    pub(super) fn register_destruction(
        &mut self,
        global: crate::lowir::model::SymbolId,
        ty: crate::sema::lowering::TypeId,
        thread: bool,
    ) -> Result<(), Error> {
        let Some(finalizer) = super::shutdown::for_type(
            self.builder.program(),
            &mut self.symbols.view(&*self.graph, &*self.tree, self.cursor),
            &*self.graph,
            global,
            ty,
            None,
        )?
        else {
            return Ok(());
        };
        self.register_finalizers(global, &[finalizer], ty, thread)
    }
    fn register_static_finalizers(
        &mut self,
        global: crate::lowir::model::SymbolId,
        finalizers: &[super::shutdown::Finalizer],
        ty: crate::sema::lowering::TypeId,
    ) -> Result<(), Error> {
        self.register_finalizers(global, finalizers, ty, false)
    }
    fn register_finalizers(
        &mut self,
        global: crate::lowir::model::SymbolId,
        finalizers: &[super::shutdown::Finalizer],
        ty: crate::sema::lowering::TypeId,
        thread: bool,
    ) -> Result<(), Error> {
        use crate::lowir::model::Call;
        let program = self.builder.program();
        let destructor = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .local_destructor_symbol(program, global)?;
        super::shutdown::local(program, destructor, finalizers)?;
        program.stream_last_definition(&mut self.symbols.generated_definitions)?;
        let (registrar, arguments) = if thread {
            let dso = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .dso_handle(program)?;
            let registrar = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .thread_atexit_function(program)?;
            let zero = self.builder.integer(0)?;
            let arguments =
                self.builder
                    .arguments(&[Value::Symbol(destructor), zero, Value::Symbol(dso)])?;
            (registrar, arguments)
        } else {
            let registrar = self
                .symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .atexit_function(program)?;
            (
                registrar,
                self.builder.arguments(&[Value::Symbol(destructor)])?,
            )
        };
        self.before_lifetime_call(false)?;
        let value = self.emit(
            I::Call(Call {
                result: Type::I32,
                callee: Value::Symbol(registrar),
                arguments,
                signature: None,
                copy_elision: false,
                argument_classes: Vec::new(),
            }),
            ty,
        )?;
        self.finish_lifetime_call(value, false)?;
        Ok(())
    }

    fn anonymous_static(&mut self, entity: EntityId) -> Result<(), Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let dynamic = !self.graph.lower_trivial_default_construction(entity)?
            && self.graph.lower_array_image(entity).is_none();
        let ordinal = self.next_static;
        self.next_static = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        let symbols = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .local_storage_symbols(
                self.builder.program(),
                entity,
                self.entity.ok_or(crate::sema::Error::Invariant(
                    "static object without function",
                ))?,
                ordinal,
                dynamic,
                None,
            )?;
        let initialized = {
            let program = self.builder.program();
            program.global(program.symbol(symbols.object)).is_some()
        };
        if !initialized {
            let items = if let Some(image) = self.graph.lower_array_image(entity) {
                {
                    let image = image.to_vec();
                    self.static_data_items(&image)?
                }
            } else {
                vec![DataItem::Zero(
                    i64::try_from(types::layout(&*self.graph, ty)?.0)
                        .map_err(|_| crate::lowir::Error::Limit)?,
                )]
            };
            let program = self.builder.program();
            let name = program.symbol(symbols.object).name;
            program.add_global(
                symbols.object,
                Global {
                    name,
                    ty: super::globals::structured_type(
                        &*self.graph,
                        ty,
                        &items,
                        self.symbols
                            .view(&*self.graph, &*self.tree, self.cursor)
                            .object_mode(),
                    )?,
                    metadata: symbols.metadata,
                    init: GlobalInit::Data(items),
                },
            )?;
            if let Some((guard, metadata)) = symbols.guard {
                let name = program.symbol(guard).name;
                program.add_global(
                    guard,
                    Global {
                        name,
                        ty: Some(Type::I64),
                        metadata,
                        init: GlobalInit::Zero,
                    },
                )?;
            }
        }
        self.locals.insert(entity, Value::Symbol(symbols.object));
        if self.terminated {
            return Ok(());
        }
        if let Some((guard, _)) = symbols.guard {
            let ready = self.new_block("local_static_ready")?;
            let initialize = self.new_block("local_static_init")?;
            let flag = self.emit(
                I::Load(Facts::memory(false, true), Type::I64, Value::Symbol(guard)),
                ty,
            )?;
            let zero = self.builder.integer(0)?;
            let condition = self.emit(I::Compare(Compare::Ne, Type::I64, flag.value, zero), ty)?;
            self.effect(I::Branch(condition.value, ready, initialize))?;
            self.place(initialize)?;
            let address = self.emit(I::Addr(Value::Symbol(symbols.object)), ty)?.value;
            self.local_aggregate(entity, ty, address, None)?;
            self.close_lifetime_region()?;
            let one = self.builder.integer(1)?;
            self.effect(I::Store(
                Facts::memory(false, true),
                Type::I64,
                one,
                Value::Symbol(guard),
            ))?;
            self.jump(ready)?;
            self.place(ready)?;
        } else if let Some(constructor) = self.graph.lower_construction(entity)
            && !self.graph.lower_constructor_trivial(constructor)
        {
            self.symbols
                .view(&*self.graph, &*self.tree, self.cursor)
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    super::construction::Variant::Complete,
                ));
        }
        Ok(())
    }
}
