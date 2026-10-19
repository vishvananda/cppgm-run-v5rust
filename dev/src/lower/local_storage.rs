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
impl Function<'_, '_, '_, '_> {
    pub(super) fn local_static(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        tokens: Option<ast::TokenSpan<u32>>,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_variable(entity)?;
        if facts.name.is_none() {
            return self.anonymous_static(entity);
        }
        let aggregate = matches!(
            self.graph.lower_type(facts.ty)?,
            TypeView::Array { .. } | TypeView::Class(_)
        );
        let destroyed = self.symbols.storage_destructor(facts.ty)?.is_some();
        let mut element = facts.ty;
        while let TypeView::Array { element: inner, .. } = self.graph.lower_type(element)? {
            element = inner;
        }
        let dynamic = destroyed
            || if aggregate {
                self.graph.lower_array_image(entity).is_none()
                    && self.graph.lower_static_construction(entity).is_none()
                    && (initializer.is_some()
                        || matches!(self.graph.lower_type(element)?, TypeView::Class(_))
                            && !self.graph.lower_trivial_default_construction(entity)?)
            } else {
                initializer.is_some() && facts.initializer.is_none()
            };
        let listed = if self.graph.lower_construction(entity).is_none()
            && !matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
        {
            self.list_variable(initializer)?
        } else {
            None
        };
        let temporary = listed.is_some()
            || if let Some(initializer) = initializer {
                if matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. }) {
                    let expr = self.initializer_expression(initializer)?;
                    !self.graph.lower_expression(expr)?.is_place
                        || self.initializer_temporaries(Some(initializer))?
                } else {
                    self.initializer_temporaries(Some(initializer))?
                }
            } else {
                false
            };
        let ordinal = self.next_static;
        self.next_static = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        let StorageSymbols {
            object: symbol,
            guard,
            metadata,
        } = self.symbols.local_storage_symbols(
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
        let saved = self.symbols.replace_temporary_owner(Some((entity, 0)));
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
        self.symbols.replace_temporary_owner(saved);
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
                    .demand_entry(super::construction::Entry::Constructor(
                        constructor,
                        super::construction::Variant::Complete,
                    ));
            }
            if self.graph.lower_evaluated_construction(entity) {
                self.symbols
                    .demand_constant_constructions(self.graph, self.tree, initializer);
            }
            for &constructor in self
                .graph
                .lower_initializer_constructors(entity)
                .iter()
                .rev()
            {
                self.symbols
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
        let loaded = self.emit(
            I::Load(Facts::memory(false, true), Type::I64, flag),
            facts.ty,
        )?;
        let zero = self.builder.integer(0)?;
        let test = self.emit(
            I::Compare(Compare::Ne, Type::I64, loaded.value, zero),
            facts.ty,
        )?;
        self.effect(I::Branch(test.value, ready, initialize))?;
        self.terminated = true;
        self.place(initialize)?;
        let address = self.emit(I::Addr(Value::Symbol(symbol)), facts.ty)?.value;
        let first = self.lifetime.cleanups.len();
        let persistent = self.persistent_references;
        self.persistent_references = true;
        let previous = self.static_temporaries.replace(StaticTemporaries {
            reference: matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. }),
            made: Vec::new(),
        });
        if let Some(list) = listed
            && let Some(array) = self.graph.lower_initializer_array(list)
            && matches!(self.graph.lower_type(array)?, TypeView::Array { bound, .. } if bound != 0)
        {
            self.generated_slot("initlist", types::storage(self.graph, array)?)?;
        }
        let saved = self.symbols.replace_temporary_owner(Some((entity, 0)));
        let mut region = false;
        let construction = self.lifetime.construction;
        self.lifetime.construction |= aggregate;
        let result = (|| {
            if aggregate {
                let temporaries = self.local_initializer_temporaries(initializer, facts.ty)?;
                region = self.begin_local_initializer(entity, initializer, true, temporaries)?;
            }
            match self.graph.lower_type(facts.ty)? {
                TypeView::Array { .. } => {
                    self.local_array(entity, facts.ty, address, initializer)?;
                }
                TypeView::Class(_) => {
                    self.local_aggregate(entity, facts.ty, address, initializer)?;
                }
                TypeView::Reference { .. } => {
                    let written = initializer.ok_or(crate::sema::Error::Invariant(
                        "static reference without initializer",
                    ))?;
                    let expr = self.initializer_expression(written)?;
                    region = self.begin_full_expression(
                        Some(expr),
                        super::temporary_scan::ExpressionUse::Ordinary,
                    )?;
                    let target = self.bind_static_reference(expr, facts.ty)?;
                    self.effect(I::Store(
                        Facts::memory(false, true),
                        Type::Ptr,
                        target,
                        address,
                    ))?;
                }
                _ => {
                    let expr = initializer
                        .map(|written| self.initializer_expression(written))
                        .transpose()?;
                    region = self.begin_full_expression(
                        expr,
                        super::temporary_scan::ExpressionUse::Ordinary,
                    )?;
                    self.initialize_scalar(facts.ty, address, initializer)?;
                }
            }
            Ok::<(), Error>(())
        })();
        self.lifetime.construction = construction;
        self.persistent_references = persistent;
        self.symbols.replace_temporary_owner(saved);
        let temporaries = std::mem::replace(&mut self.static_temporaries, previous).ok_or(
            crate::sema::Error::Invariant("local static lost temporary inventory"),
        )?;
        result?;
        self.destroy_temporaries(first)?;
        self.end_lifetime_expression(region)?;
        if !temporaries.made.is_empty() {
            self.register_static_finalizers(symbol, &temporaries.made, facts.ty)?;
        }
        self.close_lifetime_region()?;
        if destroyed {
            self.register_static_destruction(symbol, facts.ty)?;
        }
        let one = self.builder.integer(1)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, one, flag))?;
        self.jump(ready)?;
        self.place(ready)
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
        let mut init = if aggregate {
            GlobalInit::Data(
                if let Some(image) = self
                    .graph
                    .lower_static_construction(entity)
                    .or_else(|| self.graph.lower_array_image(entity))
                {
                    self.static_data_items(image)?
                } else {
                    vec![DataItem::Zero(
                        i64::try_from(types::layout(self.graph, facts.ty)?.0)
                            .map_err(|_| crate::lowir::Error::Limit)?,
                    )]
                },
            )
        } else if facts.reference_temporary.is_some() && facts.initializer.is_some() {
            GlobalInit::Address(self.static_reference_temporary(entity, false)?, 0)
        } else if let Some(value) = facts.initializer.filter(|_| initializer.is_some()) {
            match value {
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
        if !dynamic
            && matches!(self.graph.lower_type(facts.ty)?, TypeView::Reference { .. })
            && self.graph.lower_reference_reads_storage(entity)
        {
            self.symbols
                .demand_reference_initializer_calls(self.graph, self.tree, initializer)?;
        }
        if !dynamic && !facts.thread_local && self.symbols.readonly_object(facts.ty)? {
            metadata.storage = Some(Storage::ReadOnly);
        }
        let program = self.builder.program();
        let name = program.symbol(symbol).name;
        let mut ty = if let GlobalInit::Data(items) = &init {
            super::globals::structured_type(self.graph, facts.ty, items)?
        } else {
            Some(types::storage(self.graph, facts.ty)?)
        };
        super::globals::align_storage(self.graph, entity, &mut ty, &mut init)?;
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
        let Some(finalizer) = super::shutdown::for_type(
            self.builder.program(),
            self.symbols,
            self.graph,
            global,
            ty,
            None,
        )?
        else {
            return Ok(());
        };
        self.register_static_finalizers(global, &[finalizer], ty)
    }
    fn register_static_finalizers(
        &mut self,
        global: crate::lowir::model::SymbolId,
        finalizers: &[super::shutdown::Finalizer],
        ty: crate::sema::lowering::TypeId,
    ) -> Result<(), Error> {
        use crate::lowir::model::Call;
        let program = self.builder.program();
        let destructor = self.symbols.local_destructor_symbol(program, global)?;
        super::shutdown::local(program, destructor, finalizers)?;
        program.stream_last_definition(&mut self.symbols.generated_definitions)?;
        let registrar = self.symbols.atexit_function(program)?;
        let arguments = self.builder.arguments(&[Value::Symbol(destructor)])?;
        self.before_lifetime_call(false)?;
        let value = self.emit(
            I::Call(Call {
                result: Type::I32,
                callee: Value::Symbol(registrar),
                arguments,
                signature: None,
                copy_elision: false,
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
        let symbols = self.symbols.local_storage_symbols(
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
                self.static_data_items(image)?
            } else {
                vec![DataItem::Zero(
                    i64::try_from(types::layout(self.graph, ty)?.0)
                        .map_err(|_| crate::lowir::Error::Limit)?,
                )]
            };
            let program = self.builder.program();
            let name = program.symbol(symbols.object).name;
            program.add_global(
                symbols.object,
                Global {
                    name,
                    ty: super::globals::structured_type(self.graph, ty, &items)?,
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
                .demand_entry(super::construction::Entry::Constructor(
                    constructor,
                    super::construction::Variant::Complete,
                ));
        }
        Ok(())
    }
}
