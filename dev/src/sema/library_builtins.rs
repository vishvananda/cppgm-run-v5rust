// Translated from analyzer_builtins.cpp::builtin_signature. See NOTICE.
//! Canonical signatures for lazily declared fixed library builtins.
use super::{
    Error,
    analyzer::Analyzer,
    graph::BindingKind,
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{
        identifiers::NameId,
        library_builtins::{Parameter, Type},
    },
    types::FundamentalType,
};

impl Analyzer<'_> {
    pub(super) fn library_builtin_signature(
        &mut self,
        name: NameId,
    ) -> Result<Option<(TypeId, Vec<TypeId>)>, Error> {
        let entry = crate::lex::library_builtins::lookup(self.identifiers.bytes(name))
            .ok_or(Error::Invariant("library builtin without registry entry"))?;
        let Some(result) = self.library_builtin_type(entry.signature.result, entry.floating)?
        else {
            return Ok(None);
        };
        let mut parameters = Vec::with_capacity(entry.signature.parameters.len());
        for &parameter in entry.signature.parameters {
            let Some(ty) = self.library_builtin_type(parameter, entry.floating)? else {
                return Ok(None);
            };
            parameters.push(ty);
        }
        Ok(Some((result, parameters)))
    }

    fn library_builtin_type(
        &mut self,
        parameter: Parameter,
        floating: FundamentalType,
    ) -> Result<Option<TypeId>, Error> {
        let (kind, constant, pointer) = match parameter {
            Parameter::Value(kind) => (kind, false, false),
            Parameter::Pointer(kind) => (kind, false, true),
            Parameter::ConstPointer(kind) => (kind, true, true),
        };
        let mut ty = match kind {
            Type::Void => self.graph.types.scalar(Scalar::Void)?,
            Type::Scalar(ty) => self.graph.types.scalar(Scalar::Basic(ty))?,
            Type::Floating => self.graph.types.scalar(Scalar::Basic(floating))?,
            Type::Complex => return Err(Error::Missing("complex library builtin types")),
            Type::VaList => {
                let key = self
                    .identifiers
                    .intern(b"__builtin_va_list")
                    .map_err(|_| Error::Limit("semantic identifier limit"))?;
                // builtin_signature uses the declared global alias, independent
                // of an instantiation's source visibility limit. Restore the
                // query context before any fallible work or early return.
                let limit = self.graph.lookup_limit.take();
                let binding = self
                    .graph
                    .declared_name(self.graph.global, key, Filter::Prefix);
                self.graph.lookup_limit = limit;
                let Some(binding) = binding else {
                    return Ok(None);
                };
                let binding = self.graph.binding(binding);
                if binding.kind != BindingKind::Alias {
                    return Ok(None);
                }
                binding
                    .ty
                    .ok_or(Error::Invariant("variadic list alias without type"))?
            }
        };
        if constant {
            ty = self.graph.types.qualify(ty, Cv::CONST)?;
        }
        if pointer {
            ty = self
                .graph
                .types
                .intern(TypeKind::Pointer(ty), Cv::default())?;
        }
        Ok(Some(ty))
    }
}
