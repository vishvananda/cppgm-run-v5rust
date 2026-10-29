// Translated from sema/analyzer_traits.cpp::shape_trait/trait_constant. See NOTICE.
//! Classification consumes canonical types and demands only the queried class facts.
use super::{
    Error,
    analyzer::Analyzer,
    constants::Constant,
    expression_facts::{ExprFacts, ValueCategory},
    graph::ScopeId,
    integral::IntegralKind,
    types::{RefKind, Scalar, TypeId, TypeKind},
};
use crate::{
    lex::{predicates::TypePredicate as T, vocabulary::Keyword},
    support::source::SourceOffset,
    syntax::{ast, storage::List},
    types::FundamentalType as F,
};

/// Canonical query operands. Ordinary traits and short constructor queries
/// allocate no argument buffer; only long expanded packs need owned storage.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(super) enum PredicateTypes {
    Inline { values: [TypeId; 4], count: usize },
    Many(Vec<TypeId>),
}
impl PredicateTypes {
    fn new(first: TypeId) -> Self {
        Self::Inline {
            values: [first; 4],
            count: 1,
        }
    }
    fn push(&mut self, ty: TypeId) {
        match self {
            Self::Inline { values, count } if *count < values.len() => {
                values[*count] = ty;
                *count += 1;
            }
            Self::Inline { values, .. } => {
                let mut many = Vec::with_capacity(8);
                many.extend_from_slice(values);
                many.push(ty);
                *self = Self::Many(many);
            }
            Self::Many(values) => values.push(ty),
        }
    }
    pub(super) fn as_slice(&self) -> &[TypeId] {
        match self {
            Self::Inline { values, count } => &values[..*count],
            Self::Many(values) => values,
        }
    }
}

