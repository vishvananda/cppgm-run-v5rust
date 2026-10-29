// Translated from lower/virtual_base_lowering.cpp's body state. See NOTICE.
//! Each ABI entry owns its VTT parameter, block-local row cache and base-call context.
use super::{Error, function::Function};
use crate::support::hash::HashMap;
use crate::support::hash::HashSet;
use crate::{
    lowir::{
        model::{Facts, InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::{EntityId, TypeId},
};
#[derive(Default)]
pub(super) struct FunctionContext {
    pub class: Option<EntityId>,
    pub vtt: Option<Value>,
    pub hidden: Option<usize>,
    pub pending: bool,
    pub this_values: HashSet<Value>,
    table: Option<(crate::lowir::builder::Position, Value)>,
    rows: HashMap<EntityId, Value>,
}
impl Function<'_, '_, '_, '_> {
    pub(super) fn construction_table_value(&mut self, source: TypeId) -> Result<Value, Error> {
        let block = self.builder.position()?;
        if !self.terminated
            && let Some((known, table)) = self.virtual_context.table
            && self.builder.same_block(known)
        {
            return Ok(table);
        }
        let vtt = self
            .virtual_context
            .vtt
            .ok_or(crate::sema::Error::Invariant(
                "construction table without VTT",
            ))?;
        let table = self
            .emit(I::Load(Facts::memory(false, true), Type::Ptr, vtt), source)?
            .value;
        self.virtual_context.table = Some((block, table));
        self.virtual_context.rows.clear();
        Ok(table)
    }
    pub(super) fn construction_row(
        &mut self,
        source: TypeId,
        class: EntityId,
        base: EntityId,
    ) -> Result<Value, Error> {
        let table = self.construction_table_value(source)?;
        if let Some(&row) = self.virtual_context.rows.get(&base) {
            return Ok(row);
        }
        let offset = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .vbase_row_offset(class, base)?;
        let offset = self
            .builder
            .integer(u128::from_le_bytes(i128::from(offset).to_le_bytes()))?;
        let row = self
            .emit(I::Index(Type::I8, None, table, offset), source)?
            .value;
        let value = self
            .emit(I::Load(Facts::memory(false, true), Type::I64, row), source)?
            .value;
        self.virtual_context.rows.insert(base, value);
        Ok(value)
    }
    pub(super) fn hidden_vtt_value(&mut self) -> Result<Value, Error> {
        let index = self
            .virtual_context
            .hidden
            .ok_or(crate::sema::Error::Invariant(
                "base entry called outside construction",
            ))?;
        let class = self
            .virtual_context
            .class
            .ok_or(crate::sema::Error::Invariant("base entry outside class"))?;
        let source = self.graph.lower_entity_type(class)?;
        let object = if let Some(vtt) = self.virtual_context.vtt {
            if index == 0 {
                return Ok(vtt);
            }
            vtt
        } else {
            let symbol = crate::lower::unit_frontend::Context {
                graph: self.graph,
                tree: self.tree,
                cursor: self.cursor,
                symbols: self.symbols,
            }
            .vtt_symbol(self.builder.program(), class)?;
            self.emit(I::Addr(Value::Symbol(symbol)), source)?.value
        };
        let offset = u64::try_from(index)
            .map_err(|_| crate::lowir::Error::Limit)?
            .wrapping_mul(8);
        let offset = self.builder.integer(u128::from(offset))?;
        Ok(self
            .emit(I::Index(Type::I8, None, object, offset), source)?
            .value)
    }
}
