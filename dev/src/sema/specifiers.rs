// Translated from sema/analyzer_types.cpp's specifiers/fundamental_type. See NOTICE.
use super::{
    Error,
    analyzer::Analyzer,
    graph::ScopeId,
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
#[derive(Clone, Copy)]
pub(super) struct Specifiers {
    pub ty: TypeId,
    pub declared: Option<TypeId>,
    pub role: DeclarationRole,
    pub is_extern: bool,
    pub is_static: bool,
    pub virtuality: Virtuality,
    pub constexpr: bool,
}
impl Analyzer<'_> {
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
        let mut cv = Cv::default();
        let mut typedef = false;
        let mut is_extern = false;
        let mut is_static = false;
        let mut virtuality = Virtuality::NonVirtual;
        let mut constexpr = false;
        let mut fundamental = Vec::new();
        for specifier in self.tree.specs.get(list) {
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
                    Keyword::Virtual => virtuality = Virtuality::Virtual,
                    Keyword::ThreadLocal | Keyword::Inline | Keyword::Register => (),
                    Keyword::Explicit if special => (),
                    Keyword::Auto | Keyword::Friend | Keyword::Explicit | Keyword::Mutable => {
                        return Err(Error::Missing(
                            "deduced types and extended declaration specifiers",
                        ));
                    }
                    _ => fundamental.push(key),
                }
            }
        }
        let mut ty = None;
        let mut declared = None;
        for &specifier in self.tree.specs.get(list) {
            if matches!(specifier.kind, SpecifierKind::Keyword(_)) {
                continue;
            }
            let named = self.specifier_type(specifier, scope, lone)?;
            if ty.is_some() || !fundamental.is_empty() {
                return Err(Error::at(
                    specifier.location,
                    "invalid combination of type specifiers",
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
        let location = self
            .tree
            .specs
            .get(list)
            .first()
            .map_or(crate::support::source::SourceOffset(0), |spec| {
                spec.location
            });
        let has_written_type = ty.is_some() || !fundamental.is_empty();
        let ty = if let Some(ty) = ty {
            ty
        } else {
            let scalar = if special && fundamental.is_empty() {
                Some(Scalar::Void)
            } else {
                fundamental_type(&fundamental)
            }
            .ok_or_else(|| Error::at(location, "invalid combination of type specifiers"))?;
            self.graph.types.scalar(scalar)?
        };
        let ty = self.graph.types.qualify(ty, cv)?;
        Ok((
            Specifiers {
                ty,
                declared,
                role: if typedef {
                    DeclarationRole::Typedef
                } else {
                    DeclarationRole::Ordinary
                },
                is_extern,
                is_static,
                virtuality,
                constexpr,
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
