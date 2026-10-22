//! Unpublished specialization copies owned by the `LowIR` program. See NOTICE.
use super::{Body, Error, Function, FunctionId, NameId, Program, SymbolDetail, SymbolId};
use crate::lowir::vocabulary::Binding;
impl Program {
    pub(crate) fn inline_copy(
        &mut self,
        source: SymbolId,
        name: NameId,
    ) -> Result<SymbolId, Error> {
        let symbol = self.symbol(source);
        let (file, line) = (symbol.source, symbol.line);
        let function = self.function(symbol).ok_or(Error::Limit)?;
        let body: Body = function
            .body
            .as_ref()
            .ok_or(Error::Limit)?
            .inline_snapshot();
        let mut metadata = function.metadata;
        metadata.binding = Some(Binding::Internal);
        metadata.object = None;
        metadata.flags &= !4;
        let signature = function.signature;
        let location = function.location;
        let symbol = self.reference(name, file, line)?;
        if !matches!(self.symbol(symbol).detail, SymbolDetail::Unresolved) {
            return Err(Error::Construction(
                "inline copy symbol is already defined".into(),
            ));
        }
        let id = FunctionId(u32::try_from(self.functions.len()).map_err(|_| Error::Limit)?);
        self.symbols[symbol.index()].detail = SymbolDetail::Function(id);
        self.functions.push(Function {
            name,
            signature,
            metadata,
            body: Some(body),
            location,
        });
        self.unpublished_functions.insert(symbol);
        Ok(symbol)
    }
}
