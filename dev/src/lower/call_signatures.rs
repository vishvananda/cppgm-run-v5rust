// Translated from lower/symbol_lowering.cpp::call_signature/member_call_signature. See NOTICE.
//! Canonical callable types key reusable indirect call boundaries, including adjusted objects.
use super::{Error, Program, Symbols, TypeId};

impl Symbols<'_, '_> {
    pub(in crate::lower) fn call_signature(
        &mut self,
        program: &mut Program,
        ty: TypeId,
        nothrow: bool,
    ) -> Result<crate::lowir::model::SignatureId, Error> {
        if let Some(&signature) = self.signatures.get(&(ty, nothrow)) {
            return Ok(signature);
        }
        let signature = crate::lower::function::signature(program, self, self.graph, ty, nothrow)?;
        self.signatures.insert((ty, nothrow), signature);
        Ok(signature)
    }

    pub(in crate::lower) fn member_call_signature(
        &mut self,
        program: &mut Program,
        pointer: TypeId,
        nothrow: bool,
    ) -> Result<crate::lowir::model::SignatureId, Error> {
        use crate::lower::types;
        use crate::lowir::vocabulary::Type;
        if let Some(&signature) = self.signatures.get(&(pointer, nothrow)) {
            return Ok(signature);
        }
        let graph = self.graph;
        let (owner, function) = graph
            .lower_member_pointer_type(pointer)
            .ok_or(crate::sema::Error::Invariant("member call signature type"))?;
        let signature = crate::lower::function::signature(program, self, graph, function, nothrow)?;
        let first = usize::from(
            program
                .signature(signature)
                .parameters
                .first()
                .is_some_and(|p| p.pass == Some(crate::lowir::vocabulary::Pass::IndirectResult)),
        );
        let crate::sema::lowering::TypeView::Class(class) = graph.lower_type(owner)? else {
            return Err(crate::sema::Error::Invariant("member call signature owner").into());
        };
        let object_bytes = if graph.lower_class_bases(class).next().is_none() {
            types::layout(graph, owner)?.0
        } else {
            0
        };
        let name = program.names.intern("arg0")?;
        program.signature_mut(signature).parameters.insert(
            first,
            crate::lowir::model::Parameter {
                name,
                ty: Type::Ptr,
                pass: None,
                object_bytes,
                no_alias: false,
                classes: None,
            },
        );
        for index in 0..program.signature(signature).parameters.len() {
            let name = program.names.intern(&format!("arg{index}"))?;
            program.signature_mut(signature).parameters[index].name = name;
        }
        self.signatures.insert((pointer, nothrow), signature);
        Ok(signature)
    }
}
