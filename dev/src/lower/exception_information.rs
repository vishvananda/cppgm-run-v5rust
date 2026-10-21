// Translated from lower/rtti_lowering.cpp's exception type information. See NOTICE.
//! Exception clauses use runtime RTTI; `LowIR` views also retain the course's owned objects.
use super::{
    Error, Symbols,
    type_information::TypeObject,
    vtables::{InformationKind as K, address, define, integer},
};
use crate::{
    lowir::{
        model::{DataItem, Global, GlobalInit, Metadata, Program, SymbolId},
        vocabulary::{Binding, Role, Type},
    },
    sema::lowering::{Scalar, TypeId, TypeView},
};
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub(super) struct Information {
    external: HashMap<TypeId, SymbolId>,
    defined: HashMap<TypeId, SymbolId>,
    thrown: HashSet<TypeId>,
}
impl Symbols<'_, '_> {
    fn runtime_information_base(&self, ty: TypeId) -> Result<Option<(Scalar, bool, u8)>, Error> {
        let (base, pointer) = if let TypeView::Pointer(base) = self.graph.lower_type(ty)? {
            (base, true)
        } else {
            (ty, false)
        };
        Ok(match self.graph.lower_type(base)? {
            TypeView::Scalar(scalar)
                if self.graph.lower_cv(base) & 2 == 0 && (pointer || scalar != Scalar::Void) =>
            {
                Some((scalar, pointer, self.graph.lower_cv(base)))
            }
            _ => None,
        })
    }
    pub(in crate::lower) fn exception_type_information(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<SymbolId, Error> {
        let ty = self.graph.lower_rtti_type(ty)?;
        let Some((scalar, pointer, cv)) = self.runtime_information_base(ty)? else {
            return self.type_information(program, ty);
        };
        if let Some(&symbol) = self.exception_information.external.get(&ty) {
            return Ok(symbol);
        }
        let prefix = if !pointer {
            ""
        } else if cv & 1 != 0 {
            "pointer_to_const_"
        } else {
            "pointer_to_"
        };
        let label = format!(
            "__external_rtti__{prefix}{}",
            scalar.name().replace(' ', "_")
        );
        let object = self.type_encoding(ty, TypeObject::Information)?;
        let object = program.names.intern(&object)?;
        let symbol = self.table.allocate(program, &label, Some(object), false)?;
        if !program.symbol(symbol).declared() {
            program.add_global(
                symbol,
                Global {
                    name: program.symbol(symbol).name,
                    ty: None,
                    metadata: Metadata {
                        role: Some(Role::RttiData),
                        binding: Some(Binding::Strong),
                        object: Some(object),
                        ..Metadata::default()
                    },
                    init: GlobalInit::Declaration,
                },
            )?;
        }
        self.exception_information.external.insert(ty, symbol);
        Ok(symbol)
    }
    pub(in crate::lower) fn defined_exception_information(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<SymbolId, Error> {
        self.check_stack()?;
        let ty = self.graph.lower_rtti_type(ty)?;
        let Some((scalar, pointer, _)) = self.runtime_information_base(ty)? else {
            return self.type_information(program, ty);
        };
        if let Some(&symbol) = self.exception_information.defined.get(&ty) {
            return Ok(symbol);
        }
        if let Some(symbol) = self.type_information.get(ty)
            && program
                .global(program.symbol(symbol))
                .is_some_and(|global| matches!(global.init, GlobalInit::Data(_)))
        {
            return Ok(symbol);
        }
        let target_object = if let TypeView::Pointer(element) = self.graph.lower_type(ty)? {
            Some((
                element,
                self.defined_exception_information(program, element)?,
            ))
        } else {
            None
        };
        let encoding = self.type_encoding(ty, TypeObject::Encoding)?;
        let label = if pointer {
            format!("type_{encoding}")
        } else {
            scalar.name().replace(' ', "_")
        };
        let symbol = self
            .table
            .allocate(program, &format!("__rtti_{label}"), None, true)?;
        self.exception_information.defined.insert(ty, symbol);
        let vtable =
            self.rtti_vtable(program, if pointer { K::Pointer } else { K::Fundamental })?;
        let object = self.type_encoding(ty, TypeObject::Name)?;
        let object = program.names.intern(&object)?;
        let name = self.table.allocate(
            program,
            &format!("__typeinfo_name_{}{label}", if pointer { "" } else { "_" }),
            Some(object),
            false,
        )?;
        if !program.symbol(name).declared() {
            let items = encoding
                .bytes()
                .chain(std::iter::once(0))
                .map(|b| integer(Type::I8, u64::from(b)))
                .collect();
            define(
                program,
                name,
                Metadata {
                    binding: Some(Binding::Weak),
                    object: Some(object),
                    ..Metadata::default()
                },
                items,
            )?;
        }
        let mut items = vec![address(vtable, 16), address(name, 0)];
        if let Some((element, target_object)) = target_object {
            items.push(integer(Type::I32, u64::from(self.pointee_flags(element)?)));
            items.push(address(target_object, 0));
        }
        define(
            program,
            symbol,
            Metadata {
                binding: Some(Binding::Internal),
                ..Metadata::default()
            },
            items,
        )?;
        Ok(symbol)
    }
    pub(in crate::lower) fn thrown_type(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<(), Error> {
        // The course view retains otherwise unreferenced RTTI and a zeroed
        // placeholder for every thrown type. Native objects demand neither.
        if self.table.object_mode {
            return Ok(());
        }
        let ty = self.graph.lower_rtti_type(ty)?;
        if !self.exception_information.thrown.insert(ty) {
            return Ok(());
        }
        let information = self.defined_exception_information(program, ty)?;
        let name = program.names.get(program.symbol(information).name);
        let label = format!("__ehobj_{}", name.strip_prefix("__rtti_").unwrap_or(name));
        let symbol = self.table.allocate(program, &label, None, true)?;
        program.add_global(
            symbol,
            Global {
                name: program.symbol(symbol).name,
                ty: None,
                metadata: Metadata {
                    binding: Some(Binding::Internal),
                    ..Metadata::default()
                },
                init: GlobalInit::Data(vec![DataItem::Zero(i64::from_le_bytes(
                    crate::lower::types::layout(self.graph, ty)?.0.to_le_bytes(),
                ))]),
            },
        )?;
        Ok(())
    }
    pub(in crate::lower) fn caught_type(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<(), Error> {
        if !self.table.object_mode {
            self.defined_exception_information(program, ty)?;
        }
        Ok(())
    }
}
