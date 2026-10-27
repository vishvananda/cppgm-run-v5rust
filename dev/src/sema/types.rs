// Translated from sema/type_table.cpp. See NOTICE.
//! Canonical, immutable type identity owned by one semantic unit.
use super::{Error, graph::EntityId};
use crate::types::FundamentalType;
use std::{
    collections::{HashMap, hash_map::RandomState},
    hash::BuildHasher,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct TypeId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Scalar {
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
    pub(crate) fn name(self) -> &'static str {
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
    pub(super) const ATOMIC: Self = Self(4);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum RefKind {
    Lvalue,
    Rvalue,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum TypeKind {
    Auto(EntityId),
    Scalar(Scalar),
    Complex(TypeId),
    Vector {
        element: TypeId,
        bound: u64,
        operand: Option<TypeId>,
        // Only dependent sizes retain whether the expression counts bytes.
        bytes: bool,
    },
    Class(EntityId),
    Enum(EntityId),
    Template(EntityId),
    TemplateParameter {
        entity: EntityId,
        template: bool,
    },
    Value {
        ty: TypeId,
        bits: u64,
    },
    // A non-null template argument: canonical target type plus designated entity.
    // Null address arguments retain Value(type, 0); both are immutable leaves.
    Address {
        ty: TypeId,
        entity: EntityId,
    },
    Pack(Vec<TypeId>),
    Expansion(TypeId),
    BoundPack {
        arguments: TypeId,
        parameter: EntityId,
        depth: u32,
    },
    Pointer(TypeId),
    // Clang blocks retain their function identity separately from ordinary pointers.
    BlockPointer(TypeId),
    MemberPointer {
        owner: TypeId,
        element: TypeId,
    },
    Reference {
        element: TypeId,
        kind: RefKind,
    },
    Array {
        element: TypeId,
        // Zero means unknown; a retained operand instead makes the bound
        // dependent. Concrete bounds never retain an operand.
        bound: u64,
        operand: Option<TypeId>,
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
impl TypeKind {
    /// A pointer level and its member owner, if any. Matching the owner as
    /// well as the level kind preserves mixed ordinary/member pointer shapes.
    pub(super) fn pointer_level(&self) -> Option<(Option<TypeId>, TypeId)> {
        match *self {
            Self::Pointer(element) => Some((None, element)),
            Self::MemberPointer { owner, element } => Some((Some(owner), element)),
            _ => None,
        }
    }
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
    declarations: Vec<Option<TypeId>>,
    declaration_facts: Box<DeclarationFacts>,
    signatures: Vec<Option<TypeId>>,
    block_invocations: HashMap<TypeId, TypeId>,
    parameter_entities: HashMap<EntityId, EntityId>,
}
#[derive(Default)]
struct DeclarationFacts {
    operands: HashMap<EntityId, EntityId>,
    classes: HashMap<EntityId, TypeId>,
}
impl Types {
    pub(super) fn initialize_fundamentals(&mut self) -> Result<(), Error> {
        use FundamentalType as F;
        for scalar in [
            Scalar::SignedChar,
            Scalar::Short,
            Scalar::Basic(F::Int),
            Scalar::Basic(F::LongInt),
            Scalar::Basic(F::LongLongInt),
            Scalar::UnsignedChar,
            Scalar::UnsignedShort,
            Scalar::Basic(F::UnsignedInt),
            Scalar::Basic(F::UnsignedLongInt),
            Scalar::Basic(F::UnsignedLongLongInt),
            Scalar::Basic(F::WcharT),
            Scalar::Basic(F::Char),
            Scalar::Basic(F::Char16T),
            Scalar::Basic(F::Char32T),
            Scalar::Bool,
            Scalar::Basic(F::Float),
            Scalar::Basic(F::Double),
            Scalar::Basic(F::LongDouble),
            Scalar::Void,
            Scalar::Nullptr,
            Scalar::Basic(F::Int128),
            Scalar::Basic(F::UnsignedInt128),
            Scalar::Basic(F::Float16),
            Scalar::Basic(F::Float32),
            Scalar::Basic(F::Float64),
            Scalar::Basic(F::Float128),
            Scalar::Basic(F::Float32x),
            Scalar::Basic(F::Float64x),
            Scalar::Basic(F::GnuFloat128),
            Scalar::Basic(F::BFloat16),
        ] {
            self.scalar(scalar)?;
        }
        Ok(())
    }
    pub(super) fn observations(&self) -> (usize, usize) {
        let children = self
            .records
            .iter()
            .map(|ty| match &ty.kind {
                TypeKind::Function { parameters, .. } | TypeKind::Pack(parameters) => parameters
                    .capacity()
                    .saturating_mul(std::mem::size_of::<TypeId>()),
                _ => 0,
            })
            .sum::<usize>();
        (
            self.records.len(),
            self.records
                .capacity()
                .saturating_mul(std::mem::size_of::<Type>())
                .saturating_add(children),
        )
    }
    /// Published before the class type escapes formation, so declaration
    /// memos never need invalidation. Substitution keeps the original identity.
    pub(super) fn record_declared_class(&mut self, entity: EntityId, ty: TypeId) {
        self.declaration_facts.classes.insert(entity, ty);
    }
    pub(super) fn record_operand_representative(&mut self, entity: EntityId, earlier: EntityId) {
        self.declaration_facts.operands.insert(entity, earlier);
    }
    pub(super) fn record_parameter_entity(&mut self, named: EntityId, canonical: EntityId) {
        self.parameter_entities.insert(named, canonical);
    }
    pub(super) fn canonical_view(&self, id: TypeId) -> TypeId {
        self.canonical[id.0 as usize].unwrap_or(id)
    }
    /// The top-level unqualified form; array element qualification is retained.
    pub(super) fn unqualified(&mut self, id: TypeId) -> Result<TypeId, Error> {
        if self.get(id).cv.0 == 0 {
            return Ok(id);
        }
        self.intern(self.get(id).kind.clone(), Cv::default())
    }
    pub(super) fn get(&self, id: TypeId) -> &Type {
        &self.records[id.0 as usize]
    }
    pub(super) fn pack_elements(&self, id: TypeId) -> Result<&[TypeId], Error> {
        match &self.get(id).kind {
            TypeKind::Pack(elements) => Ok(elements),
            _ => Err(Error::Invariant("argument is not a pack")),
        }
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
        let id = self.intern_raw(kind, cv)?;
        if cv.0 != 0 {
            let bare = self.intern_raw(self.get(id).kind.clone(), Cv::default())?;
            self.canonical_type(bare, false)?;
        }
        self.canonical_type(id, false)?;
        Ok(id)
    }
    // Canonicalization publishes through the same structural index, without
    // recursively restarting the walk of the type being completed.
    fn intern_raw(&mut self, kind: TypeKind, cv: Cv) -> Result<TypeId, Error> {
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
        self.declarations.push(None);
        self.signatures.push(None);
        self.index.insert(fingerprint, id);
        Ok(id)
    }
    pub(super) fn function_variant(
        &self,
        signature: TypeId,
        cv: Cv,
        reference: Option<RefKind>,
    ) -> Result<Option<TypeId>, Error> {
        let mut ty = self.get(signature).clone();
        let TypeKind::Function {
            cv: old_cv,
            reference: old_reference,
            ..
        } = &mut ty.kind
        else {
            return Err(Error::Invariant("qualification query on no function"));
        };
        *old_cv = cv;
        *old_reference = reference;
        let mut current = self.index.get(&self.fingerprints.hash_one(&ty)).copied();
        while let Some(id) = current {
            if self.get(id) == &ty {
                return Ok(Some(id));
            }
            current = self.collisions[id.0 as usize];
        }
        Ok(None)
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
        while let TypeKind::Array {
            element,
            bound,
            operand,
        } = self.get(id).kind
        {
            bounds.push((bound, operand));
            id = element;
        }
        if !matches!(
            self.get(id).kind,
            TypeKind::Reference { .. }
                | TypeKind::Function { .. }
                | TypeKind::Value { .. }
                | TypeKind::Address { .. }
                | TypeKind::Pack(_)
                | TypeKind::Expansion(_)
        ) {
            let node = self.get(id);
            let cv = Cv(node.cv.0 | cv.0);
            if cv != node.cv {
                let kind = node.kind.clone();
                id = self.intern(kind, cv)?;
            }
        }
        for (bound, operand) in bounds.into_iter().rev() {
            id = self.intern(
                TypeKind::Array {
                    element: id,
                    bound,
                    operand,
                },
                Cv::default(),
            )?;
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
    /// Prepare the ABI callable only when an evaluated block call demands it.
    /// Ordinary and block calls then share canonical invoke signatures.
    pub(super) fn prepare_block_invocation(&mut self, function: TypeId) -> Result<(), Error> {
        let function = self.canonical_function(function)?;
        if self.block_invocations.contains_key(&function) {
            return Ok(());
        }
        let TypeKind::Function {
            result,
            parameters,
            variadic,
            ..
        } = &self.get(function).kind
        else {
            return Err(Error::Invariant("block invocation without function type"));
        };
        let result = *result;
        let variadic = *variadic;
        let mut parameters = parameters.clone();
        let void = self.scalar(Scalar::Void)?;
        let literal = self.pointer(void)?;
        parameters.insert(0, literal);
        let invoke = self.intern(
            TypeKind::Function {
                result,
                parameters,
                variadic,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )?;
        let invoke = self.canonical_function(invoke)?;
        self.block_invocations.insert(function, invoke);
        Ok(())
    }
    pub(super) fn block_invocation(&self, function: TypeId) -> Option<TypeId> {
        self.block_invocations
            .get(&self.canonical_view(function))
            .copied()
    }
    pub(super) fn member_pointer(
        &mut self,
        owner: TypeId,
        element: TypeId,
    ) -> Result<TypeId, Error> {
        self.intern(TypeKind::MemberPointer { owner, element }, Cv::default())
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
    // signature_of in analyzer_types.cpp: return types never key ordinary overloads.
    // Both the canonical function and its signature are immutable unit-owned types.
    pub(super) fn declaration_signature(&mut self, ty: TypeId) -> Result<TypeId, Error> {
        let ty = self.declared_type(ty)?;
        if let Some(signature) = self.signatures[ty.0 as usize] {
            return Ok(signature);
        }
        let void = self.scalar(Scalar::Void)?;
        let TypeKind::Function {
            parameters,
            variadic,
            cv,
            reference,
            ..
        } = &self.get(ty).kind
        else {
            return Err(Error::Invariant(
                "declaration signature without function type",
            ));
        };
        let kind = TypeKind::Function {
            result: void,
            parameters: parameters.clone(),
            variadic: *variadic,
            cv: *cv,
            reference: *reference,
        };
        let signature = self.intern(kind, Cv::default())?;
        self.signatures[ty.0 as usize] = Some(signature);
        Ok(signature)
    }
    pub(super) fn canonical_function(&mut self, id: TypeId) -> Result<TypeId, Error> {
        self.canonical_type(id, false)
    }
    pub(super) fn declared_type(&mut self, id: TypeId) -> Result<TypeId, Error> {
        if self.declaration_facts.operands.is_empty() && self.declaration_facts.classes.is_empty() {
            return self.canonical_function(id);
        }
        self.canonical_type(id, true)
    }
    fn completed_type(&self, id: TypeId, declaration: bool) -> Option<TypeId> {
        if declaration {
            self.declarations[id.0 as usize]
        } else {
            self.canonical[id.0 as usize]
        }
    }
    fn complete_type(&mut self, id: TypeId, result: TypeId, declaration: bool) {
        if declaration {
            self.declarations[id.0 as usize] = Some(result);
        } else {
            self.canonical[id.0 as usize] = Some(result);
        }
    }
    // Types refer only to older types: canonicalization is a memoized DAG walk.
    // Explicit frames also keep long pointer chains off the native stack.
    fn canonical_type(&mut self, root: TypeId, declaration: bool) -> Result<TypeId, Error> {
        if let Some(canonical) = self.completed_type(root, declaration) {
            return Ok(canonical);
        }
        let mut work = vec![(root, false)];
        while let Some((id, finish)) = work.pop() {
            if self.completed_type(id, declaration).is_some() {
                continue;
            }
            if declaration
                && let TypeKind::Class(entity) = self.get(id).kind
                && let Some(&aliased) = self.declaration_facts.classes.get(&entity)
            {
                if finish {
                    let canonical = self.canonical_child(aliased, true)?;
                    let canonical = self.qualify(canonical, self.get(id).cv)?;
                    self.complete_type(id, canonical, true);
                } else {
                    work.push((id, true));
                    work.push((aliased, false));
                }
                continue;
            }
            if !finish {
                work.push((id, true));
                match &self.get(id).kind {
                    TypeKind::Pointer(element)
                    | TypeKind::Complex(element)
                    | TypeKind::BlockPointer(element)
                    | TypeKind::Reference { element, .. }
                    | TypeKind::Expansion(element) => {
                        work.push((*element, false));
                    }
                    TypeKind::Pack(elements) => {
                        work.extend(elements.iter().map(|&p| (p, false)));
                    }
                    TypeKind::BoundPack { arguments, .. } => work.push((*arguments, false)),
                    TypeKind::Array {
                        element, operand, ..
                    }
                    | TypeKind::Vector {
                        element, operand, ..
                    } => {
                        work.push((*element, false));
                        work.extend(operand.map(|operand| (operand, false)));
                    }
                    TypeKind::MemberPointer { owner, element } => {
                        work.push((*owner, false));
                        work.push((*element, false));
                    }
                    TypeKind::Function {
                        result, parameters, ..
                    } => {
                        work.push((*result, false));
                        work.extend(parameters.iter().map(|&p| (p, false)));
                    }
                    TypeKind::Value { ty, .. } | TypeKind::Address { ty, .. } => {
                        work.push((*ty, false));
                    }
                    TypeKind::Auto(_)
                    | TypeKind::Scalar(_)
                    | TypeKind::Class(_)
                    | TypeKind::Template(_)
                    | TypeKind::Enum(_)
                    | TypeKind::TemplateParameter { .. } => (),
                }
                continue;
            }
            let node = self.get(id).clone();
            let kind = match node.kind {
                TypeKind::Value { ty, bits } => TypeKind::Value {
                    ty: self.canonical_child(ty, declaration)?,
                    bits,
                },
                TypeKind::Address { ty, entity } => TypeKind::Address {
                    ty: self.canonical_child(ty, declaration)?,
                    entity,
                },
                TypeKind::Pack(elements) => TypeKind::Pack(
                    elements
                        .into_iter()
                        .map(|p| self.canonical_child(p, declaration))
                        .collect::<Result<_, _>>()?,
                ),
                TypeKind::Expansion(pattern) => {
                    TypeKind::Expansion(self.canonical_child(pattern, declaration)?)
                }
                TypeKind::BoundPack {
                    arguments,
                    parameter,
                    depth,
                } => TypeKind::BoundPack {
                    arguments: self.canonical_child(arguments, declaration)?,
                    parameter,
                    depth,
                },
                TypeKind::Vector {
                    element,
                    bound,
                    operand,
                    bytes,
                } => TypeKind::Vector {
                    element: self.canonical_child(element, declaration)?,
                    bound,
                    operand: operand
                        .map(|operand| self.canonical_child(operand, declaration))
                        .transpose()?,
                    bytes,
                },
                TypeKind::Complex(element) => {
                    TypeKind::Complex(self.canonical_child(element, declaration)?)
                }
                TypeKind::Pointer(element) => {
                    TypeKind::Pointer(self.canonical_child(element, declaration)?)
                }
                TypeKind::BlockPointer(element) => {
                    TypeKind::BlockPointer(self.canonical_child(element, declaration)?)
                }
                TypeKind::MemberPointer { owner, element } => TypeKind::MemberPointer {
                    owner: self.canonical_child(owner, declaration)?,
                    element: self.canonical_child(element, declaration)?,
                },
                TypeKind::Reference { element, kind } => TypeKind::Reference {
                    element: self.canonical_child(element, declaration)?,
                    kind,
                },
                TypeKind::Array {
                    element,
                    bound,
                    operand,
                } => TypeKind::Array {
                    element: self.canonical_child(element, declaration)?,
                    bound,
                    operand: operand
                        .map(|operand| self.canonical_child(operand, declaration))
                        .transpose()?,
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
                            let canonical = self.canonical_child(p, declaration)?;
                            self.parameter(canonical)
                        })
                        .collect::<Result<_, _>>()?;
                    TypeKind::Function {
                        result: self.canonical_child(result, declaration)?,
                        parameters,
                        variadic,
                        cv,
                        reference,
                    }
                }
                TypeKind::TemplateParameter { entity, template } => {
                    let entity = self
                        .parameter_entities
                        .get(&entity)
                        .copied()
                        .unwrap_or(entity);
                    let entity = if declaration {
                        self.declaration_facts
                            .operands
                            .get(&entity)
                            .copied()
                            .unwrap_or(entity)
                    } else {
                        entity
                    };
                    TypeKind::TemplateParameter { entity, template }
                }
                other => other,
            };
            let canonical = self.intern_raw(kind, node.cv)?;
            self.complete_type(id, canonical, declaration);
            self.complete_type(canonical, canonical, declaration);
        }
        self.canonical_child(root, declaration)
    }
    fn canonical_child(&self, id: TypeId, declaration: bool) -> Result<TypeId, Error> {
        self.completed_type(id, declaration)
            .ok_or(Error::Invariant("uncompleted canonical type"))
    }
    pub(super) fn complete_array(&self, first: TypeId, second: TypeId) -> Option<TypeId> {
        if first == second {
            return Some(first);
        }
        if let (
            TypeKind::Array {
                element: a,
                bound: x,
                operand: None,
            },
            TypeKind::Array {
                element: b,
                bound: y,
                operand: None,
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
    fn newly_interned_types_already_own_their_canonical_and_unqualified_forms() {
        let mut types = Types::default();
        let int = types.scalar(Scalar::Basic(FundamentalType::Int)).unwrap();
        let array = types
            .intern(
                TypeKind::Array {
                    element: int,
                    bound: 3,
                    operand: None,
                },
                Cv::default(),
            )
            .unwrap();
        let function = types
            .intern(
                TypeKind::Function {
                    result: int,
                    parameters: vec![array],
                    variadic: false,
                    cv: Cv::default(),
                    reference: None,
                },
                Cv::default(),
            )
            .unwrap();
        let pointer = types.pointer(int).unwrap();
        let canonical = types.canonical_view(function);
        assert_ne!(function, canonical);
        assert_eq!(types.function_parameter(canonical, 0).unwrap(), pointer);
        let qualified = types
            .intern(TypeKind::Pointer(function), Cv::CONST)
            .unwrap();
        let bare = types.pointer(function).unwrap();
        let null = types
            .intern(TypeKind::Value { ty: bare, bits: 0 }, Cv::default())
            .unwrap();
        let canonical_pointer = types.canonical_view(bare);
        let canonical_null = types
            .intern(
                TypeKind::Value {
                    ty: canonical_pointer,
                    bits: 0,
                },
                Cv::default(),
            )
            .unwrap();
        assert_eq!(types.canonical_view(null), canonical_null);
        let const_int = types.qualify(int, Cv::CONST).unwrap();
        let const_array = types
            .intern(
                TypeKind::Array {
                    element: const_int,
                    bound: 5,
                    operand: None,
                },
                Cv::default(),
            )
            .unwrap();
        let count = types.observations().0;
        assert_eq!(types.unqualified(qualified).unwrap(), bare);
        assert_eq!(types.unqualified(const_array).unwrap(), const_array);
        assert!(
            matches!(types.get(const_array).kind, TypeKind::Array { element, .. } if element == const_int)
        );
        assert_eq!(types.canonical_function(function).unwrap(), canonical);
        assert_eq!(types.qualify(bare, Cv::CONST).unwrap(), qualified);
        assert_eq!(types.observations().0, count);
    }
    #[test]
    fn qualification_preserves_function_identity_and_member_qualifiers() {
        let mut types = Types::default();
        let int = types.scalar(Scalar::Basic(FundamentalType::Int)).unwrap();
        for cv in [Cv::default(), Cv::CONST, Cv(2), Cv(3)] {
            for reference in [None, Some(RefKind::Lvalue), Some(RefKind::Rvalue)] {
                let function = types
                    .intern(
                        TypeKind::Function {
                            result: int,
                            parameters: vec![int],
                            variadic: false,
                            cv,
                            reference,
                        },
                        Cv::default(),
                    )
                    .unwrap();
                let canonical = types.canonical_function(function).unwrap();
                let count = types.records.len();
                for added in [Cv::CONST, Cv(2), Cv(3)] {
                    assert_eq!(types.qualify(function, added).unwrap(), function);
                    assert_eq!(types.qualify(canonical, added).unwrap(), canonical);
                }
                assert_eq!(types.records.len(), count);
                let pointer = types.pointer(function).unwrap();
                let qualified = types.qualify(pointer, Cv::CONST).unwrap();
                assert_ne!(qualified, pointer);
                assert_eq!(types.get(qualified).kind, TypeKind::Pointer(function));
                assert_eq!(types.canonical_function(qualified).unwrap(), qualified);
            }
        }
    }

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
