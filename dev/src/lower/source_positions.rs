// Translated from lower/program_lowering.cpp source queries. See NOTICE.
use super::{Error, Symbols};
use crate::{sema::lowering::SourceLocation, support::source::SourceOffset};

impl Symbols<'_, '_> {
    pub(in crate::lower) fn locate_definition(
        &self,
        builder: &mut crate::lowir::builder::FunctionBuilder<'_>,
        definition: Option<crate::syntax::ast::DeclId>,
    ) -> Result<(), Error> {
        if let Some(definition) = definition {
            let location = self.debug_location(
                builder.program(),
                self.tree.declarations.get(definition).location,
            )?;
            builder.definition_location(location);
        }
        Ok(())
    }

    pub(in crate::lower) fn debug_locations(&self) -> bool {
        self.table.debug_locations == super::DebugLocations::Enabled && !self.table.object_mode
    }
    pub(in crate::lower) fn debug_location(
        &self,
        program: &mut crate::lowir::model::Program,
        location: SourceOffset,
    ) -> Result<Option<crate::lowir::model::DebugLocation>, Error> {
        if !self.debug_locations() {
            return Ok(None);
        }
        let input = self.cursor.post.input();
        let file = input.source_file(location);
        if file.is_empty() || file.iter().any(|byte| b" \t\n\r,()".contains(byte)) {
            return Ok(None);
        }
        let line = self.source_line(location)?;
        let column = input.source_column(location);
        let Ok(line) = u32::try_from(line) else {
            return Ok(None);
        };
        if line == 0 || column == 0 {
            return Ok(None);
        }
        Ok(Some(crate::lowir::model::DebugLocation {
            file: program.names.intern_bytes(&file)?,
            line,
            column,
        }))
    }

    pub(in crate::lower) fn predefined_variable(
        &mut self,
        program: &mut crate::lowir::model::Program,
        entity: crate::sema::lowering::EntityId,
    ) -> Result<Option<crate::lowir::model::Value>, Error> {
        let Some(text) = self.graph.lower_predefined_variable(entity) else {
            return Ok(None);
        };
        if let Some(&symbol) = self.table.predefined_variables.get(&entity) {
            return Ok(Some(crate::lowir::model::Value::Symbol(symbol)));
        }
        let mut units = self.data.take_string_units();
        units.extend(
            self.cursor
                .name(text)
                .iter()
                .copied()
                .take_while(|&byte| byte != 0)
                .map(u64::from),
        );
        units.push(0);
        let symbol = crate::lower::data::string_object(
            program,
            self,
            crate::lowir::vocabulary::Type::I8,
            units,
        )?;
        self.table.predefined_variables.insert(entity, symbol);
        Ok(Some(crate::lowir::model::Value::Symbol(symbol)))
    }

    pub(in crate::lower) fn source_line(&self, location: SourceOffset) -> Result<u64, Error> {
        let line = self
            .cursor
            .post
            .input()
            .source_line(location)
            .map_err(|_| crate::sema::Error::Invariant("invalid source position"))?;
        u64::try_from(line)
            .map_err(|_| crate::sema::Error::Invariant("negative source line").into())
    }
    pub(in crate::lower) fn source_text(
        &self,
        kind: SourceLocation,
        location: SourceOffset,
        function: Option<crate::sema::lowering::SourceFunction>,
    ) -> Result<Vec<u8>, Error> {
        Ok(if kind == SourceLocation::File {
            self.cursor.post.input().source_file(location).to_vec()
        } else if let Some(function) = function {
            self.graph.lower_predefined_function_name(
                self.cursor.post.input().identifiers(),
                function,
                self.budget,
                self.stack_base,
            )?
        } else {
            Vec::new()
        })
    }
}
