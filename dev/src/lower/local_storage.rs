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
impl Function<'_, '_, '_, '_> {
    pub(super) fn local_static(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
    ) -> Result<(), Error> {
        let facts = self.graph.lower_variable(entity)?;
        if facts.name.is_none() {
            return self.anonymous_static(entity);
        }
        let array = matches!(self.graph.lower_type(facts.ty)?, TypeView::Array { .. });
        if matches!(
            self.graph.lower_type(facts.ty)?,
            TypeView::Reference { .. } | TypeView::Class(_) | TypeView::Unsupported
        ) {
            return Err(Error::Missing(
                "static reference and class storage lowering",
            ));
        }
        self.symbols.require_trivial_storage(facts.ty)?;
        let dynamic = initializer.is_some()
            && if array {
                self.graph.lower_array_image(entity).is_none()
            } else {
                facts.initializer.is_none()
            };
        let ordinal = self.next_static;
        self.next_static = ordinal.checked_add(1).ok_or(crate::lowir::Error::Limit)?;
        let StorageSymbols {
            object: symbol,
            guard,
            mut metadata,
        } = self.symbols.local_storage_symbols(
            self.builder.program(),
            entity,
            self.entity
                .ok_or(Error::Semantic(crate::sema::Error::Invariant(
                    "implicit constructor with static local",
                )))?,
            ordinal,
            dynamic,
        )?;
        let defined = {
            let program = self.builder.program();
            program.global(program.symbol(symbol)).is_some()
        };
        if !defined {
            let mut init = if array {
                GlobalInit::Data(
                    if initializer.is_some()
                        && !dynamic
                        && let Some(image) = self.graph.lower_array_image(entity)
                    {
                        data::items(
                            self.builder.program(),
                            self.symbols,
                            self.graph,
                            self.tree,
                            image,
                        )?
                    } else {
                        vec![DataItem::Zero(
                            i64::try_from(types::layout(self.graph, facts.ty)?.0)
                                .map_err(|_| crate::lowir::Error::Limit)?,
                        )]
                    },
                )
            } else if let Some(value) = facts.initializer.filter(|_| initializer.is_some()) {
                match value {
                    StaticValue::Address(..) | StaticValue::Literal(..) => {
                        let (target, offset) = data::address(
                            self.builder.program(),
                            self.symbols,
                            self.graph,
                            self.tree,
                            value,
                        )?;
                        GlobalInit::Address(target, offset)
                    }
                    _ => GlobalInit::Constant(data::constant(value)?),
                }
            } else {
                GlobalInit::Zero
            };
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
        }
        self.locals.insert(entity, Value::Symbol(symbol));
        let Some((guard, _)) = guard else {
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
        if array {
            self.local_array(entity, facts.ty, address, initializer)?;
        } else {
            self.initialize_scalar(facts.ty, address, initializer)?;
        }
        self.close_lifetime_region()?;
        let one = self.builder.integer(1)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, one, flag))?;
        self.jump(ready)?;
        self.place(ready)
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
        )?;
        let initialized = {
            let program = self.builder.program();
            program.global(program.symbol(symbols.object)).is_some()
        };
        if !initialized {
            let items = if let Some(image) = self.graph.lower_array_image(entity) {
                data::items(
                    self.builder.program(),
                    self.symbols,
                    self.graph,
                    self.tree,
                    image,
                )?
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
