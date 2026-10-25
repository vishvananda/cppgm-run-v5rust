//! Unpublished specialization copies owned by the `LowIR` program. See NOTICE.
use super::{Body, Error, Function, FunctionId, NameId, Program, SymbolDetail, SymbolId};
use crate::lowir::vocabulary::Binding;
impl Program {
    /// Specialization changes the definition's signature alone; declarations
    /// and other definitions may still share the old immutable signature.
    pub(crate) fn remove_function_parameters(
        &mut self,
        symbol: SymbolId,
        removed: &[bool],
    ) -> Result<(), Error> {
        let function = self.function(self.symbol(symbol)).ok_or(Error::Limit)?;
        let sig = self.signature(function.signature);
        if sig.parameters.len() != removed.len() {
            return Err(Error::Limit);
        }
        let parameters = sig
            .parameters
            .iter()
            .zip(removed)
            .filter(|&(_, &removed)| !removed)
            .map(|(p, _)| super::Parameter {
                name: p.name,
                ty: p.ty,
                pass: p.pass,
                no_alias: p.no_alias,
                object_bytes: p.object_bytes,
                classes: p.classes,
            })
            .collect();
        let signature = super::Signature {
            parameters,
            result: sig.result,
            boundary: super::Boundary {
                arity: sig.boundary.arity,
                effects: sig.boundary.effects,
                unwind: sig.boundary.unwind,
                returns: sig.boundary.returns,
                result_classes: sig.boundary.result_classes,
            },
        };
        let signature = self.add_signature(signature)?;
        let SymbolDetail::Function(id) = self.symbol(symbol).detail else {
            return Err(Error::Limit);
        };
        self.functions[id.0 as usize].signature = signature;
        Ok(())
    }
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
