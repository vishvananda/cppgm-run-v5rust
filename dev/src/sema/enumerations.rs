// Translated from sema/analyzer_classes.cpp's enumeration group. See NOTICE.
//! Enumerators publish sequential values, then acquire the finished enum's identity.
use super::{
    Error,
    analyzer::Analyzer,
    constants::{Constant, ConstantState},
    graph::{Binding, BindingId, BindingKind, EntityId, ScopeId, ScopeKind},
    integral::{self, IntegralKind},
    lookup::Filter,
    types::{Cv, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::vocabulary::Keyword,
    support::source::SourceOffset,
    syntax::ast::{self, DeclKind},
    types::FundamentalType as F,
};
#[derive(Clone, Copy)]
pub(super) struct EnumInfo {
    pub scoped: bool,
    pub fixed: bool,
    pub negative: bool,
    pub underlying: Option<TypeId>,
    pub owner: ScopeId,
    pub anonymous_ordinal: Option<u32>,
}
const CANDIDATES: [F; 6] = [
    F::Int,
    F::UnsignedInt,
    F::LongInt,
    F::UnsignedLongInt,
    F::LongLongInt,
    F::UnsignedLongLongInt,
];
impl Analyzer<'_> {
    pub(super) fn name_unnamed_enum(
        &mut self,
        ty: TypeId,
        name: Option<crate::lex::identifiers::NameId>,
    ) -> Result<(), Error> {
        let TypeKind::Enum(entity) = self.graph.types.get(ty).kind else {
            return Ok(());
        };
        let info = self
            .graph
            .entity(entity)
            .enumeration
            .ok_or(Error::Invariant("unnamed enum metadata"))?;
        if self.graph.entity(entity).name.is_some() || info.anonymous_ordinal.is_some() {
            return Ok(());
        }
        if let Some(name) = name {
            self.graph.entity_mut(entity).name = Some(name);
        } else {
            self.graph.anonymous_enums = self
                .graph
                .anonymous_enums
                .checked_add(1)
                .ok_or(Error::Limit("anonymous enumeration count"))?;
            self.graph.entity_mut(entity).enumeration = Some(EnumInfo {
                anonymous_ordinal: Some(self.graph.anonymous_enums),
                ..info
            });
        }
        Ok(())
    }
    pub(super) fn elaborated_enum(
        &mut self,
        name: ast::Name,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        let found = self.lookup(name, scope, Filter::Elaborated)?;
        let binding = self.graph.binding(found);
        let ty = binding
            .ty
            .ok_or(Error::Invariant("elaborated enum without type"))?;
        if binding.kind != BindingKind::Type
            || !matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_))
        {
            return Err(Error::at(
                self.tree.names.get(name).location,
                "elaborated enum specifier does not name an enumeration",
            ));
        }
        Ok(ty)
    }
    fn check_enum_parameter_names(
        &self,
        name: Option<ast::Name>,
        introduces: bool,
        enumerators: crate::syntax::storage::List<ast::Enumerator>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        if self.template_depth == 0 {
            return Ok(());
        }
        if let Some(name) = name {
            let node = self.tree.names.get(name);
            if !node.global && self.tree.name_lists.get(node.components).len() == 1 && introduces {
                let last = self.tree.name_lists.get(node.components)[0];
                if last.arguments.is_none() {
                    self.check_template_parameter_name(
                        scope,
                        Self::component_name(last.kind)?,
                        node.location,
                    )?;
                }
            }
        }
        for syntax_index in 0..self.tree.enumerators.get(enumerators).len() {
            let syntax_item = self.tree.enumerators.get(enumerators)[syntax_index];
            let enumerator = &syntax_item;
            self.check_template_parameter_name(scope, enumerator.name, enumerator.location)?;
        }
        Ok(())
    }
    pub(super) fn enum_type(
        &mut self,
        id: ast::DeclId,
        scope: ScopeId,
        lone: bool,
    ) -> Result<TypeId, Error> {
        let declaration = *self.tree.declarations.get(id);
        let DeclKind::Enum {
            name,
            key,
            underlying,
            enumerators,
            has_body,
            ..
        } = declaration.kind
        else {
            return Err(Error::Invariant("enum syntax mismatch"));
        };
        let location = declaration.location;
        self.check_enum_parameter_names(name, has_body || lone, enumerators, scope)?;
        let scoped = key.is_some();
        let underlying = underlying
            .map(|id| self.type_syntax(id, scope))
            .transpose()?;
        let underlying = underlying
            .map(|ty| {
                self.graph
                    .types
                    .intern(self.graph.types.get(ty).kind.clone(), Cv::default())
            })
            .transpose()?;
        if underlying.is_some_and(|ty| {
            (!matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(_))
                || self.integral_kind(ty).is_none())
                && !(self.template_depth != 0 && self.graph.dependent_type(ty))
        }) {
            return Err(Error::at(
                location,
                "enumeration underlying type is not integral",
            ));
        }
        let Some(name) = name else {
            if !has_body || scoped {
                return Err(Error::at(
                    location,
                    "an enumeration without a name needs a body and cannot be scoped",
                ));
            }
            let (entity, binding) = self.new_enum(
                None,
                EnumInfo {
                    scoped: false,
                    negative: false,
                    fixed: underlying.is_some(),
                    underlying,
                    owner: scope,
                    anonymous_ordinal: None,
                },
                scope,
            )?;
            let ty = self
                .graph
                .entity(entity)
                .ty
                .ok_or(Error::Invariant("anonymous enum type"))?;
            self.enumerators(entity, enumerators, scope, binding, None)?;
            return Ok(ty);
        };
        if !has_body && !lone {
            if scoped || underlying.is_some() {
                return Err(Error::at(
                    location,
                    "invalid elaborated enumeration specifier",
                ));
            }
            return self.elaborated_enum(name, scope);
        }
        let components = self
            .tree
            .name_lists
            .get(self.tree.names.get(name).components);
        let last = components
            .last()
            .ok_or(Error::Invariant("empty enum name"))?;
        if last.arguments.is_some() {
            return Err(Error::at(
                self.tree.names.get(name).location,
                "template-ids are not supported here",
            ));
        }
        let identifier = Self::component_name(last.kind)?;
        let prefix = self.prefix(name, scope)?;
        let home = prefix.unwrap_or(scope);
        if prefix.is_some() {
            if !matches!(
                self.graph.scope(home).kind,
                ScopeKind::Namespace | ScopeKind::Class
            ) {
                return Err(Error::at(
                    self.tree.names.get(name).location,
                    "qualified enumeration name must name a class or namespace member",
                ));
            }
            if !self.graph.encloses(self.declaration_scope(scope), home) {
                return Err(Error::at(
                    self.tree.names.get(name).location,
                    "enumeration definition in a scope that does not enclose it",
                ));
            }
        }
        let expected = if scoped && underlying.is_none() {
            Some(self.graph.types.scalar(Scalar::Basic(F::Int))?)
        } else {
            underlying
        };
        let existing = self
            .graph
            .scope(home)
            .index
            .get(&identifier)
            .and_then(|entry| entry.tag);
        let entity = if let Some(existing) = existing {
            self.redeclared_enum(existing, scoped, expected, location)?
        } else {
            if prefix.is_some() {
                return Err(Error::at(
                    location,
                    "qualified enumeration does not match a declaration",
                ));
            }
            if self
                .graph
                .scope(home)
                .index
                .get(&identifier)
                .is_some_and(|entry| entry.ty.is_some())
            {
                return Err(Error::at(
                    self.tree.names.get(name).location,
                    "type name conflicts with an earlier declaration",
                ));
            }
            if !has_body && !scoped && underlying.is_none() {
                return Err(Error::at(
                    location,
                    "an opaque unscoped enumeration needs an underlying type",
                ));
            }
            self.new_enum(
                Some(identifier),
                EnumInfo {
                    scoped,
                    negative: false,
                    fixed: scoped || underlying.is_some(),
                    underlying: expected,
                    owner: home,
                    anonymous_ordinal: None,
                },
                scope,
            )?
            .0
        };
        let ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("enum without type"))?;
        if has_body {
            if self.graph.entity(entity).defined {
                return Err(Error::at(location, "enumeration redefinition"));
            }
            let enum_scope = self
                .graph
                .entity(entity)
                .scope
                .ok_or(Error::Invariant("enum without scope"))?;
            let mut declaration_binding = self
                .graph
                .scope(home)
                .index
                .get(&identifier)
                .and_then(|entry| entry.tag)
                .ok_or(Error::Invariant("enum without declaration"))?;
            let print = if scoped && prefix.is_some() {
                let binding = self.graph.add_binding(
                    home,
                    Some(scope),
                    Binding {
                        kind: BindingKind::Type,
                        name: Some(identifier),
                        entity,
                        ty: Some(ty),
                        key: Keyword::Enum,
                    },
                )?;
                self.graph.binding_labels.insert(binding, name);
                declaration_binding = binding;
                let print = self.graph.add_scope(
                    ScopeKind::Enum,
                    enum_scope,
                    Some(scope),
                    Some(identifier),
                )?;
                self.graph.scope_mut(print).written_name = Some(name);
                print
            } else if scoped {
                enum_scope
            } else {
                home
            };
            let values = if prefix.is_some() && self.template_depth != 0 {
                let values =
                    self.member_lookup_bridge(scope, if scoped { enum_scope } else { home })?;
                self.record_member_enum(entity, id, scope);
                Some(values)
            } else {
                None
            };
            self.enumerators(entity, enumerators, print, declaration_binding, values)?;
            self.finish_enum_waiters(entity)?;
        } else if self.template_depth == 0 && self.graph.within_class_instance(home) {
            self.declare_member_enum_instance(entity)?;
        }
        Ok(ty)
    }

    // analyzer_classes.cpp::redeclared_enum: agreement is on canonical
    // underlying identity, including renamed dependent template parameters.
    fn redeclared_enum(
        &mut self,
        binding: BindingId,
        scoped: bool,
        expected: Option<TypeId>,
        location: SourceOffset,
    ) -> Result<EntityId, Error> {
        let entity = self.graph.binding(binding).entity;
        let info = self.graph.entity(entity).enumeration.ok_or_else(|| {
            Error::at(
                location,
                "enumeration name redeclared as a different kind of type",
            )
        })?;
        let underlying_matches = match (info.underlying, expected) {
            (Some(previous), Some(expected)) => {
                self.graph.types.canonical_function(previous)?
                    == self.graph.types.canonical_function(expected)?
            }
            _ => false,
        };
        if info.scoped != scoped || !info.fixed || !underlying_matches {
            return Err(Error::at(location, "enumeration redeclared differently"));
        }
        Ok(entity)
    }

    fn new_enum(
        &mut self,
        name: Option<crate::lex::identifiers::NameId>,
        info: EnumInfo,
        print: ScopeId,
    ) -> Result<(EntityId, BindingId), Error> {
        let entity = self.graph.add_entity(None, name, Keyword::Enum)?;
        self.record_unnamed_local_type(entity, info.owner)?;
        let ty = self
            .graph
            .types
            .intern(TypeKind::Enum(entity), Cv::default())?;
        let scope = self.graph.add_scope(
            ScopeKind::Enum,
            info.owner,
            if info.scoped { Some(print) } else { None },
            name,
        )?;
        self.graph.entity_mut(entity).ty = Some(ty);
        self.graph.entity_mut(entity).scope = Some(scope);
        self.graph.entity_mut(entity).enumeration = Some(info);
        let binding = self.graph.add_binding(
            info.owner,
            Some(print),
            Binding {
                kind: BindingKind::Type,
                name,
                entity,
                ty: Some(ty),
                key: Keyword::Enum,
            },
        )?;
        Ok((entity, binding))
    }
    pub(super) fn enumerators(
        &mut self,
        entity: EntityId,
        enumerators: crate::syntax::storage::List<ast::Enumerator>,
        print: ScopeId,
        declaration: BindingId,
        values: Option<ScopeId>,
    ) -> Result<(), Error> {
        let info = self
            .graph
            .entity(entity)
            .enumeration
            .ok_or(Error::Invariant("enum metadata"))?;
        let enum_ty = self
            .graph
            .entity(entity)
            .ty
            .ok_or(Error::Invariant("enum type"))?;
        let enum_scope = self
            .graph
            .entity(entity)
            .scope
            .ok_or(Error::Invariant("enum scope"))?;
        let lookup = if info.scoped { enum_scope } else { info.owner };
        let value_scope = values.unwrap_or(lookup);
        self.graph.entity_mut(entity).defined = true;
        let fixed_kind = info.underlying.and_then(|ty| self.integral_kind(ty));
        let dependent = info.fixed && fixed_kind.is_none();
        let mut previous: Option<Constant> = None;
        let mut values = Vec::new();
        for syntax_index in 0..self.tree.enumerators.get(enumerators).len() {
            let syntax_item = self.tree.enumerators.get(enumerators)[syntax_index];
            let enumerator = &syntax_item;
            let value = if self.template_depth != 0 {
                if let Some(expr) = enumerator.value {
                    self.expression(expr, value_scope)?;
                }
                if let Some(previous) = previous {
                    if dependent {
                        Constant {
                            bits: previous.bits.wrapping_add(1),
                            ..previous
                        }
                    } else {
                        self.next_enumerator(previous, info, enumerator.location)?
                    }
                } else {
                    Constant {
                        bits: 0,
                        ty: info
                            .underlying
                            .filter(|_| !dependent)
                            .unwrap_or(self.graph.types.scalar(Scalar::Basic(F::Int))?),
                    }
                }
            } else if let Some(expr) = enumerator.value {
                if self.semantic() {
                    self.integral_constant_operand(expr, value_scope, info.underlying)?;
                }
                let mut value = self
                    .constant_value(expr, value_scope, true)?
                    .ok_or(Error::Invariant("enumerator constant"))?;
                self.check_integral(value, self.tree.expressions.get(expr).location)?;
                if let Some(kind) = fixed_kind {
                    let from = self
                        .integral_kind(value.ty)
                        .ok_or(Error::Invariant("enumerator kind"))?;
                    if !kind.represents(value.bits, from) {
                        return Err(Error::at(
                            self.tree.expressions.get(expr).location,
                            "enumerator value is not representable in the underlying type",
                        ));
                    }
                    value = Constant {
                        bits: kind.convert(value.bits),
                        ty: info
                            .underlying
                            .ok_or(Error::Invariant("fixed underlying type"))?,
                    };
                } else {
                    value.ty = self.promoted_type(value.ty)?;
                }
                value
            } else if let Some(previous) = previous {
                self.next_enumerator(previous, info, enumerator.location)?
            } else {
                Constant {
                    bits: 0,
                    ty: if let Some(ty) = info.underlying {
                        ty
                    } else {
                        self.graph.types.scalar(Scalar::Basic(F::Int))?
                    },
                }
            };
            previous = Some(value);
            if self
                .graph
                .scope(lookup)
                .index
                .get(&enumerator.name)
                .is_some_and(|entry| {
                    entry.value.is_some()
                        || entry
                            .ty
                            .is_some_and(|id| self.graph.binding(id).kind != BindingKind::Type)
                })
            {
                return Err(Error::at(
                    enumerator.location,
                    "enumerator conflicts with an earlier declaration",
                ));
            }
            let constant =
                self.graph
                    .add_entity(Some(enum_ty), Some(enumerator.name), Keyword::Enum)?;
            self.graph.entity_mut(constant).constant = ConstantState::Value(value);
            let binding = Binding {
                kind: BindingKind::Enumerator,
                name: Some(enumerator.name),
                entity: constant,
                ty: Some(enum_ty),
                key: Keyword::Enum,
            };
            let id = self.graph.add_binding(lookup, Some(print), binding)?;
            self.graph.record_enum_declaration(id, declaration);
            if !info.scoped {
                let id = self.graph.add_binding(enum_scope, None, binding)?;
                self.graph.record_enum_declaration(id, declaration);
            }
            values.push(constant);
        }
        let underlying = if let Some(ty) = info.underlying {
            ty
        } else {
            let mut selected = F::UnsignedLongLongInt;
            for candidate in CANDIDATES {
                let kind = IntegralKind::of(Scalar::Basic(candidate))
                    .ok_or(Error::Invariant("enum candidate kind"))?;
                if values.iter().all(|&id| {
                    if let ConstantState::Value(value) = self.graph.entity(id).constant {
                        self.integral_kind(value.ty)
                            .is_some_and(|from| kind.represents(value.bits, from))
                    } else {
                        false
                    }
                }) {
                    selected = candidate;
                    break;
                }
            }
            self.graph.types.scalar(Scalar::Basic(selected))?
        };
        let negative = values.iter().any(|&id| {
            let ConstantState::Value(value) = self.graph.entity(id).constant else {
                return false;
            };
            self.integral_kind(value.ty)
                .is_some_and(|kind| kind.signed && super::integral::signed(value.bits) < 0)
        });
        self.graph.entity_mut(entity).enumeration = Some(EnumInfo {
            negative,
            underlying: Some(underlying),
            ..info
        });
        let kind = if dependent {
            IntegralKind {
                bits: 32,
                signed: true,
            }
        } else {
            self.integral_kind(underlying)
                .ok_or(Error::Invariant("final enum kind"))?
        };
        for id in values {
            let ConstantState::Value(value) = self.graph.entity(id).constant else {
                return Err(Error::Invariant("missing enum value"));
            };
            self.graph.entity_mut(id).constant = ConstantState::Value(Constant {
                bits: kind.convert(value.bits),
                ty: enum_ty,
            });
        }
        Ok(())
    }
    fn next_enumerator(
        &mut self,
        previous: Constant,
        info: EnumInfo,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let kind = self
            .integral_kind(previous.ty)
            .ok_or(Error::Invariant("previous enum kind"))?;
        if kind.signed && integral::signed(previous.bits) == i64::MAX
            || !kind.signed && previous.bits == u64::MAX
        {
            if info.fixed || !kind.signed {
                return Err(Error::at(location, "enumerator value overflows"));
            }
            return Ok(Constant {
                bits: previous.bits.wrapping_add(1),
                ty: self.graph.types.scalar(Scalar::Basic(F::UnsignedLongInt))?,
            });
        }
        let bits = previous.bits.wrapping_add(1);
        let from = IntegralKind {
            bits: 64,
            signed: kind.signed,
        };
        if let Some(ty) = info.underlying {
            let to = self
                .integral_kind(ty)
                .ok_or(Error::Invariant("fixed next enum kind"))?;
            if !to.represents(bits, from) {
                return Err(Error::at(
                    location,
                    "enumerator value is not representable in the underlying type",
                ));
            }
            return Ok(Constant {
                bits: to.convert(bits),
                ty,
            });
        }
        if kind.represents(bits, from) {
            return Ok(Constant {
                bits,
                ty: previous.ty,
            });
        }
        let from = IntegralKind {
            bits: 64,
            signed: kind.signed && integral::signed(bits) < 0,
        };
        for candidate in CANDIDATES {
            let to = IntegralKind::of(Scalar::Basic(candidate))
                .ok_or(Error::Invariant("next enum candidate"))?;
            if to.represents(bits, from) {
                return Ok(Constant {
                    bits,
                    ty: self.graph.types.scalar(Scalar::Basic(candidate))?,
                });
            }
        }
        Err(Error::Invariant("enum value without integer type"))
    }
}
