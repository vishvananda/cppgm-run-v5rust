// Translated from sema/type_table.cpp. See NOTICE.
//! Canonical, immutable type identity owned by one semantic unit.
use super::{Error, graph::EntityId};
use crate::types::FundamentalType;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    index: HashMap<Type, TypeId>,
    canonical: Vec<Option<TypeId>>,
}
impl Types {
    pub(super) fn get(&self, id: TypeId) -> &Type {
        &self.records[id.0 as usize]
    }
    pub(super) fn intern(&mut self, kind: TypeKind, cv: Cv) -> Result<TypeId, Error> {
        let ty = Type { cv, kind };
        if let Some(&id) = self.index.get(&ty) {
            return Ok(id);
        }
        let id = TypeId(
            u32::try_from(self.records.len()).map_err(|_| Error::Limit("semantic type limit"))?,
        );
        self.records.push(ty.clone());
        self.canonical.push(None);
        self.index.insert(ty, id);
        Ok(id)
    }
    pub(super) fn scalar(&mut self, scalar: Scalar) -> Result<TypeId, Error> {
        self.intern(TypeKind::Scalar(scalar), Cv::default())
    }
    pub(super) fn qualify(&mut self, mut id: TypeId, cv: Cv) -> Result<TypeId, Error> {
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
            let kind = node.kind.clone();
            id = self.intern(kind, cv)?;
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
    pub(super) fn parameter(&mut self, id: TypeId) -> Result<TypeId, Error> {
        match self.get(id).kind {
            TypeKind::Array { element, .. } => self.pointer(element),
            TypeKind::Function { .. } => self.pointer(id),
            _ => {
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
        let mut work = vec![(root, false)];
        while let Some((id, finish)) = work.pop() {
            if self.canonical[id.0 as usize].is_some() {
                continue;
            }
            let node = self.get(id).clone();
            if !finish {
                work.push((id, true));
                match node.kind {
                    TypeKind::Pointer(element)
                    | TypeKind::Reference { element, .. }
                    | TypeKind::Array { element, .. } => work.push((element, false)),
                    TypeKind::Function {
                        result, parameters, ..
                    } => {
                        work.push((result, false));
                        work.extend(parameters.iter().map(|&p| (p, false)));
                    }
                    TypeKind::Scalar(_) | TypeKind::Class(_) => (),
                }
                continue;
            }
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
