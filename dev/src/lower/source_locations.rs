// Translated from lower/intrinsic_lowering.cpp::source_location. See NOTICE.
//! Queries use the evaluation site's source map and the ordinary string pool.
use super::{
    Error,
    function::{Function, RValue},
};
use crate::{
    lowir::{
        model::{InstructionKind as I, Value},
        vocabulary::Type,
    },
    sema::lowering::SourceLocation,
    support::source::SourceOffset,
    syntax::ast,
};

impl Function<'_, '_, '_, '_> {
    pub(super) fn with_debug_location<T>(
        &mut self,
        location: SourceOffset,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        if !self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .debug_locations()
        {
            return action(self);
        }
        let saved = self.builder.current_location();
        if let Some(here) = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .debug_location(self.builder.program(), location)?
        {
            self.builder.set_current_location(Some(here));
        }
        let result = action(self);
        self.builder.set_current_location(saved);
        result
    }

    pub(super) fn with_source_site<T>(
        &mut self,
        site: Option<SourceOffset>,
        action: impl FnOnce(&mut Self) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let saved = self.source_site;
        if saved.is_none() {
            self.source_site = site.filter(|site| site.0 != 0);
        }
        let result = action(self);
        self.source_site = saved;
        result
    }
    pub(super) fn source_location(
        &mut self,
        id: ast::ExprId,
        kind: SourceLocation,
    ) -> Result<RValue, Error> {
        let location = self
            .source_site
            .unwrap_or(self.tree.expressions.get(id).location);
        let source = self.graph.lower_expression(id)?.ty;
        if matches!(kind, SourceLocation::Line | SourceLocation::Column) {
            let bits = if kind == SourceLocation::Line {
                self.symbols
                    .view(&*self.graph, &*self.tree, self.cursor)
                    .source_line(location)?
            } else {
                0
            };
            return self.runtime_integer(Type::I32, bits, source);
        }
        let text = self
            .symbols
            .view(&*self.graph, &*self.tree, self.cursor)
            .source_text(kind, location, self.name_context)?;
        let mut units = self.symbols.data.take_string_units();
        // The frozen predefined_name accepts a C string. Constant text
        // literals retain embedded nulls; this runtime path stops at one.
        units.extend(
            text.into_iter()
                .take_while(|&byte| byte != 0)
                .map(u64::from),
        );
        units.push(0);
        let symbol = super::data::string_object(
            self.builder.program(),
            &mut self.symbols.view(&*self.graph, &*self.tree, self.cursor),
            Type::I8,
            units,
        )?;
        self.emit(I::Addr(Value::Symbol(symbol)), source)
    }
}

impl super::construction::Entry {
    pub(super) fn source_function(
        self,
        graph: &crate::sema::Graph,
    ) -> Option<crate::sema::lowering::SourceFunction> {
        use crate::sema::lowering::SourceFunction;
        let entity = self.entity(graph);
        if let Self::Constructor(constructor, _) = self {
            Some(entity.map_or_else(
                || SourceFunction::ImplicitConstructor(graph.lower_constructor(constructor).class),
                SourceFunction::Function,
            ))
        } else {
            entity.map(SourceFunction::Function)
        }
    }
}
