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
        let init = if array {
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
        let ty = if let GlobalInit::Data(items) = &init {
            super::globals::structured_type(self.graph, facts.ty, items)?
        } else {
            Some(types::storage(self.graph, facts.ty)?)
        };
        program.add_global(
            symbol,
            Global {
                name,
                ty,
                metadata,
                init,
            },
        )?;
        self.locals.insert(entity, Value::Symbol(symbol));
        let Some((guard, metadata)) = guard else {
            return Ok(());
        };
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
        let one = self.builder.integer(1)?;
        self.effect(I::Store(Facts::memory(false, true), Type::I64, one, flag))?;
        self.jump(ready)?;
        self.place(ready)
    }
}
