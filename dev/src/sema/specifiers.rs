// Translated from sema/analyzer_types.cpp's specifiers/fundamental_type. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    graph::{Mutability, ScopeId},
    types::{Cv, Scalar, TypeId},
};
use crate::{
    lex::vocabulary::Keyword,
    syntax::{
        ast::{Specifier, SpecifierKind},
        storage::List,
    },
    types::FundamentalType as F,
};
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum DeclarationRole {
    Ordinary,
    Typedef,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Virtuality {
    NonVirtual,
    Virtual,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ThreadStorage {
    Ordinary,
    ThreadLocal,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum StaticSpecifier {
    Absent,
    Present,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Explicitness {
    #[default]
    Converting,
    Explicit,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum InlinePreference {
    #[default]
    Unspecified,
    Inline,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Friendship {
    #[default]
    Ordinary,
    Friend,
}
#[derive(Clone, Copy)]
pub(super) struct Specifiers {
    pub dependent_uses: u64,
    pub alias_alignment: u32,
    pub friendship: Friendship,
    pub auto_cv: Cv,
    pub inline: InlinePreference,
    pub ty: TypeId,
    pub declared: Option<TypeId>,
    pub role: DeclarationRole,
    pub is_extern: bool,
    pub is_static: bool,
    pub mutability: Mutability,
    pub virtuality: Virtuality,
    pub constexpr: bool,
    pub explicitness: Explicitness,
    pub explicit_condition: Option<crate::syntax::ast::ExprId>,
    pub thread_storage: ThreadStorage,
}
impl Specifiers {
    pub(super) fn implicit(ty: TypeId, dependent_uses: u64) -> Self {
        Self {
            dependent_uses,
            alias_alignment: 0,
            friendship: Friendship::Ordinary,
            auto_cv: Cv::default(),
            inline: InlinePreference::Unspecified,
            ty,
            declared: None,
            role: DeclarationRole::Ordinary,
            is_extern: false,
            is_static: false,
            mutability: Mutability::Ordinary,
            virtuality: Virtuality::NonVirtual,
            constexpr: false,
            explicitness: Explicitness::Converting,
            explicit_condition: None,
            thread_storage: ThreadStorage::Ordinary,
        }
    }
}
impl Analyzer<'_> {
    // analyzer_declarations.cpp::declare_declarator: function-only specifiers
    // are checked before mutable and typedef restrictions, in source order.
    pub(super) fn check_function_specifiers(
        &self,
        specs: Specifiers,
        info: &super::declarators::DeclaratorInfo,
        init: crate::syntax::ast::InitDeclarator,
    ) -> Result<(), Error> {
        let function = specs.role != DeclarationRole::Typedef
            && matches!(
                self.graph.types.get(info.ty).kind,
                super::types::TypeKind::Function { .. }
            );
        let location = self.declarator_location(init.declarator)?;
        if specs.virtuality == Virtuality::Virtual && !function {
            return Err(Error::at(
                location,
                "virtual in a declaration that declares no function",
            ));
        }
        if specs.explicitness == Explicitness::Explicit && !function {
            return Err(Error::at(
                location,
                "explicit in a declaration that declares no function",
            ));
        }
        Ok(())
    }

    // Extended type specifiers consume a sign keyword in the shared sequence.
    fn consume_extended_specifier(
        &self,
        specifier: Specifier,
        named: TypeId,
        fundamental: &mut Vec<Keyword>,
        int128: &mut bool,
    ) -> bool {
        if !fundamental.is_empty()
            && !*int128
            && self.graph.types.get(named).kind
                == super::types::TypeKind::Scalar(Scalar::Basic(F::Int128))
            && matches!(specifier.kind, SpecifierKind::Named(name) if {
                let name = self.tree.names.get(name);
                let parts = self.tree.name_lists.get(name.components);
                !name.global && matches!(parts, [part] if matches!(part.kind, crate::syntax::ast::NameKind::Identifier(identifier) if identifier == self.int128_name) && part.arguments.is_none())
            })
        {
            *int128 = true;
            return true;
        }
        if matches!(specifier.kind, SpecifierKind::Decltype(expr)
                if matches!(self.tree.expressions.get(expr).kind, crate::syntax::ast::ExprKind::BitInt(_)))
        {
            fundamental.clear();
        }
        false
    }

    pub(super) fn specifiers(
        &mut self,
        list: List<Specifier>,
        scope: ScopeId,
        lone: bool,
    ) -> Result<Specifiers, Error> {
        self.specifiers_with_optional_type(list, scope, lone, false)
            .map(|(specifiers, _)| specifiers)
    }
    pub(super) fn special_specifiers(
        &mut self,
        list: List<Specifier>,
        scope: ScopeId,
    ) -> Result<(Specifiers, Option<TypeId>), Error> {
        self.specifiers_with_optional_type(list, scope, false, true)
    }
    fn specifiers_with_optional_type(
        &mut self,
        list: List<Specifier>,
        scope: ScopeId,
        lone: bool,
        special: bool,
    ) -> Result<(Specifiers, Option<TypeId>), Error> {
        let dependent_uses = self.dependent_uses.get();
        let mut friendship = Friendship::Ordinary;
        let mut cv = Cv::default();
        let mut typedef = false;
        let mut is_extern = false;
        let mut is_static = false;
        let mut mutability = Mutability::Ordinary;
        let mut virtuality = Virtuality::NonVirtual;
        let mut constexpr = false;
        let mut inline = InlinePreference::Unspecified;
        let mut explicitness = Explicitness::Converting;
        let mut explicit_condition = None;
        let mut thread_storage = ThreadStorage::Ordinary;
        let mut fundamental = Vec::new();
        let mut auto_type = false;
        for syntax_index in 0..self.tree.specs.get(list).len() {
            let syntax_item = self.tree.specs.get(list)[syntax_index];
            let specifier = &syntax_item;
            let conditional = match specifier.kind {
                SpecifierKind::ExplicitCondition(operand) => Some(operand),
                _ => None,
            };
            if conditional.is_some()
                || matches!(specifier.kind, SpecifierKind::Keyword(Keyword::Explicit))
            {
                if explicitness == Explicitness::Explicit
                    && (explicit_condition.is_some() || conditional.is_some())
                {
                    return Err(Error::at(
                        specifier.location,
                        "an explicit-specifier repeated in a decl-specifier-seq",
                    ));
                }
                explicit_condition = conditional;
                explicitness = Explicitness::Explicit;
            }
            if let SpecifierKind::Keyword(key) = specifier.kind {
                match key {
                    Keyword::Const | Keyword::Volatile => {
                        let bit = Self::cv(&[key]).0;
                        if cv.0 & bit != 0 {
                            return Err(Error::at(
                                specifier.location,
                                "a cv-qualifier repeated in a decl-specifier-seq (7.1.6p2)",
                            ));
                        }
                        cv.0 |= bit;
                    }
                    Keyword::Typedef => typedef = true,
                    Keyword::Extern => is_extern = true,
                    Keyword::Constexpr => constexpr = true,
                    Keyword::Static => is_static = true,
                    Keyword::Mutable => mutability = Mutability::Mutable,
                    Keyword::Virtual => virtuality = Virtuality::Virtual,
                    Keyword::ThreadLocal => thread_storage = ThreadStorage::ThreadLocal,
                    Keyword::Inline => inline = InlinePreference::Inline,
                    Keyword::Register => (),
                    Keyword::Explicit => explicitness = Explicitness::Explicit,
                    Keyword::Auto => auto_type = true,
                    Keyword::Friend => friendship = Friendship::Friend,
                    _ => fundamental.push(key),
                }
            }
        }
        let location = self
            .tree
            .specs
            .get(list)
            .first()
            .map_or(crate::support::source::SourceOffset(0), |spec| {
                spec.location
            });
        let mut int128 = false;
        let mut complex = false;
        let mut ty = None;
        let mut declared = None;
        let mut alias_alignment = 0;
        for syntax_index in 0..self.tree.specs.get(list).len() {
            let specifier = self.tree.specs.get(list)[syntax_index];
            if matches!(
                specifier.kind,
                SpecifierKind::Keyword(_)
                    | SpecifierKind::ExplicitCondition(_)
                    | SpecifierKind::Vector { .. }
            ) {
                continue;
            }
            if self.complex_specifier(specifier) {
                if complex {
                    return Err(Error::at(
                        specifier.location,
                        "_Complex repeated in a decl-specifier-seq",
                    ));
                }
                complex = true;
                continue;
            }
            let named = if friendship == Friendship::Friend {
                if matches!(specifier.kind, SpecifierKind::Class(_)) {
                    return Err(Error::at(
                        specifier.location,
                        "a class defined in a friend declaration",
                    ));
                }
                if let SpecifierKind::Elaborated { key, name } = specifier.kind {
                    self.elaborated_friend_type(key, name, scope)?
                } else {
                    self.specifier_type(specifier, scope, lone)?
                }
            } else {
                self.specifier_type(specifier, scope, lone)?
            };
            if let SpecifierKind::Named(name) = specifier.kind
                && let Some(binding) = self
                    .graph
                    .resolved_names
                    .get(name.index())
                    .copied()
                    .flatten()
            {
                alias_alignment = self.graph.alias_alignments.alias(binding);
            }
            if self.consume_extended_specifier(specifier, named, &mut fundamental, &mut int128) {
                continue;
            }
            if ty.is_some() || !fundamental.is_empty() {
                return Err(Error::at(
                    if ty.is_some() {
                        specifier.location
                    } else {
                        location
                    },
                    if int128 || self.integral_kind(named).is_some_and(|kind| kind.bits > 64) {
                        "more than one type in a declaration"
                    } else {
                        "invalid combination of type specifiers"
                    },
                ));
            }
            ty = Some(named);
            if matches!(
                specifier.kind,
                SpecifierKind::Class(_) | SpecifierKind::Enum(_)
            ) {
                declared = Some(named);
            }
        }
        let has_written_type = auto_type || complex || ty.is_some() || !fundamental.is_empty();
        let ty = if auto_type {
            if ty.is_some() || !fundamental.is_empty() {
                return Err(Error::at(location, "auto with another type specifier"));
            }
            self.graph
                .types
                .intern(super::types::TypeKind::Auto, Cv::default())?
        } else if let Some(ty) = ty {
            ty
        } else {
            let scalar = if int128 {
                if fundamental.len() != 1
                    || !matches!(fundamental[0], Keyword::Signed | Keyword::Unsigned)
                {
                    return Err(Error::at(
                        location,
                        "invalid combination of type specifiers",
                    ));
                }
                Some(Scalar::Basic(if fundamental[0] == Keyword::Unsigned {
                    F::UnsignedInt128
                } else {
                    F::Int128
                }))
            } else if complex && fundamental.is_empty() {
                Some(Scalar::Basic(F::Double))
            } else if special && fundamental.is_empty() {
                Some(Scalar::Void)
            } else {
                fundamental_type(&fundamental)
            }
            .ok_or_else(|| Error::at(location, "invalid combination of type specifiers"))?;
            self.graph.types.scalar(scalar)?
        };
        let ty = if complex {
            if !matches!(self.graph.types.get(ty).kind, super::types::TypeKind::Scalar(scalar)
                if !matches!(scalar, Scalar::Void | Scalar::Bool | Scalar::Nullptr))
            {
                return Err(Error::at(
                    location,
                    "_Complex of a type that is no arithmetic type",
                ));
            }
            let part = self.bare_type(ty)?;
            self.graph.types.intern(
                super::types::TypeKind::Complex(part),
                self.graph.types.get(ty).cv,
            )?
        } else {
            ty
        };
        let ty = if auto_type {
            ty
        } else {
            self.graph.types.qualify(ty, cv)?
        };
        let ty = self.vector_specifier_types(ty, list, scope)?;
        Ok((
            Specifiers {
                dependent_uses,
                alias_alignment,
                friendship,
                auto_cv: cv,
                inline,
                ty,
                declared,
                role: if typedef {
                    DeclarationRole::Typedef
                } else {
                    DeclarationRole::Ordinary
                },
                is_extern,
                is_static,
                mutability,
                virtuality,
                constexpr,
                explicitness,
                explicit_condition,
                thread_storage,
            },
            has_written_type.then_some(ty),
        ))
    }
}
fn fundamental_type(keys: &[Keyword]) -> Option<Scalar> {
    let count = |key| keys.iter().filter(|&&value| value == key).count();
    let signed = count(Keyword::Signed);
    let unsigned = count(Keyword::Unsigned);
    let short = count(Keyword::Short);
    let long = count(Keyword::Long);
    let int = count(Keyword::Int);
    let char_count = count(Keyword::Char);
    let double = count(Keyword::Double);
    let others: Vec<_> = keys
        .iter()
        .filter(|&&key| {
            !matches!(
                key,
                Keyword::Signed
                    | Keyword::Unsigned
                    | Keyword::Short
                    | Keyword::Long
                    | Keyword::Int
                    | Keyword::Char
                    | Keyword::Double
            )
        })
        .copied()
        .collect();
    if keys.is_empty()
        || signed > 1
        || unsigned > 1
        || signed + unsigned > 1
        || short > 1
        || long > 2
        || int > 1
        || char_count > 1
        || double > 1
        || others.len() > 1
        || short != 0 && long != 0
    {
        return None;
    }
    if let Some(&other) = others.first() {
        if keys.len() != 1 {
            return None;
        }
        return Some(match other {
            Keyword::Void => Scalar::Void,
            Keyword::Bool => Scalar::Bool,
            Keyword::Float => Scalar::Basic(F::Float),
            Keyword::WcharT => Scalar::Basic(F::WcharT),
            Keyword::Char16T => Scalar::Basic(F::Char16T),
            Keyword::Char32T => Scalar::Basic(F::Char32T),
            _ => return None,
        });
    }
    if char_count != 0 {
        if short + long + int + double != 0 {
            return None;
        }
        return Some(if signed != 0 {
            Scalar::SignedChar
        } else if unsigned != 0 {
            Scalar::UnsignedChar
        } else {
            Scalar::Basic(F::Char)
        });
    }
    if double != 0 {
        if signed + unsigned + short + int != 0 || long > 1 {
            return None;
        }
        return Some(Scalar::Basic(if long != 0 {
            F::LongDouble
        } else {
            F::Double
        }));
    }
    Some(match (short, long, unsigned) {
        (1, _, 0) => Scalar::Short,
        (1, _, _) => Scalar::UnsignedShort,
        (_, 2, 0) => Scalar::Basic(F::LongLongInt),
        (_, 2, _) => Scalar::Basic(F::UnsignedLongLongInt),
        (_, 1, 0) => Scalar::Basic(F::LongInt),
        (_, 1, _) => Scalar::Basic(F::UnsignedLongInt),
        (_, _, 0) => Scalar::Basic(F::Int),
        _ => Scalar::Basic(F::UnsignedInt),
    })
}
