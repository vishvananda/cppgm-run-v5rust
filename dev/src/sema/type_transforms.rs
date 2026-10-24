// Translated from sema/analyzer_traits.cpp::trait_type/transformed_type. See NOTICE.
//! Transform canonical types without demanding class layout or template bodies.
use super::{
    Error,
    analyzer::Analyzer,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    integral::IntegralKind,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::type_transforms::TypeTransform as T, support::source::SourceOffset, syntax::ast,
    types::FundamentalType as F,
};

impl Analyzer<'_> {
    pub(super) fn transform_expression(
        &mut self,
        id: ast::ExprId,
        transform: T,
        syntax: crate::syntax::storage::List<ast::TypeId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let ty = self.transform_operand_type(syntax, scope)?;
        let ty = self.transformed_type(transform, ty, self.tree.expressions.get(id).location)?;
        let (ty, category) = match self.graph.types.get(ty).kind {
            TypeKind::Reference { element, kind } => (
                element,
                match kind {
                    RefKind::Lvalue => ValueCategory::Lvalue,
                    RefKind::Rvalue => ValueCategory::Xvalue,
                },
            ),
            _ => (ty, ValueCategory::Prvalue),
        };
        self.new_expression_facts(ty, category)
    }

    fn transform_operand_type(
        &mut self,
        syntax: crate::syntax::storage::List<ast::TypeId>,
        scope: ScopeId,
    ) -> Result<TypeId, Error> {
        // ast_clone expands the type-id list before trait_types checks every
        // operand; the transform consumes the first after those checks.
        let mut first = None;
        for index in 0..self.tree.type_lists.get(syntax).len() {
            let ty = self.type_syntax(self.tree.type_lists.get(syntax)[index], scope)?;
            first.get_or_insert(ty);
        }
        first.ok_or(Error::Invariant("type transform without an operand"))
    }

    pub(super) fn transformed_type(
        &mut self,
        transform: T,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        if transform == T::UnderlyingType {
            let info = if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
                self.graph.entity(entity).enumeration
            } else {
                None
            };
            let Some(info) = info.filter(|info| info.underlying.is_some()) else {
                return Err(Error::at(
                    location,
                    "__underlying_type of a type that is no complete enumeration",
                ));
            };
            let underlying = info.underlying.ok_or(Error::Invariant(
                "complete enumeration without underlying type",
            ))?;
            if info.fixed || info.negative {
                return Ok(underlying);
            }
            let TypeKind::Scalar(scalar) = self.graph.types.get(underlying).kind else {
                return Err(Error::Invariant(
                    "enumeration without scalar underlying type",
                ));
            };
            return self.graph.types.scalar(counterpart(scalar, 0, false));
        }
        let bare = self.bare_type(ty)?;
        let abominable = matches!(self.graph.types.get(bare).kind,
            TypeKind::Function { cv, reference, .. } if cv.0 != 0 || reference.is_some());
        let void = matches!(
            self.graph.types.get(bare).kind,
            TypeKind::Scalar(Scalar::Void)
        );
        let referenceable = !abominable && !void;
        let referee = match self.graph.types.get(ty).kind {
            TypeKind::Reference { element, .. } => element,
            _ => ty,
        };
        match transform {
            T::AddAtomic => {
                if matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Array { .. } | TypeKind::Function { .. } | TypeKind::Reference { .. }
                ) || self.type_cv(ty).0 != 0
                    || void
                {
                    return Err(Error::at(
                        location,
                        "_Atomic of an array, function, reference, void or qualified type",
                    ));
                }
                self.graph.types.qualify(ty, Cv::ATOMIC)
            }
            T::AddLvalueReference | T::AddRvalueReference if referenceable => {
                self.graph.types.reference(
                    ty,
                    if transform == T::AddRvalueReference {
                        RefKind::Rvalue
                    } else {
                        RefKind::Lvalue
                    },
                )
            }
            T::AddPointer if referenceable || void => self.graph.types.pointer(referee),
            T::Decay => match self.graph.types.get(referee).kind {
                TypeKind::Array { element, .. } => self.graph.types.pointer(element),
                TypeKind::Function { .. } if !abominable => self.graph.types.pointer(referee),
                TypeKind::Function { .. } => Ok(ty),
                _ => self.bare_type(referee),
            },
            T::RemoveAllExtents => {
                let mut element = ty;
                while let TypeKind::Array { element: inner, .. } =
                    self.graph.types.get(element).kind
                {
                    element = inner;
                }
                Ok(element)
            }
            T::RemoveExtent => Ok(
                if let TypeKind::Array { element, .. } = self.graph.types.get(ty).kind {
                    element
                } else {
                    ty
                },
            ),
            T::RemoveConst | T::RemoveVolatile | T::RemoveCv | T::RemoveCvref => {
                let from = if transform == T::RemoveCvref {
                    referee
                } else {
                    ty
                };
                let mask = match transform {
                    T::RemoveConst => 1,
                    T::RemoveVolatile => 2,
                    _ => 3,
                };
                let cv = Cv(self.type_cv(from).0 & !mask);
                let bare = self.bare_type(from)?;
                self.graph.types.qualify(bare, cv)
            }
            T::RemovePointer => Ok(
                if let TypeKind::Pointer(element) = self.graph.types.get(bare).kind {
                    element
                } else {
                    ty
                },
            ),
            T::RemoveReference | T::RemoveReferenceGnu => Ok(referee),
            T::MakeSigned | T::MakeUnsigned => self.signed_transform(transform, ty, location),
            _ => Ok(ty),
        }
    }

    fn signed_transform(
        &mut self,
        transform: T,
        ty: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let enumeration = matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_));
        let integral = if let TypeKind::Enum(entity) = self.graph.types.get(ty).kind {
            self.graph
                .entity(entity)
                .enumeration
                .and_then(|info| info.underlying)
        } else {
            Some(ty)
        };
        let scalar = integral.and_then(|ty| {
            if let TypeKind::Scalar(scalar) = self.graph.types.get(ty).kind {
                Some(scalar)
            } else {
                None
            }
        });
        let Some(scalar) =
            scalar.filter(|&scalar| scalar != Scalar::Bool && IntegralKind::of(scalar).is_some())
        else {
            return Err(Error::at(
                location,
                "make_signed or make_unsigned of a type that is no integral or enumeration type",
            ));
        };
        let size = self
            .type_size(
                integral.ok_or(Error::Invariant("integral transform without type"))?,
                location,
            )?
            .0;
        let from = if enumeration { Scalar::Bool } else { scalar };
        let result =
            self.graph
                .types
                .scalar(counterpart(from, size, transform == T::MakeSigned))?;
        self.graph.types.qualify(result, self.type_cv(ty))
    }
}

