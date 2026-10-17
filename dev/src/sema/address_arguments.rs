// Translated from sema/template_addresses.cpp and class_templates.cpp::argument_target.
// See NOTICE.
//! Null address arguments share canonical Value identities with integral arguments.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::Root,
    function_templates::Substitution,
    graph::{Graph, ScopeId},
    templates::HeadId,
    types::{Cv, RefKind, Scalar, TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};

pub(super) enum ArgumentTarget {
    Unknown,
    Invalid,
    Type(TypeId),
}

impl ArgumentTarget {
    pub(super) fn known(self) -> Option<TypeId> {
        if let Self::Type(ty) = self {
            Some(ty)
        } else {
            None
        }
    }
}

impl Graph {
    pub(super) fn address_parameter_type(&self, ty: TypeId) -> bool {
        matches!(
            self.types.get(ty).kind,
            TypeKind::Scalar(Scalar::Nullptr)
                | TypeKind::Pointer(_)
                | TypeKind::MemberPointer { .. }
                | TypeKind::Reference {
                    kind: RefKind::Lvalue,
                    ..
                }
        )
    }
}

impl Analyzer<'_> {
    pub(super) fn argument_target(
        &mut self,
        head: HeadId,
        index: usize,
        earlier: &[TypeId],
    ) -> Result<ArgumentTarget, Error> {
        let parameter = self.graph.templates.parameters(head)[index];
        let Some(mut target) = self
            .graph
            .entity(parameter)
            .template_parameter
            .ok_or(Error::Invariant("argument without parameter"))?
            .value_type
        else {
            return Ok(ArgumentTarget::Unknown);
        };
        if self.graph.dependent_type(target) {
            if earlier.iter().any(|&ty| self.graph.dependent_type(ty)) {
                return Ok(ArgumentTarget::Unknown);
            }
            let given = earlier.iter().copied().map(Some).collect::<Vec<_>>();
            let mut environment = Substitution::new(self.graph.templates.depth(head), &given);
            if let TypeKind::Reference { element, .. } = self.graph.types.get(target).kind
                && self
                    .substitute_signature_type(element, &mut environment)?
                    .is_some_and(|ty| {
                        self.graph.types.get(ty).kind == TypeKind::Scalar(Scalar::Void)
                    })
            {
                return Err(Error::at(SourceOffset(0), "reference to void"));
            }
            let Some(substituted) = self.substitute_signature_type(target, &mut environment)?
            else {
                return Ok(ArgumentTarget::Invalid);
            };
            target = substituted;
        }
        Ok(if self.graph.dependent_type(target) {
            ArgumentTarget::Unknown
        } else {
            ArgumentTarget::Type(target)
        })
    }

    pub(super) fn null_template_argument(
        &mut self,
        expression: ast::ExprId,
        scope: ScopeId,
        target: Option<TypeId>,
    ) -> Result<TypeId, Error> {
        let ty = self.graph.expression_facts.get(expression)?.ty;
        let mut named = expression;
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
            named = inner;
        }
        if let ast::ExprKind::Id(name) = self.tree.expressions.get(named).kind
            && self.graph.resolved_name(name).is_some_and(|binding| {
                self.graph.binding(binding).kind == super::graph::BindingKind::Variable
            })
            && (target.is_some_and(|ty| {
                matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            }) || !self.graph.address_parameter_type(ty))
        {
            // A known address parameter designates this object instead of
            // reading its integral value. Non-null entity arguments are a
            // separate representation boundary, never an ordinary rejection.
            return Err(Error::Missing("non-null address template arguments"));
        }
        if !self.graph.address_parameter_type(ty) {
            // Even literal integer zero retains its integer type; conversion
            // to a pointer parameter diagnoses it where that parameter is known.
            let value = self.evaluate_constant(expression, scope, false)?;
            let ty = self.bare_type(value.ty)?;
            return self.graph.types.intern(
                TypeKind::Value {
                    ty,
                    bits: value.bits,
                },
                Cv::default(),
            );
        }
        let value = self.evaluate_address(expression, scope)?;
        if value.root != Root::Null {
            let address = matches!(self.tree.expressions.get(named).kind,
                ast::ExprKind::Unary { op, .. }
                    if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::BitAnd));
            if address
                || matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::Array { .. } | TypeKind::Function { .. }
                )
            {
                return Err(Error::Missing("non-null address template arguments"));
            }
            return Err(Error::at(
                self.tree.expressions.get(expression).location,
                "a template argument that designates no named object or function",
            ));
        }
        let ty = self.bare_type(ty)?;
        self.graph
            .types
            .intern(TypeKind::Value { ty, bits: 0 }, Cv::default())
    }

    pub(super) fn converted_null_argument(
        &mut self,
        target: TypeId,
        argument: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let TypeKind::Value { ty: source, .. } = self.graph.types.get(argument).kind else {
            return Err(Error::Invariant("null argument without value"));
        };
        let null_type = TypeKind::Scalar(Scalar::Nullptr);
        let failure = if !self.graph.address_parameter_type(source) {
            Some("an integral template argument given for a pointer or reference parameter")
        } else if !self.graph.address_parameter_type(target) {
            Some(
                if self.integral_kind(target).is_some()
                    || matches!(self.graph.types.get(target).kind, TypeKind::Enum(_))
                {
                    "a pointer or reference template argument given for an integral or enumeration parameter"
                } else {
                    "a non-type template parameter of a type 14.1p4 does not allow"
                },
            )
        } else if self.graph.types.get(target).kind == null_type {
            (self.graph.types.get(source).kind != null_type)
                .then_some("a std::nullptr_t template parameter's argument is not nullptr")
        } else if let TypeKind::MemberPointer { element, .. } = self.graph.types.get(target).kind {
            let same = source == target
                || !matches!(
                    self.graph.types.get(element).kind,
                    TypeKind::Function { .. }
                ) && matches!(
                    self.graph.types.get(source).kind,
                    TypeKind::MemberPointer { .. }
                ) && self.pointer_qualification(source, target)?;
            (self.graph.types.get(source).kind != null_type && !same).then_some(
                "a template argument does not convert to its pointer to member parameter's type",
            )
        } else if matches!(
            self.graph.types.get(target).kind,
            TypeKind::Reference { .. }
        ) {
            Some("a reference template parameter's argument names no object or function")
        } else {
            (self.graph.types.get(source).kind != null_type
                && source != target
                && !(matches!(self.graph.types.get(source).kind, TypeKind::Pointer(_))
                    && self.pointer_qualification(source, target)?))
            .then_some("a null pointer template argument of another pointer type")
        };
        if let Some(failure) = failure {
            return Err(Error::at(location, failure));
        }
        self.graph.types.intern(
            TypeKind::Value {
                ty: target,
                bits: 0,
            },
            Cv::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::{lookup::Filter, template_bodies::analyze_unit},
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn converted_null_arguments_reuse_shells_without_initializer_or_body_demand() {
        let source = SourceBuffer::new(
            b"template<int* P>struct A{static int f(){return 3;}};\
            using X=A<nullptr>;using Y=A<(int*)nullptr>;\
            template<class T,T V>struct B;using U=B<int*,nullptr>;using W=B<const int*,nullptr>;\
            template<decltype(nullptr) P>struct N;using Z=N<nullptr>;"
                .to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"null-identity").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            false,
        )
        .unwrap();
        let mut alias = |name| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
                .ty
                .unwrap()
        };
        assert_eq!(alias(b"X"), alias(b"Y"));
        assert_ne!(alias(b"U"), alias(b"W"));
        for name in [b"X", b"U", b"W", b"Z"] {
            let TypeKind::Class(entity) = graph.types.get(alias(name)).kind else {
                panic!("class shell")
            };
            assert!(graph.entity(entity).scope.is_none());
            let instance = graph.class_templates.instance(entity).unwrap();
            let argument = *instance.arguments.last().unwrap();
            let TypeKind::Value { ty, bits } = graph.types.get(argument).kind else {
                panic!("value argument")
            };
            assert!(graph.address_parameter_type(ty));
            assert_eq!(bits, 0);
            assert!(!instance.dependent);
        }
        assert!(graph.root_demands.as_slice().is_empty());
    }
}