impl Analyzer<'_> {
    pub(super) fn predicate_expression(
        &mut self,
        id: ast::ExprId,
        predicate: T,
        syntax: List<ast::TypeId>,
        scope: ScopeId,
    ) -> Result<ExprFacts, Error> {
        let (types, _) = self.predicate_types(syntax, scope, false)?;
        let ty = self.predicate_result_type(predicate)?;
        let mut facts = self.new_expression_facts(ty, ValueCategory::Prvalue)?;
        let bits = self.predicate_value(id, predicate, &types, scope)?;
        facts.set_folded(Some(bits));
        super::record(
            &mut self.graph.constant_facts,
            id.index(),
            Constant {
                state: crate::sema::constant_addresses::ScalarState::Value,
                high: 0,
                bits,
                ty,
            },
        );
        Ok(facts)
    }

    pub(super) fn predicate_constant(
        &mut self,
        id: ast::ExprId,
        predicate: T,
        syntax: List<ast::TypeId>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Constant, Error> {
        let (types, dependent) = self.predicate_types(syntax, scope, true)?;
        if dependent {
            return Err(Error::NonConstant {
                failure: super::ConstantFailure::Reported,
                location,
                message: "a type trait of a dependent type",
            });
        }
        Ok(Constant {
            state: crate::sema::constant_addresses::ScalarState::Value,
            high: 0,
            bits: self.predicate_value(id, predicate, &types, scope)?,
            ty: self.predicate_result_type(predicate)?,
        })
    }

    fn predicate_result_type(&mut self, predicate: T) -> Result<TypeId, Error> {
        self.graph.types.scalar(if predicate == T::ArrayRank {
            Scalar::Basic(F::UnsignedLongInt)
        } else {
            Scalar::Bool
        })
    }

    fn predicate_types(
        &mut self,
        syntax: List<ast::TypeId>,
        scope: ScopeId,
        check_dependence: bool,
    ) -> Result<(PredicateTypes, bool), Error> {
        // Substitution expands the retained parsed type list. Check every
        // resulting type, as trait_types does, before consuming the operands.
        let mut types: Option<PredicateTypes> = None;
        let mut dependent = false;
        for index in 0..self.tree.type_lists.get(syntax).len() {
            let ty = self.type_syntax(self.tree.type_lists.get(syntax)[index], scope)?;
            if let Some(types) = &mut types {
                types.push(ty);
            } else {
                types = Some(PredicateTypes::new(ty));
            }
            dependent |= check_dependence && self.graph.dependent_type(ty);
        }
        Ok((
            types.ok_or(Error::Invariant("type predicate without an operand"))?,
            dependent,
        ))
    }

    fn predicate_value(
        &mut self,
        id: ast::ExprId,
        predicate: T,
        types: &PredicateTypes,
        scope: ScopeId,
    ) -> Result<u64, Error> {
        let location = self.tree.expressions.get(id).location;
        if predicate.is_expression() {
            return self
                .expression_predicate(id, predicate, types, scope)
                .map(u64::from);
        }
        let ty = types.as_slice()[0];
        let second = types.as_slice().get(1).copied();
        if predicate.is_property() {
            return self
                .property_predicate(predicate, ty, scope, location)
                .map(u64::from);
        }
        if let Some(value) = self.class_predicate_value(predicate, ty, second, location)? {
            return Ok(u64::from(value));
        }
        let kind = &self.graph.types.get(ty).kind;
        let reference = matches!(kind, TypeKind::Reference { .. });
        let function = matches!(kind, TypeKind::Function { .. });
        let void = matches!(kind, TypeKind::Scalar(Scalar::Void));
        let nullptr = matches!(kind, TypeKind::Scalar(Scalar::Nullptr));
        let integral = if let TypeKind::Scalar(scalar) = *kind {
            IntegralKind::of(scalar)
        } else {
            None
        };
        let floating = matches!(
            kind,
            TypeKind::Scalar(Scalar::Basic(f)) if f.floating_rank() != 0
        );
        let arithmetic = integral.is_some() || floating;
        let fundamental = arithmetic || void || nullptr;
        let member_function = matches!(kind, TypeKind::MemberPointer { element, .. }
            if matches!(self.graph.types.get(*element).kind, TypeKind::Function { .. }));
        let union = matches!(kind, TypeKind::Class(entity)
            if self.graph.entity(*entity).key == Keyword::Union);
        let value = match predicate {
            T::IsClass => matches!(kind, TypeKind::Class(_)) && !union,
            T::IsEnum => matches!(kind, TypeKind::Enum(_)),
            T::IsUnion => union,
            T::IsVoid => void,
            T::IsNullptr => nullptr,
            T::IsIntegral => integral.is_some(),
            T::IsFloatingPoint => floating,
            T::IsArray => matches!(kind, TypeKind::Array { .. }),
            T::IsBoundedArray => matches!(kind, TypeKind::Array { bound, operand, .. }
                if *bound != 0 || operand.is_some()),
            T::IsUnboundedArray => matches!(
                kind,
                TypeKind::Array {
                    bound: 0,
                    operand: None,
                    ..
                }
            ),
            T::IsPointer => matches!(kind, TypeKind::Pointer(_)),
            T::IsLvalueReference => matches!(
                kind,
                TypeKind::Reference {
                    kind: RefKind::Lvalue,
                    ..
                }
            ),
            T::IsRvalueReference => matches!(
                kind,
                TypeKind::Reference {
                    kind: RefKind::Rvalue,
                    ..
                }
            ),
            T::IsReference => reference,
            T::IsMemberObjectPointer => {
                matches!(kind, TypeKind::MemberPointer { .. }) && !member_function
            }
            T::IsMemberFunctionPointer => member_function,
            T::IsMemberPointer => matches!(kind, TypeKind::MemberPointer { .. }),
            T::IsFunction => function,
            T::IsArithmetic => arithmetic,
            T::IsFundamental => fundamental,
            T::IsObject => !function && !reference && !void,
            T::IsScalar => {
                arithmetic
                    || nullptr
                    || matches!(
                        kind,
                        TypeKind::Enum(_) | TypeKind::Pointer(_) | TypeKind::MemberPointer { .. }
                    )
            }
            T::IsCompound => !fundamental,
            T::IsConst => !reference && self.type_cv(ty).0 & 1 != 0,
            T::IsVolatile => !reference && self.type_cv(ty).0 & 2 != 0,
            T::IsSigned => floating || integral.is_some_and(|kind| kind.signed),
            T::IsUnsigned => integral.is_some_and(|kind| !kind.signed),
            T::IsReferenceable => {
                !void
                    && !matches!(kind, TypeKind::Function { cv, reference, .. }
                if cv.0 != 0 || reference.is_some())
            }
            T::IsScopedEnum => self.scoped_enum(ty),
            T::IsSame => {
                self.graph.types.canonical_view(ty)
                    == self
                        .graph
                        .types
                        .canonical_view(second.ok_or(Error::Invariant(
                            "binary type predicate without second operand",
                        ))?)
            }
            T::ArrayRank => {
                let mut rank = 0u64;
                let mut element = ty;
                while let TypeKind::Array { element: inner, .. } =
                    self.graph.types.get(element).kind
                {
                    rank += 1;
                    element = inner;
                }
                return Ok(rank);
            }
            T::IsEmpty
            | T::IsPolymorphic
            | T::IsAbstract
            | T::IsFinal
            | T::IsAggregate
            | T::HasVirtualDestructor
            | T::IsBaseOf
            | T::IsTrivial
            | T::IsTriviallyCopyable
            | T::IsStandardLayout
            | T::IsPod
            | T::IsLiteralType
            | T::HasTrivialDestructor
            | T::IsDestructible
            | T::IsNothrowDestructible
            | T::IsTriviallyDestructible
            | T::HasTrivialConstructor
            | T::HasTrivialCopy
            | T::HasTrivialAssign
            | T::HasNothrowConstructor
            | T::HasNothrowCopy
            | T::HasNothrowAssign => {
                return Err(Error::Invariant("class predicate without a result"));
            }
            T::IsAssignable | T::IsNothrowAssignable | T::IsTriviallyAssignable => {
                return Err(Error::Invariant("assignment predicate without a result"));
            }
            T::IsConvertible | T::IsConvertibleTo | T::IsNothrowConvertible => {
                return Err(Error::Invariant("conversion predicate without a result"));
            }
            T::IsInvocable | T::IsNothrowInvocable => {
                return Err(Error::Invariant("invocation predicate without a result"));
            }
            T::IsConstructible
            | T::IsNothrowConstructible
            | T::IsTriviallyConstructible
            | T::ReferenceBindsToTemporary
            | T::ReferenceConstructsFromTemporary
            | T::ReferenceConvertsFromTemporary => {
                return Err(Error::Invariant(
                    "initialization predicate without a result",
                ));
            }
        };
        Ok(u64::from(value))
    }
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
    fn classification_keeps_parsed_operands_and_leaves_class_definitions_undemanded() {
        let source = SourceBuffer::new(
            b"struct Incomplete;template<class T>struct Dormant{static_assert(sizeof(T)==0,\"undemanded\");};bool query(){return __is_class(Incomplete)&&__is_class(Dormant<int>);}static_assert(__is_same(const int,int const),\"canonical\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"shape-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let expressions = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let incomplete = analyzer.identifiers.intern(b"Incomplete").unwrap();
        let graph = analyzer.analyze(declarations).unwrap();
        let binding = graph
            .own(
                graph.global,
                incomplete,
                super::super::lookup::Filter::Elaborated,
            )
            .unwrap();
        assert!(matches!(
            graph.entity(graph.binding(binding).entity).layout,
            super::super::layout::LayoutState::NotStarted
        ));
        assert_eq!(tree.expressions.mark(), expressions);
        assert!(super::super::function_demands::trailing(&graph).is_empty());
        let ast::DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[2])
            .kind
        else {
            panic!("query body");
        };
        let ast::Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("parsed body");
        };
        let ast::StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound");
        };
        let ast::StmtKind::Return(Some(result)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return");
        };
        let ast::ExprKind::Binary { left, right, .. } = tree.expressions.get(result).kind else {
            panic!("predicates");
        };
        for id in [left, right] {
            let facts = graph.expression_facts.get(id).unwrap();
            assert_eq!(facts.folded(), Some(1));
            assert_eq!(facts.category, ValueCategory::Prvalue);
            assert!(matches!(
                graph.types.get(facts.ty).kind,
                TypeKind::Scalar(Scalar::Bool)
            ));
            assert!(matches!(
                tree.expressions.get(id).kind,
                ast::ExprKind::TypePredicate { .. }
            ));
            assert!(facts.call(&graph.expression_facts).is_none());
        }
    }
}
