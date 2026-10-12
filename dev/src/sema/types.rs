// Translated from sema/type_table.cpp. See NOTICE.
//! Canonical, immutable type identity owned by one semantic unit.
use super::{Error, graph::EntityId};
use crate::types::FundamentalType;
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::BuildHasher,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct TypeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Scalar {
    Basic(FundamentalType),
    Void,
    Bool,
    SignedChar,
    UnsignedChar,
    Short,
    UnsignedShort,
    Nullptr,
}
impl Scalar {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Basic(ty) => ty.name(),
            Self::Void => "void",
            Self::Bool => "bool",
            Self::SignedChar => "signed char",
            Self::UnsignedChar => "unsigned char",
            Self::Short => "short int",
            Self::UnsignedShort => "unsigned short int",
            Self::Nullptr => "nullptr_t",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) struct Cv(pub u8);
impl Cv {
    pub(super) const CONST: Self = Self(1);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RefKind {
    Lvalue,
    Rvalue,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum TypeKind {
    Scalar(Scalar),
    Class(EntityId),
    Enum(EntityId),
    Pointer(TypeId),
    Reference {
        element: TypeId,
        kind: RefKind,
    },
    Array {
        element: TypeId,
        bound: u64,
    },
    Function {
        result: TypeId,
        parameters: Vec<TypeId>,
        variadic: bool,
        cv: Cv,
        reference: Option<RefKind>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Type {
    pub cv: Cv,
    pub kind: TypeKind,
}
#[derive(Default)]
pub(super) struct Types {
    records: Vec<Type>,
    // Fingerprints index store-owned records. Dense collision links retain
    // exact structural equality without a second owned parameter vector.
    index: HashMap<u64, TypeId>,
    fingerprints: RandomState,
    collisions: Vec<Option<TypeId>>,
    canonical: Vec<Option<TypeId>>,
}
impl Types {
    pub(super) fn get(&self, id: TypeId) -> &Type {
        &self.records[id.0 as usize]
    }
    pub(super) fn function_parameter_count(&self, ty: TypeId) -> Result<usize, Error> {
        let TypeKind::Function { ref parameters, .. } = self.get(ty).kind else {
            return Err(Error::Invariant("parameter query on no function"));
        };
        Ok(parameters.len())
    }
    pub(super) fn function_parameter(&self, ty: TypeId, index: usize) -> Result<TypeId, Error> {
        let TypeKind::Function { ref parameters, .. } = self.get(ty).kind else {
            return Err(Error::Invariant("parameter query on no function"));
        };
        parameters
            .get(index)
            .copied()
            .ok_or(Error::Invariant("parameter index outside function"))
    }
    pub(super) fn intern(&mut self, kind: TypeKind, cv: Cv) -> Result<TypeId, Error> {
        let ty = Type { cv, kind };
        let fingerprint = self.fingerprints.hash_one(&ty);
        let head = self.index.get(&fingerprint).copied();
        let mut entry = head;
        while let Some(id) = entry {
            if self.get(id) == &ty {
                return Ok(id);
            }
            entry = self.collisions[id.0 as usize];
        }
        let id = TypeId(
            u32::try_from(self.records.len()).map_err(|_| Error::Limit("semantic type limit"))?,
        );
        self.records.push(ty);
        self.collisions.push(head);
        self.canonical.push(None);
        self.index.insert(fingerprint, id);
        Ok(id)
    }
    pub(super) fn scalar(&mut self, scalar: Scalar) -> Result<TypeId, Error> {
        self.intern(TypeKind::Scalar(scalar), Cv::default())
    }
    pub(super) fn qualify(&mut self, mut id: TypeId, cv: Cv) -> Result<TypeId, Error> {
        if cv.0 == 0 {
            return Ok(id);
        }
        // Cv on an array belongs to its element; use explicit frames so an
        // arbitrarily long alias chain cannot exhaust the native stack.
        let mut bounds = Vec::new();
        while let TypeKind::Array { element, bound } = self.get(id).kind {
            bounds.push(bound);
            id = element;
        }
        if !matches!(self.get(id).kind, TypeKind::Reference { .. }) {
            let node = self.get(id);
            let cv = Cv(node.cv.0 | cv.0);
            if cv != node.cv {
                let kind = node.kind.clone();
                id = self.intern(kind, cv)?;
            }
        }
        for bound in bounds.into_iter().rev() {
            id = self.intern(TypeKind::Array { element: id, bound }, Cv::default())?;
        }
        Ok(id)
    }
    pub(super) fn reference(
        &mut self,
        mut element: TypeId,
        mut kind: RefKind,
    ) -> Result<TypeId, Error> {
        if let TypeKind::Reference {
            element: inner,
            kind: existing,
        } = self.get(element).kind
        {
            if existing == RefKind::Lvalue {
                kind = existing;
            }
            element = inner;
        }
        self.intern(TypeKind::Reference { element, kind }, Cv::default())
    }
    pub(super) fn pointer(&mut self, element: TypeId) -> Result<TypeId, Error> {
        self.intern(TypeKind::Pointer(element), Cv::default())
    }
    pub(super) fn parameter_object(&mut self, id: TypeId) -> Result<TypeId, Error> {
        match self.get(id).kind {
            TypeKind::Array { element, .. } => self.pointer(element),
            TypeKind::Function { .. } => self.pointer(id),
            _ => Ok(id),
        }
    }
    pub(super) fn parameter(&mut self, id: TypeId) -> Result<TypeId, Error> {
        match self.get(id).kind {
            TypeKind::Array { element, .. } => self.pointer(element),
            TypeKind::Function { .. } => self.pointer(id),
            _ => {
                if self.get(id).cv.0 == 0 {
                    return Ok(id);
                }
                let kind = self.get(id).kind.clone();
                self.intern(kind, Cv::default())
            }
        }
    }
    pub(super) fn canonical_function(&mut self, id: TypeId) -> Result<TypeId, Error> {
        self.canonical_type(id)
    }
    // Types refer only to older types: canonicalization is a memoized DAG walk.
    // Explicit frames also keep long pointer chains off the native stack.
    fn canonical_type(&mut self, root: TypeId) -> Result<TypeId, Error> {
        if let Some(canonical) = self.canonical[root.0 as usize] {
            return Ok(canonical);
        }
        let mut work = vec![(root, false)];
        while let Some((id, finish)) = work.pop() {
            if self.canonical[id.0 as usize].is_some() {
                continue;
            }
            if !finish {
                work.push((id, true));
                match &self.get(id).kind {
                    TypeKind::Pointer(element)
                    | TypeKind::Reference { element, .. }
                    | TypeKind::Array { element, .. } => work.push((*element, false)),
                    TypeKind::Function {
                        result, parameters, ..
                    } => {
                        work.push((*result, false));
                        work.extend(parameters.iter().map(|&p| (p, false)));
                    }
                    TypeKind::Scalar(_) | TypeKind::Class(_) | TypeKind::Enum(_) => (),
                }
                continue;
            }
            let node = self.get(id).clone();
            let kind = match node.kind {
                TypeKind::Pointer(element) => TypeKind::Pointer(self.canonical_child(element)?),
                TypeKind::Reference { element, kind } => TypeKind::Reference {
                    element: self.canonical_child(element)?,
                    kind,
                },
                TypeKind::Array { element, bound } => TypeKind::Array {
                    element: self.canonical_child(element)?,
                    bound,
                },
                TypeKind::Function {
                    result,
                    parameters,
                    variadic,
                    cv,
                    reference,
                } => {
                    let parameters = parameters
                        .into_iter()
                        .map(|p| {
                            let canonical = self.canonical_child(p)?;
                            self.parameter(canonical)
                        })
                        .collect::<Result<_, _>>()?;
                    TypeKind::Function {
                        result: self.canonical_child(result)?,
                        parameters,
                        variadic,
                        cv,
                        reference,
                    }
                }
                other => other,
            };
            let canonical = self.intern(kind, node.cv)?;
            self.canonical[id.0 as usize] = Some(canonical);
            self.canonical[canonical.0 as usize] = Some(canonical);
        }
        self.canonical_child(root)
    }
    fn canonical_child(&self, id: TypeId) -> Result<TypeId, Error> {
        self.canonical[id.0 as usize].ok_or(Error::Invariant("uncompleted canonical type"))
    }
    pub(super) fn complete_array(&self, first: TypeId, second: TypeId) -> Option<TypeId> {
        if first == second {
            return Some(first);
        }
        if let (
            TypeKind::Array {
                element: a,
                bound: x,
            },
            TypeKind::Array {
                element: b,
                bound: y,
            },
        ) = (&self.get(first).kind, &self.get(second).kind)
            && a == b
            && (*x == 0 || *y == 0)
        {
            return Some(if *x == 0 { second } else { first });
        }
        None
    }
}

#[cfg(test)]
mod collision_tests {
    use super::*;
    #[test]
    fn a_fingerprint_collision_keeps_exact_identity_and_reuses_each_record() {
        let mut types = Types::default();
        let int = types.scalar(Scalar::Basic(FundamentalType::Int)).unwrap();
        let pointer = Type {
            kind: TypeKind::Pointer(int),
            cv: Cv::default(),
        };
        let fingerprint = types.fingerprints.hash_one(&pointer);
        // Force a collision without relying on a particular randomized hash.
        types.index.insert(fingerprint, int);
        let id = types.pointer(int).unwrap();
        assert_ne!(int, id);
        assert_eq!(types.collisions[id.0 as usize], Some(int));
        assert_eq!(types.pointer(int).unwrap(), id);
        assert_eq!(
            types.scalar(Scalar::Basic(FundamentalType::Int)).unwrap(),
            int
        );
        assert_eq!(types.records.len(), 2);
        assert_eq!(types.records.len(), types.collisions.len());
    }
}
