// Translated from sema/template_addresses.cpp and class_templates.cpp::argument_target.
// See NOTICE.
//! Address arguments retain canonical target types and designated entity identities.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::Root,
    function_templates::Substitution,
    graph::{BindingKind, EntityId, Graph, ScopeId, ScopeKind},
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AddressIdentity {
    Null,
    Entity(EntityId),
}

impl Graph {
    pub(super) fn argument_address(&self, argument: TypeId) -> Option<AddressIdentity> {
        match self.types.get(argument).kind {
            TypeKind::Address { entity, .. } => Some(AddressIdentity::Entity(entity)),
            TypeKind::Value { ty, .. } if self.address_parameter_type(ty) => {
                Some(AddressIdentity::Null)
            }
            _ => None,
        }
    }

    pub(super) fn address_argument_unit_local(&self, entity: EntityId) -> Result<bool, Error> {
        let node = self.entity(entity);
        if node.function.is_some() {
            return self.lower_function_unit_local(entity);
        }
        Ok(self.lower_variable(entity)?.locality == super::lowering::Locality::Unit)
    }
    pub(super) fn address_argument_object_local(&self, entity: EntityId) -> Result<bool, Error> {
        let node = self.entity(entity);
        if node.function.is_some() {
            return self.lower_function_local_in_mode(entity, true);
        }
        let variable = node
            .variable
            .ok_or(Error::Invariant("address argument without variable"))?;
        if variable.internal_linkage {
            return Ok(true);
        }
        if let Some(class) = node
            .home
            .filter(|&home| self.scope(home).kind == ScopeKind::Class)
            .and_then(|home| self.scope(home).owner)
            && self.lower_object_type_unit_local(self.lower_entity_type(class)?)?
        {
            return Ok(true);
        }
        Ok(variable.language != super::linkage::Language::C
            && self.lower_object_type_unit_local(self.lower_entity_type(entity)?)?)
    }
    pub(super) fn reference_argument_entity(&self, binding: EntityId) -> Option<EntityId> {
        let argument = self.constant_addresses.argument(binding)?;
        if let TypeKind::Address { ty, entity } = self.types.get(argument).kind
            && matches!(self.types.get(ty).kind, TypeKind::Reference { .. })
        {
            Some(entity)
        } else {
            None
        }
    }

