// Translated from lower/program_lowering.cpp source queries. See NOTICE.
use super::{Error, Symbols};
use crate::{sema::lowering::SourceLocation, support::source::SourceOffset};

impl Symbols<'_, '_> {
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