fn counterpart(from: Scalar, size: u64, signed: bool) -> Scalar {
    let pairs = [
        (Scalar::SignedChar, Scalar::UnsignedChar),
        (Scalar::Short, Scalar::UnsignedShort),
        (Scalar::Basic(F::Int), Scalar::Basic(F::UnsignedInt)),
        (Scalar::Basic(F::LongInt), Scalar::Basic(F::UnsignedLongInt)),
        (
            Scalar::Basic(F::LongLongInt),
            Scalar::Basic(F::UnsignedLongLongInt),
        ),
        (Scalar::Basic(F::Int128), Scalar::Basic(F::UnsignedInt128)),
    ];
    let pair = pairs
        .iter()
        .find(|&&(a, b)| from == a || from == b)
        .copied()
        .unwrap_or(
            pairs[match size {
                1 => 0,
                2 => 1,
                4 => 2,
                8 => 3,
                _ => 5,
            }],
        );
    if signed { pair.0 } else { pair.1 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn types_only_specialization_preserves_unconditional_operand_failure_and_context() {
        let source = SourceBuffer::new(
            b"template<class T>using A=__make_unsigned(T);template<class T>A<T>f(T);template<>unsigned f(int);".to_vec(),
        ).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"transform-context").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        assert!(matches!(
            analyzer.declarations(declarations, global),
            Err(Error::UnconditionalDiagnostic {
                message: "unsupported decltype operand",
                ..
            })
        ));
        assert_eq!(analyzer.template_depth, 0);
        assert_eq!(analyzer.unevaluated, 0);
        assert!(analyzer.signature_object.is_none());
        assert!(analyzer.body_context.is_none());
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }
    #[test]
    fn expanded_transform_operands_keep_the_canonical_first_type() {
        let source = SourceBuffer::new(
            b"template<class...T>using A=__make_unsigned(T...);A<int,long>*p;".to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"transform-expansion").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let name = analyzer.identifiers.intern(b"p").unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let binding = graph
            .own(graph.global, name, super::super::lookup::Filter::Ordinary)
            .unwrap();
        let ty = graph.entity(graph.binding(binding).entity).ty.unwrap();
        let TypeKind::Pointer(element) = graph.types.get(ty).kind else {
            panic!("pointer");
        };
        assert_eq!(
            graph.types.get(element).kind,
            TypeKind::Scalar(Scalar::Basic(F::UnsignedInt))
        );
    }
}
