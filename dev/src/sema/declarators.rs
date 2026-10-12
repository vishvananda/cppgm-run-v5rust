// Translated from sema/analyzer_types.cpp's declarator/ptr_operator/apply_suffix. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    graph::ScopeId,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{PostTokenKind, identifiers::NameId, pp_lexer::Punctuator, vocabulary::Keyword},
    syntax::ast::{self, ExprKind, SpecifierKind},
};
pub(super) struct DeclaratorInfo {
    pub ty: TypeId,
    pub name: Option<NameId>,
    pub home: Option<ScopeId>,
    pub parameters: Option<ast::ParameterClause>,
}
impl Analyzer<'_> {
    pub(super) fn declarator_name(
        &self,
        mut id: Option<ast::DeclaratorId>,
    ) -> Result<Option<NameId>, Error> {
        while let Some(current) = id {
            let node = self.tree.declarators.get(current);
            if let Some(name) = node.name {
                let last = self
                    .tree
                    .name_lists
                    .get(self.tree.names.get(name).components)
                    .last()
                    .ok_or(Error::Invariant("empty declaration name"))?;
                return Self::component_name(last.kind).map(Some);
            }
            id = node.nested;
        }
        Ok(None)
    }
    pub(super) fn declarator_location(
        &self,
        mut id: ast::DeclaratorId,
    ) -> Result<crate::support::source::SourceOffset, Error> {
        loop {
            let node = self.tree.declarators.get(id);
            if let Some(name) = node.name {
                return Ok(self.tree.names.get(name).location);
            }
            let Some(nested) = node.nested else {
                return Ok(node.location);
            };
            id = nested;
        }
    }
    pub(super) fn type_syntax(&mut self, id: ast::TypeId, scope: ScopeId) -> Result<TypeId, Error> {
        let node = *self.tree.types.get(id);
        let specs = self.specifiers(node.specifiers, scope, false)?;
        let ty = self.declarator(node.declarator, specs.ty, scope)?.ty;
        super::record(&mut self.graph.type_facts, id.index(), ty);
        Ok(ty)
    }
    pub(super) fn declarator(
        &mut self,
        mut id: Option<ast::DeclaratorId>,
        mut ty: TypeId,
        scope: ScopeId,
    ) -> Result<DeclaratorInfo, Error> {
        let root = id;
        let mut name = None;
        let mut home = None;
        let mut parameters = None;
        let mut after = scope;
        // Qualified declarator-id controls lookup in subsequent suffixes.
        let mut probe = id;
        while let Some(current) = probe {
            let node = self.tree.declarators.get(current);
            if let Some(named) = node.name {
                home = self.prefix(named, scope)?;
                after = home.unwrap_or(scope);
                break;
            }
            probe = node.nested;
        }
        if home.is_some_and(|home| !self.graph.encloses(scope, home)) {
            return Err(Error::at(
                self.tree
                    .declarators
                    .get(id.ok_or(Error::Invariant("missing qualified declarator"))?)
                    .location,
                "qualified definition in a scope that does not enclose the member",
            ));
        }
        while let Some(current) = id {
            self.check_stack()?;
            let node = *self.tree.declarators.get(current);
            if node.pack {
                return Err(Error::Missing("semantic parameter packs"));
            }
            if node.name.is_some() {
                name = self.declarator_name(Some(current))?;
            }
            for pointer in self.tree.pointers.get(node.pointers) {
                parameters = None;
                if pointer.class_name.is_some() {
                    return Err(Error::Missing("member pointer type construction"));
                }
                let cv = Self::cv(self.tree.cvs.get(pointer.cv));
                match pointer.op.token.as_punctuator() {
                    Some(Punctuator::Multiply) => {
                        if matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. }) {
                            return Err(Error::at(node.location, "pointer to reference"));
                        }
                        let ptr = self.graph.types.pointer(ty)?;
                        ty = self.graph.types.qualify(ptr, cv)?;
                    }
                    Some(Punctuator::BitAnd | Punctuator::LogicalAnd) => {
                        if cv.0 != 0 {
                            return Err(Error::at(node.location, "cv-qualified reference"));
                        }
                        if matches!(
                            self.graph.types.get(ty).kind,
                            TypeKind::Scalar(Scalar::Void)
                        ) {
                            return Err(Error::at(node.location, "reference to void"));
                        }
                        let kind = if pointer.op.token.as_punctuator() == Some(Punctuator::BitAnd) {
                            RefKind::Lvalue
                        } else {
                            RefKind::Rvalue
                        };
                        ty = self.graph.types.reference(ty, kind)?;
                    }
                    _ => return Err(Error::Missing("extended declarator pointer operator")),
                }
            }
            for suffix in self.tree.suffixes.get(node.suffixes).iter().rev() {
                match *suffix {
                    ast::Suffix::Array(bound) => {
                        parameters = None;
                        let failure = match self.graph.types.get(ty).kind {
                            TypeKind::Reference { .. } => Some("array of references"),
                            TypeKind::Function { .. } => Some("array of functions"),
                            TypeKind::Scalar(Scalar::Void) => Some("array of void"),
                            TypeKind::Array { bound: 0, .. } => {
                                Some("array of arrays of unknown bound")
                            }
                            _ => None,
                        };
                        if let Some(message) = failure {
                            return Err(Error::at(node.location, message));
                        }
                        let bound = bound
                            .map(|bound| self.literal_bound(bound))
                            .transpose()?
                            .unwrap_or(0);
                        ty = self
                            .graph
                            .types
                            .intern(TypeKind::Array { element: ty, bound }, Cv::default())?;
                    }
                    ast::Suffix::Function {
                        parameters: clause,
                        qualifiers,
                    } => {
                        ty = self.function_type(ty, clause, qualifiers, after, node.location)?;
                        parameters = Some(clause);
                    }
                }
            }
            id = node.nested;
        }
        if let Some(root) = root {
            super::record(&mut self.graph.declarator_types, root.index(), ty);
        }
        Ok(DeclaratorInfo {
            ty,
            name,
            home,
            parameters,
        })
    }
    fn function_type(
        &mut self,
        mut ty: TypeId,
        clause: ast::ParameterClause,
        qualifiers: crate::syntax::storage::List<ast::Qualifier>,
        scope: ScopeId,
        location: crate::support::source::SourceOffset,
    ) -> Result<TypeId, Error> {
        if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Array { .. } | TypeKind::Function { .. }
        ) {
            return Err(Error::at(
                location,
                "function returning an array or function",
            ));
        }
        let mut types = Vec::new();
        for parameter in self.tree.parameters.get(clause.parameters) {
            let specs = self.specifiers(parameter.specifiers, scope, false)?;
            let info = self.declarator(parameter.declarator, specs.ty, scope)?;
            types.push(info.ty);
        }
        if types.len() == 1
            && !clause.ellipsis
            && self
                .declarator_name(self.tree.parameters.get(clause.parameters)[0].declarator)?
                .is_none()
            && matches!(
                self.graph.types.get(types[0]).kind,
                TypeKind::Scalar(Scalar::Void)
            )
            && self.graph.types.get(types[0]).cv.0 == 0
        {
            types.clear();
        }
        if types.iter().any(|&ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Scalar(Scalar::Void)
            )
        }) {
            return Err(Error::at(location, "parameter of type void"));
        }
        let mut cv = Cv::default();
        let mut reference = None;
        for qualifier in self.tree.qualifiers.get(qualifiers) {
            match qualifier.kind {
                ast::QualifierKind::Cv(key) => cv.0 |= Self::cv(&[key]).0,
                ast::QualifierKind::Ref(op) => {
                    reference = Some(if op.token.as_punctuator() == Some(Punctuator::BitAnd) {
                        RefKind::Lvalue
                    } else {
                        RefKind::Rvalue
                    });
                }
                ast::QualifierKind::Trailing(result) => {
                    ty = self.type_syntax(result, scope)?;
                }
                ast::QualifierKind::Noexcept(_) | ast::QualifierKind::Throw(_) => {
                    return Err(Error::Missing("function exception specification analysis"));
                }
                ast::QualifierKind::Virt(_) => {
                    return Err(Error::Missing("virtual override checks"));
                }
            }
        }
        let ty = self.graph.types.intern(
            TypeKind::Function {
                result: ty,
                parameters: types,
                variadic: clause.ellipsis,
                cv,
                reference,
            },
            Cv::default(),
        )?;
        Ok(ty)
    }
    pub(super) fn cv(keys: &[Keyword]) -> Cv {
        Cv(keys.iter().fold(0, |bits, key| {
            bits | if *key == Keyword::Const { 1 } else { 2 }
        }))
    }
    fn literal_bound(&self, id: ast::ExprId) -> Result<u64, Error> {
        let expr = self.tree.expressions.get(id);
        let ExprKind::Literal(literal) = expr.kind else {
            return Err(Error::Missing("array-bound integral constant evaluation"));
        };
        let (PostTokenKind::Literal { ty, data, .. }
        | PostTokenKind::Character {
            ty,
            data,
            suffix_begin: None,
            ..
        }) = literal.kind
        else {
            return Err(Error::Missing("array-bound integral constant evaluation"));
        };

        if matches!(
            ty,
            crate::types::FundamentalType::Float
                | crate::types::FundamentalType::Double
                | crate::types::FundamentalType::LongDouble
        ) {
            return Err(Error::at(
                expr.location,
                "floating-point constant evaluation is not supported here",
            ));
        }
        let mut bytes = [0u8; 8];
        let data = data.as_slice();
        let count = data.len().min(8);
        bytes[..count].copy_from_slice(&data[..count]);
        let bound = u64::from_le_bytes(bytes);
        let signed = matches!(
            ty,
            crate::types::FundamentalType::Char
                | crate::types::FundamentalType::WcharT
                | crate::types::FundamentalType::Int
                | crate::types::FundamentalType::LongInt
                | crate::types::FundamentalType::LongLongInt
        );
        if signed && count != 0 && data[count - 1] & 0x80 != 0 {
            return Err(Error::at(expr.location, "negative array bound"));
        }
        if bound == 0 {
            return Err(Error::at(expr.location, "array bound is zero"));
        }
        Ok(bound)
    }
    pub(super) fn specifier_type(
        &mut self,
        spec: ast::Specifier,
        scope: ScopeId,
        lone: bool,
    ) -> Result<TypeId, Error> {
        match spec.kind {
            SpecifierKind::Named(name) => {
                let found = self.lookup(name, scope, super::lookup::Filter::Ordinary)?;
                let binding = self.graph.binding(found);
                if !matches!(
                    binding.kind,
                    super::graph::BindingKind::Type | super::graph::BindingKind::Alias
                ) {
                    return Err(Error::at(spec.location, "name does not denote a type"));
                }
                binding
                    .ty
                    .ok_or(Error::Invariant("type binding lacks a type"))
            }
            SpecifierKind::Class(id) => self.class_type(id, scope),
            SpecifierKind::Elaborated { key, name } => self.elaborated_type(key, name, scope, lone),
            SpecifierKind::Enum(_) => Err(Error::Missing(
                "enumeration declarations and integral constant evaluation",
            )),
            SpecifierKind::Decltype(_) => Err(Error::Missing("decltype semantic facts")),
            SpecifierKind::Vector { .. } => Err(Error::Missing("vector types")),
            SpecifierKind::Keyword(_) => {
                Err(Error::Invariant("keyword reached named-type dispatcher"))
            }
        }
    }
}