    fn linked_argument_entity(&self, entity: EntityId) -> bool {
        let record = self.entity(entity);
        if let Some(function) = record.function {
            return function.is_static
                || record
                    .home
                    .is_none_or(|home| self.scope(home).kind != ScopeKind::Class);
        }
        record.data_member != Some(super::graph::DataMemberKind::Instance)
            && record.variable.is_some_and(|variable| {
                variable.thread_storage == super::specifiers::ThreadStorage::Ordinary
                    && (variable.is_extern
                        || record.home.is_some_and(|home| {
                            matches!(
                                self.scope(home).kind,
                                ScopeKind::Namespace | ScopeKind::Class
                            )
                        }))
            })
    }

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
    fn note_argument_function(&mut self, function: EntityId) {
        use super::constant_addresses::ArgumentContext;
        if self.unevaluated != 0 || self.template_depth != 0 {
            return;
        }
        let context = if let Some(body) = &self.body_context {
            ArgumentContext::Function(body.entity)
        } else if let Some(owner) = self.initializer_demand_owner {
            ArgumentContext::Initializer(owner)
        } else {
            ArgumentContext::Unit
        };
        self.graph
            .constant_addresses
            .note_argument_function(context, function);
    }

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
            if earlier.iter().enumerate().any(|(index, &ty)| {
                self.argument_parameter_pack(head, index) || self.graph.dependent_type(ty)
            }) {
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
                // This probe precedes argument value formation, outside the
                // candidate's soft conversion. Preserve a dependent member's
                // diagnostic; an unreported substitution miss stays unknown.
                if let Some(message) = environment.failure {
                    return Err(Error::at(environment.failure_location, message));
                }
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

    pub(super) fn names_linked_integral_object(
        &self,
        mut expression: ast::ExprId,
    ) -> Result<bool, Error> {
        while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(expression).kind {
            expression = inner;
        }
        let ast::ExprKind::Id(name) = self.tree.expressions.get(expression).kind else {
            return Ok(false);
        };
        let Some(binding) = self.graph.resolved_name(name) else {
            return Ok(false);
        };
        let binding = self.graph.binding(binding);
        let ty = self.graph.expression_facts.get(expression)?.ty;
        Ok(binding.kind == BindingKind::Variable
            && self.graph.linked_argument_entity(binding.entity)
            && self.graph.entity(binding.entity).ty.is_some_and(|ty| {
                !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            })
            && (self.integral_kind(ty).is_some()
                || matches!(self.graph.types.get(ty).kind, TypeKind::Enum(_))))
    }

    pub(super) fn read_integral_argument(
        &mut self,
        argument: TypeId,
        target: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let TypeKind::Address { ty, entity } = self.graph.types.get(argument).kind else {
            return Ok(argument);
        };
        if !matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
            || self.graph.entity(entity).variable.is_none()
            || !(self.integral_kind(target).is_some()
                || matches!(self.graph.types.get(target).kind, TypeKind::Enum(_)))
        {
            return Ok(argument);
        }
        // Match variable_constant: only a const/constexpr member's initializer
        // can be demanded here. Taking its designation did not demand it.
        let variable = self.graph.entity(entity);
        if matches!(variable.constant, super::constants::ConstantState::None)
            && (variable.constexpr
                || variable
                    .ty
                    .is_some_and(|ty| self.graph.types.get(ty).cv.0 & Cv::CONST.0 != 0))
        {
            self.use_instance_variable(entity)?;
        }
        let super::constants::ConstantState::Value(value) = self.graph.entity(entity).constant
        else {
            return Err(Error::at(
                location,
                "a template argument that is not a constant expression",
            ));
        };
        let ty = self.bare_type(value.ty)?;
        self.graph.types.intern(
            TypeKind::Value {
                ty,
                bits: value.bits,
            },
            Cv::default(),
        )
    }

    pub(super) fn address_template_argument(
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
        // Only built-in & designates its operand. An overloaded operator is a
        // call whose result must satisfy the constant null-pointer rule.
        let address = self.graph.expression_facts.get(named)?.call.is_none()
            && matches!(self.tree.expressions.get(named).kind, ast::ExprKind::Unary { op, .. }
                if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::BitAnd));
        if address
            && let ast::ExprKind::Unary { operand, .. } = self.tree.expressions.get(named).kind
        {
            named = operand;
            while let ast::ExprKind::Paren(inner) = self.tree.expressions.get(named).kind {
                named = inner;
            }
        }
        if let ast::ExprKind::Id(name) = self.tree.expressions.get(named).kind
            && let Some(binding) = self
                .graph
                .expression_facts
                .get(named)?
                .function
                .or_else(|| self.graph.resolved_name(name))
        {
            let binding = self.graph.binding(binding);
            if address
                && matches!(
                    self.graph.types.get(ty).kind,
                    TypeKind::MemberPointer { .. }
                )
            {
                if !matches!(binding.kind, BindingKind::Variable | BindingKind::Function) {
                    return Err(Error::at(
                        self.tree.expressions.get(expression).location,
                        "a pointer to member template argument designates no member",
                    ));
                }
                if binding.kind == BindingKind::Function {
                    self.note_argument_function(binding.entity);
                }
                let ty = self.bare_type(ty)?;
                return self.graph.types.intern(
                    TypeKind::Address {
                        ty,
                        entity: binding.entity,
                    },
                    Cv::default(),
                );
            }
            if let Some(argument) = self.graph.constant_addresses.argument(binding.entity) {
                if !address {
                    return Ok(argument);
                }
                if let TypeKind::Address { ty: source, entity } =
                    self.graph.types.get(argument).kind
                    && matches!(
                        self.graph.types.get(source).kind,
                        TypeKind::Reference { .. }
                    )
                {
                    if self.graph.entity(entity).variable.is_some() {
                        self.use_instance_variable(entity)?;
                    } else {
                        self.note_argument_function(entity);
                    }
                    let ty = self.bare_type(ty)?;
                    return self
                        .graph
                        .types
                        .intern(TypeKind::Address { ty, entity }, Cv::default());
                }
            }
            let named_type = self.graph.expression_facts.get(named)?.ty;
            let read_pointer = !address
                && self.graph.address_parameter_type(named_type)
                && !target.is_some_and(|target| {
                    matches!(
                        self.graph.types.get(target).kind,
                        TypeKind::Reference { .. }
                    )
                });
            if matches!(binding.kind, BindingKind::Variable | BindingKind::Function)
                && !read_pointer
            {
                let entity = binding.entity;
                let failure = if !self.graph.linked_argument_entity(entity) {
                    Some(
                        "a template argument designates no object of static storage duration with linkage",
                    )
                } else if !address
                    && self.graph.entity(entity).ty.is_some_and(|ty| {
                        matches!(self.graph.types.get(ty).kind, TypeKind::Reference { .. })
                    })
                {
                    Some("a reference named as a template argument (14.3.2p1 names an object)")
                } else {
                    None
                };
                if let Some(message) = failure {
                    return Err(Error::at(
                        self.tree.expressions.get(named).location,
                        message,
                    ));
                }
                let readable = !address
                    && target.is_none()
                    && (self.integral_kind(named_type).is_some()
                        || matches!(self.graph.types.get(named_type).kind, TypeKind::Enum(_)));
                if binding.kind == BindingKind::Variable && !readable {
                    self.use_instance_variable(entity)?;
                } else if binding.kind == BindingKind::Function {
                    self.note_argument_function(entity);
                }
                let ty = if address {
                    self.bare_type(ty)?
                } else {
                    self.graph.types.reference(named_type, RefKind::Lvalue)?
                };
                return self
                    .graph
                    .types
                    .intern(TypeKind::Address { ty, entity }, Cv::default());
            }
        }
        if address {
            return Err(Error::at(
                self.tree.expressions.get(expression).location,
                "a template argument's address of something that is no named object or function",
            ));
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
        let null = if matches!(
            self.graph.types.get(ty).kind,
            TypeKind::MemberPointer { .. }
        ) {
            self.evaluate_member_pointer(expression, scope)?
                .member
                .is_none()
        } else {
            self.evaluate_address(expression, scope)?.root == Root::Null
        };
        if !null {
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

    pub(super) fn converted_address_argument(
        &mut self,
        target: TypeId,
        argument: TypeId,
        location: SourceOffset,
    ) -> Result<TypeId, Error> {
        let (source, entity) = match self.graph.types.get(argument).kind {
            TypeKind::Value { ty, .. } => (ty, None),
            TypeKind::Address { ty, entity } => (ty, Some(entity)),
            _ => return Err(Error::Invariant("address argument without value")),
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
            (entity.is_some() || self.graph.types.get(source).kind != null_type)
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
            (!(same || entity.is_none() && self.graph.types.get(source).kind == null_type)).then_some(
                "a template argument does not convert to its pointer to member parameter's type",
            )
        } else if let TypeKind::Reference {
            element: referred, ..
        } = self.graph.types.get(target).kind
        {
            if let TypeKind::Reference { element, .. } = self.graph.types.get(source).kind {
                (self.bare_type(element)? != self.bare_type(referred)?
                    || self.graph.types.get(element).cv.0 & !self.graph.types.get(referred).cv.0
                        != 0)
                    .then_some("a reference template parameter's argument of another type")
            } else {
                Some("a reference template parameter's argument names no object or function")
            }
        } else if entity.is_none() {
            (self.graph.types.get(source).kind != null_type
                && source != target
                && !(matches!(self.graph.types.get(source).kind, TypeKind::Pointer(_))
                    && self.pointer_qualification(source, target)?))
            .then_some("a null pointer template argument of another pointer type")
        } else {
            let pointer =
                if let TypeKind::Reference { element, .. } = self.graph.types.get(source).kind {
                    match self.graph.types.get(element).kind {
                        TypeKind::Array { element, .. } => Some(self.graph.types.pointer(element)?),
                        TypeKind::Function { .. } => Some(self.graph.types.pointer(element)?),
                        _ => None,
                    }
                } else {
                    Some(source)
                };
            if let Some(pointer) = pointer {
                (pointer != target && !self.pointer_qualification(pointer, target)?).then_some(
                    "a template argument does not convert to its pointer parameter's type",
                )
            } else {
                Some("a pointer template parameter's argument is an object without `&`")
            }
        };
        if let Some(failure) = failure {
            return Err(Error::at(location, failure));
        }
        let kind = entity.map_or(
            TypeKind::Value {
                ty: target,
                bits: 0,
            },
            |entity| TypeKind::Address { ty: target, entity },
        );
        self.graph.types.intern(kind, Cv::default())
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
    fn unknown_value_parameters_keep_object_identity_until_candidate_conversion() {
        let source = SourceBuffer::new(
            b"int x=3;const int y=7;\
            template<int& V>int pick(int){return V;}\
            template<int V>int pick(...){return V;}\
            template<class T>auto caller(T)->decltype(pick<x>(0)){return pick<x>(0);}\
            int main(){return caller(0)+pick<y>(0);}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unknown-object").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let pick = cursor.intern_name(b"pick").unwrap();
        let x = cursor.intern_name(b"x").unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let x = graph
            .binding(graph.own(graph.global, x, Filter::Ordinary).unwrap())
            .entity;
        let arguments = graph
            .function_templates
            .arguments
            .iter()
            .filter(|(entity, _)| graph.entity(**entity).name == Some(pick))
            .map(|(_, arguments)| arguments[0])
            .collect::<Vec<_>>();
        assert_eq!(arguments.len(), 2);
        assert!(
            arguments
                .iter()
                .any(|&argument| matches!(graph.types.get(argument).kind,
            TypeKind::Address { entity, .. } if entity == x))
        );
        assert!(arguments.iter().any(|&argument| matches!(
            graph.types.get(argument).kind,
            TypeKind::Value { bits: 7, .. }
        )));
    }

    #[test]
    fn function_argument_locality_inherits_a_const_objects_internal_linkage() {
        let source = SourceBuffer::new(
            b"const int y=7;template<const int& V>int f(){return V;}\
            int main(){return f<y>();}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"argument-locality").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let primary = graph
            .binding(graph.own(graph.global, f, Filter::Ordinary).unwrap())
            .entity;
        assert!(!graph.lower_function_unit_local(primary).unwrap());
        let (&instance, arguments) = graph
            .function_templates
            .arguments
            .iter()
            .find(|(entity, _)| graph.entity(**entity).name == Some(f))
            .unwrap();
        assert!(graph.lower_type_unit_local(arguments[0]).unwrap());
        assert!(graph.lower_function_unit_local(instance).unwrap());
        assert!(graph.address_argument_unit_local(instance).unwrap());
    }

    #[test]
    fn absent_qualified_operator_is_a_substitution_failure() {
        let source = SourceBuffer::new(
            b"void* operator new(unsigned long);enum E{};\
            template<class T,void*(*P)(unsigned long)=&T::operator new>char f(int);\
            template<class>long f(...);static_assert(sizeof(f<E>(0))==sizeof(long),\"\");"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"missing-operator").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
    }

    #[test]
    fn target_probe_stops_before_substitution_past_a_parameter_pack() {
        let source = SourceBuffer::new(
            b"struct X{};template<class... Ts,class T,typename T::missing V>void f();".to_vec(),
        )
        .unwrap();
        let mut unit = SyntaxUnit::new(Preprocessor::with_name(&source, b"target-pack").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"f").unwrap();
        let class_name = cursor.intern_name(b"X").unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        let function = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
        let entity = analyzer.graph.binding(function).entity;
        let head = analyzer.graph.entity(entity).template_head.unwrap();
        let class = analyzer
            .graph
            .own(global, class_name, Filter::Ordinary)
            .unwrap();
        let class = analyzer.graph.binding(class).ty.unwrap();
        let empty = analyzer
            .graph
            .types
            .intern(TypeKind::Pack(Vec::new()), Cv::default())
            .unwrap();
        assert!(matches!(
            analyzer.argument_target(head, 2, &[empty, class]).unwrap(),
            ArgumentTarget::Unknown
        ));
        assert!(analyzer.graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn converted_address_arguments_reuse_shells_without_initializer_or_body_demand() {
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

    #[test]
    fn address_arguments_keep_entity_and_target_identity_without_completing_shells() {
        let source = SourceBuffer::new(b"static int x;int y;template<int* V>struct P;template<const int* V>struct C;template<int& V>struct R;using A=P<&x>;using B=P<(&x)>;using D=P<&y>;using E=C<&x>;using F=R<x>;".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"address-identity").unwrap());
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
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .ty
                .unwrap()
        };
        assert_eq!(alias(b"A"), alias(b"B"));
        let TypeKind::Class(class) = graph.types.get(alias(b"A")).kind else {
            panic!("class argument")
        };
        assert!(graph.lower_entity_unit_local(class).unwrap());
        assert!(!graph.lower_class_no_linkage(class).unwrap());
        assert_ne!(alias(b"A"), alias(b"D"));
        let mut values = Vec::new();
        for name in [b"A", b"D", b"E", b"F"] {
            let TypeKind::Class(entity) = graph.types.get(alias(name)).kind else {
                panic!("class shell")
            };
            assert!(graph.entity(entity).scope.is_none());
            let argument = graph.class_templates.instance(entity).unwrap().arguments[0];
            let TypeKind::Address { ty, entity } = graph.types.get(argument).kind else {
                panic!("address argument")
            };
            values.push((ty, entity));
        }
        assert_ne!(values[0].1, values[1].1);
        assert_eq!(values[0].1, values[2].1);
        assert_eq!(values[0].1, values[3].1);
        assert_ne!(values[0].0, values[2].0);
        assert!(matches!(
            graph.types.get(values[3].0).kind,
            TypeKind::Reference { .. }
        ));
        assert!(graph.root_demands.as_slice().is_empty());
    }

    #[test]
    fn function_argument_demand_is_deduplicated_and_owned_by_its_definition() {
        let source = SourceBuffer::new(b"inline int f(int x){return x;}inline int g(int x){return x;}template<int(*F)(int)>struct H{};using Root=H<f>;using Again=H<&f>;void owner(){using A=H<g>;using B=H<&g>;using C=H<f>;}void other(){(void)sizeof((H<&g>()));}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"argument-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let mut entity = |name| {
            let name = cursor.intern_name(name).unwrap();
            graph
                .binding(
                    graph
                        .own(graph.global, name, super::super::lookup::Filter::Ordinary)
                        .unwrap(),
                )
                .entity
        };
        let (f, g, owner, other) = (
            entity(b"f"),
            entity(b"g"),
            entity(b"owner"),
            entity(b"other"),
        );
        assert_eq!(
            graph.lower_argument_functions(None).collect::<Vec<_>>(),
            [f]
        );
        assert_eq!(
            graph
                .lower_argument_functions(Some(owner))
                .collect::<Vec<_>>(),
            [f, g]
        );
        assert_eq!(graph.lower_argument_functions(Some(other)).count(), 0);
    }

    #[test]
    fn storage_only_initializer_arguments_keep_their_origin_across_instantiations() {
        let source = SourceBuffer::new(
            b"inline int target(){return 1;}\
              template<int(*F)()>struct Token{};\
              template<class T>struct Box{static int value;};\
              template<class T>int Box<T>::value=sizeof(Token<&T::f>);\
              struct A{static int f(){return target();}};\
              inline int unused(){return Box<A>::value;}\
              int main(){return 0;}"
                .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"initializer-demand").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        assert_eq!(graph.lower_argument_functions(None).count(), 0);
        let variables = graph.lower_demanded_variables();
        assert_eq!(variables.len(), 1);
        let functions: Vec<_> = graph
            .lower_initializer_argument_functions(variables[0])
            .collect();
        assert_eq!(functions.len(), 1);
        let name = graph.entity(functions[0]).name.unwrap();
        assert_eq!(cursor.name(name), b"f");
        // The function's own calls remain body-owned; they are not promoted
        // just because its address is an initializer's template argument.
        assert_eq!(
            graph.lower_argument_functions(Some(functions[0])).count(),
            0
        );
    }

    #[test]
    fn function_reference_argument_retains_the_selected_direct_call_target() {
        let source = SourceBuffer::new(b"int f(int x){return x;}template<int(&F)(int)>int call(int x){return F(x);}int main(){return call<f>(1);}".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"function-reference").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let graph = analyze_unit(
            tree,
            cursor,
            declarations,
            StackBudget::default(),
            true,
            true,
        )
        .unwrap();
        let f = cursor.intern_name(b"f").unwrap();
        let f = graph
            .binding(
                graph
                    .own(graph.global, f, super::super::lookup::Filter::Ordinary)
                    .unwrap(),
            )
            .entity;
        let call = cursor.intern_name(b"call").unwrap();
        let instance = graph
            .function_templates
            .arguments
            .keys()
            .copied()
            .find(|&entity| graph.entity(entity).name == Some(call))
            .unwrap();
        let ast::Body::Parsed(body) = tree
            .bodies
            .get(graph.lower_function_body(instance).unwrap())
        else {
            panic!("instantiated body")
        };
        let ast::StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
            panic!("compound")
        };
        let ast::StmtKind::Return(Some(call)) =
            tree.statements.get(tree.stmt_lists.get(items)[0]).kind
        else {
            panic!("return call")
        };
        assert!(
            matches!(graph.lower_call(call).unwrap(), super::super::lowering::CallTarget::Direct(entity) if entity == f)
        );
    }
}
