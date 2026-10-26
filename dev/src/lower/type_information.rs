// Translated from lower/rtti_lowering.cpp::type_information/type_name_object. See NOTICE.
//! Program-owned RTTI is memoized by canonical type, with exact ABI object identities.
use super::{
    Error, Symbols,
    vtables::{InformationKind as K, address, define, integer},
};
use crate::{
    abi::encoder::{Encoder, Target},
    lowir::{
        model::{Global, GlobalInit, Metadata, Program, SymbolId},
        vocabulary::{Binding, Role, Type},
    },
    sema::lowering::{Scalar, TypeId, TypeView},
};
use std::collections::HashMap;
#[derive(Default)]
pub(super) struct Information {
    objects: HashMap<TypeId, SymbolId>,
}
impl Information {
    pub(super) fn get(&self, ty: TypeId) -> Option<SymbolId> {
        self.objects.get(&ty).copied()
    }
}
#[derive(Clone, Copy)]
pub(super) enum TypeObject {
    Encoding,
    Information,
    Name,
}
impl Symbols<'_, '_> {
    pub(super) fn local_type_information(&self, mut ty: TypeId) -> Result<bool, Error> {
        if !self.host_objects() {
            return Ok(false);
        }
        loop {
            if let Some((owner, element)) = self.graph.lower_member_pointer_type(ty) {
                if let TypeView::Class(class) = self.graph.lower_type(owner)?
                    && !self.graph.lower_class_complete(class)
                {
                    return Ok(true);
                }
                ty = element;
            } else {
                match self.graph.lower_type(ty)? {
                    TypeView::Class(class) => return Ok(!self.graph.lower_class_complete(class)),
                    TypeView::Pointer(element) => ty = element,
                    _ => return Ok(false),
                }
            }
        }
    }
    pub(super) fn type_encoding(&mut self, ty: TypeId, kind: TypeObject) -> Result<String, Error> {
        let node = self.abi_type(ty)?;
        let mut bytes = Vec::new();
        let target = match kind {
            TypeObject::Information => Target::Typeinfo(node),
            TypeObject::Name => Target::TypeinfoName(node),
            TypeObject::Encoding => Target::Type(node),
        };
        self.table.work.mangled_names += 1;
        Encoder::encode(&self.abi, target, &mut bytes, self.budget)?;
        String::from_utf8(bytes)
            .map_err(|_| crate::sema::Error::Invariant("non-UTF8 ABI type name").into())
    }
    pub(super) fn pointee_flags(&self, mut ty: TypeId) -> Result<u32, Error> {
        let mut flags = u32::from(self.graph.lower_cv(ty) & 3);
        while let TypeView::Pointer(element) = self.graph.lower_type(ty)? {
            ty = element;
        }
        if let TypeView::Class(class) = self.graph.lower_type(ty)?
            && !self.graph.lower_class_complete(class)
        {
            flags |= 8;
        }
        Ok(flags)
    }
    pub(in crate::lower) fn type_information(
        &mut self,
        program: &mut Program,
        ty: TypeId,
    ) -> Result<SymbolId, Error> {
        self.check_stack()?;
        let ty = self.graph.lower_rtti_type(ty)?;
        if let TypeView::Class(class) = self.graph.lower_type(ty)? {
            return self.typeinfo_symbol(program, class);
        }
        if self.host_objects() && self.runtime_information_base(ty)?.is_some() {
            return self.exception_type_information(program, ty);
        }
        if let Some(&symbol) = self.type_information.objects.get(&ty) {
            return Ok(symbol);
        }
        if matches!(self.graph.lower_type(ty)?, TypeView::Scalar(Scalar::Void)) {
            let symbol = self
                .table
                .allocate(program, "__external_rtti__void", None, true)?;
            let object = program.names.intern("_ZTIv")?;
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
            self.type_information.objects.insert(ty, symbol);
            return Ok(symbol);
        }
        let encoding = self.type_encoding(ty, TypeObject::Encoding)?;
        let fundamental = if let TypeView::Scalar(scalar) = self.graph.lower_type(ty)? {
            Some(scalar)
        } else {
            None
        };
        let label = if let Some(scalar) = fundamental {
            scalar.name().replace(' ', "_")
        } else {
            format!("type_{encoding}")
        };
        let object = self.type_encoding(ty, TypeObject::Information)?;
        let object = program.names.intern(&object)?;
        let internal = if self.table.object_mode {
            self.graph.lower_object_type_unit_local(ty)?
        } else {
            self.graph.lower_type_unit_local(ty)?
        };
        let local_information = self.local_type_information(ty)?;
        let symbol = self.table.allocate(
            program,
            &format!("__rtti_{label}"),
            Some(object),
            internal || local_information,
        )?;
        self.type_information.objects.insert(ty, symbol);
        if program.symbol(symbol).declared() {
            return Ok(symbol);
        }
        let (kind, pointee, owner) = match self.graph.lower_type(ty)? {
            TypeView::Pointer(element) => (K::Pointer, Some(element), None),
            TypeView::Function { .. } => (K::Function, None, None),
            TypeView::Array { .. } => (K::Array, None, None),
            TypeView::Enum(_) => (K::Enum, None, None),
            _ => {
                if let Some((owner, element)) = self.graph.lower_member_pointer_type(ty) {
                    (K::MemberPointer, Some(element), Some(owner))
                } else {
                    (K::Fundamental, None, None)
                }
            }
        };
        let target_object = pointee
            .map(|pointee| {
                let symbol = self.type_information(program, pointee)?;
                if !self.host_objects()
                    && self.runtime_information_base(pointee)?.is_some()
                    && program
                        .global(program.symbol(symbol))
                        .is_some_and(|global| matches!(global.init, GlobalInit::Declaration))
                {
                    self.defined_exception_information(program, pointee)
                } else {
                    Ok(symbol)
                }
            })
            .transpose()?;
        let holder = owner
            .map(|owner| self.type_information(program, owner))
            .transpose()?;
        let vtable = self.rtti_vtable(program, kind)?;
        let name_object = self.type_encoding(ty, TypeObject::Name)?;
        let name_object = program.names.intern(&name_object)?;
        let name = self.table.allocate(
            program,
            &format!(
                "__typeinfo_name_{}{label}",
                if fundamental.is_some() { "_" } else { "" }
            ),
            Some(name_object),
            internal,
        )?;
        if !program.symbol(name).declared() {
            let marker =
                self.table.object_mode && internal && self.graph.lower_private_type_name(ty)?;
            let items = marker
                .then_some(b'*')
                .into_iter()
                .chain(encoding.bytes())
                .chain(std::iter::once(0))
                .map(|b| integer(Type::I8, u64::from(b)))
                .collect();
            define(
                program,
                name,
                Metadata {
                    object: Some(name_object),
                    binding: Some(if internal {
                        Binding::Internal
                    } else {
                        Binding::Weak
                    }),
                    ..Metadata::default()
                },
                items,
            )?;
        }
        let mut items = vec![address(vtable, 16), address(name, 0)];
        if let (Some(pointee), Some(target_object)) = (pointee, target_object) {
            let mut flags = self.pointee_flags(pointee)?;
            if let Some(owner) = owner
                && let TypeView::Class(class) = self.graph.lower_type(owner)?
                && !self.graph.lower_class_complete(class)
            {
                flags |= 16;
            }
            items.push(integer(Type::I32, u64::from(flags)));
            items.push(address(target_object, 0));
        }
        if let Some(holder) = holder {
            items.push(address(holder, 0));
        }
        define(
            program,
            symbol,
            Metadata {
                object: Some(object),
                binding: Some(if internal || local_information {
                    Binding::Internal
                } else {
                    Binding::Weak
                }),
                ..Metadata::default()
            },
            items,
        )?;
        Ok(symbol)
    }
}
