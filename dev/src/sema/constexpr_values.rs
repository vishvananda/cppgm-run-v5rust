// Translated from constant_evaluator.cpp::record_constant_initializer and
// constant_interpreter.cpp::initialize/publish/root_storage. See NOTICE.
//! Declaration-owned aggregate values survive the evaluation that produced them.
use super::{
    Error,
    analyzer::Analyzer,
    constant_addresses::{Address, Root, TemporaryValue as Value, nonconstant},
    constexpr_frames::Frozen,
    graph::{EntityId, ScopeId},
    types::{TypeId, TypeKind},
};
use crate::{support::source::SourceOffset, syntax::ast};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TemporaryId(u32);

pub(super) struct Temporary {
    pub ty: TypeId,
    pub value: Frozen,
    pub image: Option<Vec<super::static_arrays::Item>>,
}

#[derive(Default)]
pub(super) struct Values {
    complete: HashMap<EntityId, Frozen>,
    constructing: HashMap<EntityId, Address>,
    static_roots: std::collections::HashSet<Root>,
    temporaries: Vec<Temporary>,
}

impl Values {
    pub(super) fn get(&self, entity: EntityId) -> Option<&Frozen> {
        self.complete.get(&entity)
    }
    pub(super) fn constructing(&self, entity: EntityId) -> Option<Address> {
        self.constructing.get(&entity).copied()
    }
    pub(super) fn temporary(&self, id: TemporaryId) -> &Temporary {
        &self.temporaries[id.0 as usize]
    }
    pub(super) fn static_target(&self, place: Address) -> bool {
        self.static_roots.contains(&place.root)
    }
}
impl Analyzer<'_> {
    pub(super) fn initialized_scalar_value(
        &mut self,
        ty: TypeId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<Value, Error> {
        let region = self.constexpr_frames.region();
        let result = (|| {
            let place = self.allocate_constant_storage(ty, None, initializer.location)?;
            self.constexpr_frames.initialize_declaration(place)?;
            self.construct_constant_object(
                place,
                Some(initializer),
                None,
                scope,
                initializer.location,
            )?;
            // Publication observes the completed storage, rather than reading
            // a cv-qualified source glvalue. Indeterminate arithmetic values
            // remain available for the declaration's completeness check.
            let place = Address {
                designated: self.bare_type(ty)?,
                ..place
            };
            if matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Pointer(_)
                    | TypeKind::MemberPointer { .. }
                    | TypeKind::Scalar(super::types::Scalar::Nullptr)
            ) {
                self.constexpr_frames
                    .read_value(
                        &self.graph.constant_member_spans,
                        place,
                        initializer.location,
                    )?
                    .ok_or_else(|| {
                        nonconstant(
                            initializer.location,
                            "a constexpr object leaves a subobject uninitialized",
                        )
                    })
            } else {
                self.constant_arithmetic_value(place, initializer.location)
            }
        })();
        self.constexpr_frames.release(region);
        result
    }
    pub(super) fn integral_initializer_value(
        &mut self,
        entity: EntityId,
        initializer: ast::Initializer,
        scope: ScopeId,
    ) -> Result<Option<super::constants::Constant>, Error> {
        let ty = self.graph.lower_entity_type(entity)?;
        let result = self.in_constant_evaluation(|analyzer| {
            // Initialization copies indeterminate arithmetic values; the owning
            // declaration diagnoses completeness after construction.
            let Value::Integer(value) =
                analyzer.initialized_scalar_value(ty, initializer, scope)?
            else {
                return Err(Error::Invariant("integral initializer holds noninteger"));
            };
            Ok(value)
        });
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error @ Error::NonConstant { .. }) if self.graph.entity(entity).constexpr => {
                Err(error.constant_initializer(initializer.location))
            }
            Err(Error::NonConstant { .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
    pub(super) fn constant_binds_temporary(&self, id: ast::ExprId) -> Result<bool, Error> {
        let mut id = self.graph.expression_facts.root(id);
        if let Some(step) = self.graph.expression_facts.function_conversion(id) {
            let function = self.graph.binding(step.binding).entity;
            let TypeKind::Function { result, .. } = self
                .graph
                .types
                .get(self.graph.lower_function(function)?.ty)
                .kind
            else {
                return Err(Error::Invariant("conversion without function type"));
            };
            return Ok(!matches!(
                self.graph.types.get(result).kind,
                TypeKind::Reference { .. }
            ));
        }
        let facts = self.graph.expression_facts.get(id)?;
        if self.graph.expression_facts.class_conversion(id).is_some()
            || facts
                .conversion
                .is_some_and(|conversion| conversion.temporary)
        {
            return Ok(true);
        }
        for _ in 0..256 {
            let facts = self.graph.expression_facts.get(id)?;
            if facts.category == super::expression_facts::ValueCategory::Prvalue {
                return Ok(!matches!(
                    self.graph.types.get(facts.ty).kind,
                    TypeKind::Function { .. }
                ));
            }
            id = match self.tree.expressions.get(id).kind {
                ast::ExprKind::Member { base, op, .. }
                    if op.token.as_punctuator() == Some(crate::lex::pp_lexer::Punctuator::Dot)
                        && facts.member.is_some_and(|binding| {
                            let entity = self.graph.binding(binding).entity;
                            self.graph.entity(entity).data_member
                                == Some(super::graph::DataMemberKind::Instance)
                                && self.graph.entity(entity).ty.is_some_and(|ty| {
                                    !matches!(
                                        self.graph.types.get(ty).kind,
                                        TypeKind::Reference { .. }
                                    )
                                })
                        }) =>
                {
                    base
                }
                ast::ExprKind::Subscript { base, .. }
                    if !facts.commuted
                        && facts.call_form != super::expression_facts::CallForm::Operator
                        && matches!(
                            self.graph
                                .types
                                .get(self.graph.expression_facts.get(base)?.ty)
                                .kind,
                            TypeKind::Array { .. }
                        ) =>
                {
                    base
                }
                _ => return Ok(false),
            };
            id = self.graph.expression_facts.root(id);
        }
        Ok(false)
    }
    pub(super) fn extend_constant_temporary(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Address, Error> {
        let Root::Object(object) = place.root else {
            return Ok(place);
        };
        let ty = self.constexpr_frames.ty(object, location)?;
        // skeleton() keeps an array's type; scalar/class values carry their
        // unqualified type, independent of the reference's binding cv.
        let ty = if matches!(self.graph.types.get(ty).kind, TypeKind::Array { .. }) {
            ty
        } else {
            self.bare_type(ty)?
        };
        let root = Address::new(place.root, self.bare_type(ty)?);
        let value = self.constexpr_frames.freeze(
            &self.graph.constant_member_spans,
            root,
            location,
            |value| {
                if let Value::Pointer(pointer) = value
                    && !self.address_publishable(pointer)
                {
                    return Err(nonconstant(
                        location,
                        "a constant designates an object without static storage duration",
                    ));
                }
                Ok(value)
            },
        )?;
        if !value.complete(|ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Class(_) | TypeKind::Complex(_) | TypeKind::Array { .. }
            ) || self.constant_lane_vector(ty).is_some()
        }) {
            return Err(nonconstant(
                location,
                "a temporary bound to a reference leaves a subobject uninitialized",
            ));
        }
        // constant_temporary uses constant_global for every non-class/array
        // object. Its scalar item cannot state vectors, complex values, member
        // pointers or binary16/128 values. Keep the core value and runtime path.
        let image = if self.vector_shape(ty).is_some()
            || self.complex_part_type(ty).is_some()
            || matches!(
                self.graph.types.get(ty).kind,
                TypeKind::MemberPointer { .. }
            )
            || matches!(self.graph.types.get(ty).kind, TypeKind::Scalar(super::types::Scalar::Basic(kind))
                if matches!(kind.floating_representation(), crate::types::FundamentalType::Float16 | crate::types::FundamentalType::Float128))
        {
            None
        } else {
            self.completed_image(ty, &value)?
        };
        let id = TemporaryId(
            u32::try_from(self.graph.aggregate_constants.temporaries.len())
                .map_err(|_| Error::Limit("constant temporary limit"))?,
        );
        self.graph
            .aggregate_constants
            .temporaries
            .push(Temporary { ty, value, image });
        place.root = Root::Published(id);
        Ok(place)
    }
    pub(super) fn record_aggregate_constant(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        required: bool,
    ) -> Result<(), Error> {
        let location = initializer.map_or(SourceOffset(0), |i| i.location);
        let ty = self.graph.lower_entity_type(entity)?;
        if !matches!(
            self.graph.types.get(ty).kind,
            TypeKind::Class(_)
                | TypeKind::Complex(_)
                | TypeKind::Vector { .. }
                | TypeKind::Array { .. }
        ) || self.graph.aggregate_constants.get(entity).is_some()
        {
            return Ok(());
        }
        if required && !self.constant_literal_type(ty)? {
            return Err(Error::at(
                location,
                "a constexpr variable of a type that is not a literal type",
            ));
        }
        let result = self.in_constant_evaluation(|analyzer| {
            let region = analyzer.constexpr_frames.region();
            let result =
                analyzer.evaluate_aggregate_initializer(entity, initializer, scope, location);
            if let Some(place) = analyzer
                .graph
                .aggregate_constants
                .constructing
                .remove(&entity)
            {
                analyzer
                    .graph
                    .aggregate_constants
                    .static_roots
                    .remove(&place.root);
            }
            analyzer.constexpr_frames.release(region);
            result
        });
        match result {
            Ok(value) => {
                if required && value.values().any(|(_, _, _, value)|
                    matches!(value, Some(Value::MemberPointer(value)) if value.provisional())) {
                    self.deferred.push(super::analyzer::DeferredMember::AggregateConstant(entity, initializer, scope));
                }
                self.graph
                    .aggregate_constants
                    .complete
                    .insert(entity, value);
            }
            Err(error @ Error::NonConstant { .. }) if required => {
                return Err(error.constant_initializer(location));
            }
            Err(Error::NonConstant { .. } | Error::Missing(_)) if !required => (),
            Err(error) => return Err(error),
        }
        Ok(())
    }
    pub(super) fn finish_deferred_aggregate_constant(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
    ) -> Result<(), Error> {
        self.graph.aggregate_constants.complete.remove(&entity);
        self.record_aggregate_constant(entity, initializer, scope, true)
    }
    fn evaluate_aggregate_initializer(
        &mut self,
        entity: EntityId,
        initializer: Option<ast::Initializer>,
        scope: ScopeId,
        location: SourceOffset,
    ) -> Result<Frozen, Error> {
        if self
            .graph
            .aggregate_constants
            .constructing(entity)
            .is_some()
        {
            return Err(nonconstant(
                location,
                "a read of an object that is not initialized",
            ));
        }
        let ty = self.graph.lower_entity_type(entity)?;
        let place = self.allocate_constant_storage(ty, None, location)?;
        self.constexpr_frames.initialize_declaration(place)?;
        self.graph
            .aggregate_constants
            .constructing
            .insert(entity, place);
        if !self.graph.lower_automatic(entity) && !self.graph.lower_variable(entity)?.thread_local {
            self.graph
                .aggregate_constants
                .static_roots
                .insert(place.root);
        }
        let constructor = self.graph.lower_construction(entity);
        self.construct_constant_object(place, initializer, constructor, scope, location)?;
        let storage = Address {
            designated: self.bare_type(place.designated)?,
            ..place
        };
        let value = self.constexpr_frames.freeze(&self.graph.constant_member_spans, storage, location, |value| {
            let Value::Pointer(mut pointer) = value else {
                return Ok(value);
            };
            if pointer.root == place.root {
                pointer.root = Root::Variable(entity);
            }
            if matches!(pointer.root, Root::Object(_))
                || matches!(pointer.root, Root::Variable(root) if self.graph.lower_automatic(root)
                    || self.graph.lower_variable(root).is_ok_and(|v| v.thread_local))
            {
                return Err(nonconstant(
                    location,
                    "a constant designates an object without static storage duration",
                ));
            }
            Ok(Value::Pointer(pointer))
        })?;
        if !value.complete(|ty| {
            matches!(
                self.graph.types.get(ty).kind,
                TypeKind::Class(_) | TypeKind::Complex(_) | TypeKind::Array { .. }
            ) || self.constant_lane_vector(ty).is_some()
        }) {
            return Err(nonconstant(
                location,
                "a constexpr object leaves a subobject uninitialized",
            ));
        }
        Ok(value)
    }
    pub(super) fn published_scalar(
        &mut self,
        mut place: Address,
        location: SourceOffset,
    ) -> Result<Option<Value>, Error> {
        if let Root::Published(id) = place.root {
            if self.type_cv(place.designated) != super::types::Cv::CONST {
                return Err(nonconstant(
                    location,
                    "an access to a temporary that is not const, bound outside the evaluation",
                ));
            }
            place.designated = self.bare_type(place.designated)?;
            return self
                .graph
                .aggregate_constants
                .temporary(id)
                .value
                .read(
                    &self.graph.constant_member_spans,
                    place,
                    self.constexpr_frames.designation(place.variant()),
                    location,
                )
                .map(Some);
        }
        let Root::Variable(entity) = place.root else {
            return Ok(None);
        };
        self.instantiate_variable_value(entity)?;
        if !self.graph.entity(entity).constexpr {
            return Ok(None);
        }
        if self.type_cv(self.graph.lower_entity_type(entity)?).0 & 2 != 0 {
            return Err(nonconstant(location, "a read of a volatile object"));
        }
        place.designated = self.bare_type(place.designated)?;
        self.graph
            .aggregate_constants
            .get(entity)
            .map(|value| {
                value.read(
                    &self.graph.constant_member_spans,
                    place,
                    self.constexpr_frames.designation(place.variant()),
                    location,
                )
            })
            .transpose()
    }
    pub(super) fn copy_constant_object(
        &mut self,
        mut target: Address,
        mut source: Address,
        location: SourceOffset,
    ) -> Result<(), Error> {
        target.designated = self.bare_type(target.designated)?;
        source.designated = self.bare_type(source.designated)?;
        if let Root::Published(id) = source.root {
            return self.constexpr_frames.copy_published(
                &self.graph.constant_member_spans,
                target,
                source,
                &self.graph.aggregate_constants.temporary(id).value,
                location,
            );
        }
        if let Root::Variable(entity) = source.root {
            self.instantiate_variable_value(entity)?;
            if self.graph.entity(entity).constexpr
                && let Some(value) = self.graph.aggregate_constants.get(entity)
            {
                return self.constexpr_frames.copy_published(
                    &self.graph.constant_member_spans,
                    target,
                    source,
                    value,
                    location,
                );
            }
            return Err(nonconstant(
                location,
                "a variable is not usable in a constant expression",
            ));
        }
        self.constexpr_frames
            .copy(&self.graph.constant_member_spans, target, source, location)
    }
}

impl super::graph::Graph {
    /// A successfully evaluated object can still lack the frozen static-data
    /// representation (shared/split bit units). Keep its semantic value while
    /// allowing the owning initializer to retain runtime storage construction.
    pub(super) fn static_temporary_value(
        &self,
        id: TemporaryId,
        offset: i64,
    ) -> Option<super::static_initialization::Value> {
        self.aggregate_constants.temporary(id).image.as_ref()?;
        Some(super::static_initialization::Value::Temporary(id, offset))
    }
    pub(crate) fn lower_constant_temporary(
        &self,
        id: TemporaryId,
    ) -> Result<(TypeId, &[super::static_arrays::Item]), Error> {
        let temporary = self.aggregate_constants.temporary(id);
        Ok((
            temporary.ty,
            temporary
                .image
                .as_deref()
                .ok_or(Error::Missing("constant temporary image"))?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        preprocess::unit::Preprocessor,
        sema::lookup::Filter,
        support::{source::SourceBuffer, stack::StackBudget},
        syntax::SyntaxUnit,
    };

    #[test]
    fn constant_addresses_keep_root_identity_views_offsets_and_extended_storage() {
        use crate::sema::{
            constant_addresses::Root,
            constants::{Constant, ConstantState},
        };
        use crate::{sema::types::Scalar, types::FundamentalType};
        let source = SourceBuffer::new(b"int a[3];constexpr int*p=a+3;int*const&r=p;constexpr long distance=p-a;constexpr const char*t=\"abc\";constexpr bool equal=t+1==\"abc\"+1;const int&temporary=7;constexpr int read=temporary;int f(int);constexpr bool callable=f;".to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"address-ownership").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_semantics(true);
        let graph = analyzer.analyze(declarations).unwrap();
        let mut entity = |spelling: &[u8]| {
            let name = cursor.intern_name(spelling).unwrap();
            graph
                .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
                .entity
        };
        let array = entity(b"a");
        let pointer = entity(b"p");
        let reference = entity(b"r");
        let value = graph.constant_addresses.value(pointer).unwrap();
        assert_eq!(value.root, Root::Variable(array));
        assert_eq!(value.offset, 12);
        assert!(value.past_end);
        assert!(matches!(
            graph.types.get(value.designated).kind,
            TypeKind::Scalar(Scalar::Basic(FundamentalType::Int))
        ));
        let view = graph.constant_addresses.value(reference).unwrap();
        assert_eq!(view.root, Root::Variable(pointer));
        assert_eq!(view.offset, 0);
        assert!(!view.past_end);
        assert!(matches!(
            graph.types.get(view.designated).kind,
            TypeKind::Pointer(_)
        ));
        assert_ne!(view.designated, value.designated);
        for (spelling, expected) in [
            (b"distance".as_slice(), 3),
            (b"equal", 1),
            (b"read", 7),
            (b"callable", 1),
        ] {
            let ConstantState::Value(constant) = graph.entity(entity(spelling)).constant else {
                panic!("published integer expected");
            };
            assert_eq!(constant.bits, expected);
        }
        let temporary = entity(b"temporary");
        let Root::Published(id) = graph.constant_addresses.value(temporary).unwrap().root else {
            panic!("published temporary expected");
        };
        assert!(graph.aggregate_constants.temporary(id).value.values().any(
            |(_, _, _, value)| matches!(
                value,
                Some(crate::sema::constant_addresses::TemporaryValue::Integer(
                    Constant { bits: 7, .. }
                ))
            )
        ));
        let text = graph.constant_addresses.value(entity(b"t")).unwrap();
        let Root::Literal(literal) = text.root else {
            panic!("retained literal root expected");
        };
        assert!(matches!(
            tree.expressions.get(literal).kind,
            crate::syntax::ast::ExprKind::Literal(_)
        ));
        // Pointer/reference values have their own typed store; integral facts never
        // stand in for an address, and string objects need no per-character graph.
        assert!(matches!(
            graph.entity(pointer).constant,
            ConstantState::None
        ));
        assert!(matches!(
            graph.entity(reference).constant,
            ConstantState::None
        ));
    }

    #[test]
    fn repeated_reference_cast_occurrences_create_distinct_evaluation_objects() {
        let source =
            SourceBuffer::new(b"constexpr int value=static_cast<const int&>(3);".to_vec()).unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"reference-occurrences").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let ast::DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[0])
            .kind
        else {
            panic!("scalar declaration");
        };
        let initializer = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0_u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let expressions = analyzer.tree.expressions.mark();
        analyzer
            .in_constant_evaluation(|analyzer| {
                let mut first = analyzer.constant_place(expr, scope)?;
                let second = analyzer.constant_place(expr, scope)?;
                assert!(matches!(first.root, Root::Object(_)));
                assert!(matches!(second.root, Root::Object(_)));
                assert_ne!(first.root, second.root);
                first.designated = analyzer.bare_type(first.designated)?;
                analyzer.write_constant_value(
                    first,
                    Value::Integer(super::super::constants::Constant {
                        state: super::super::constant_addresses::ScalarState::Value,
                        ty: first.designated,
                        bits: 9,
                        high: 0,
                    }),
                    initializer.location,
                )?;
                assert_eq!(
                    analyzer
                        .read_constant_integer(first, initializer.location)?
                        .bits,
                    9
                );
                assert_eq!(
                    analyzer
                        .read_constant_integer(second, initializer.location)?
                        .bits,
                    3
                );
                Ok(())
            })
            .unwrap();
        assert_eq!(analyzer.tree.expressions.mark(), expressions);
        assert!(!analyzer.constexpr_frames.active());
        assert!(analyzer.constant_evaluation.is_none());
    }

    #[test]
    fn extended_storage_keeps_identity_and_offsets_after_query_release() {
        let source = SourceBuffer::new(
            br#"
            struct A { int first, second; constexpr A(int n):first(n),second(n+1){} };
            constexpr const A& a=A(3);
            constexpr const A& same=a;
            constexpr const A& distinct=A(3);
            constexpr const int* member=&a.second;
            struct R {const int& r;};
            constexpr R array[2]={{7},{9}};
            constexpr R copy=array[1];
            static_assert(&same.second==member && &distinct.second!=member, "");
            static_assert(copy.r==9 && &copy.r==&array[1].r, "");
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"extended-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a".as_slice(), b"same", b"distinct", b"member"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        analyzer.finish_static_initializers().unwrap();
        let values: Vec<_> = names
            .into_iter()
            .map(|name| {
                let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
                analyzer
                    .graph
                    .constant_addresses
                    .value(analyzer.graph.binding(binding).entity)
                    .unwrap()
            })
            .collect();
        assert!(matches!(values[0].root, Root::Published(_)));
        assert_eq!(values[0].root, values[1].root);
        assert_ne!(values[0].root, values[2].root);
        assert_eq!(values[0].root, values[3].root);
        assert_eq!(values[3].offset, 4);
        assert_eq!(analyzer.graph.aggregate_constants.temporaries.len(), 4);
        assert!(analyzer.graph.aggregate_constants.constructing.is_empty());
        assert!(analyzer.graph.aggregate_constants.static_roots.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(!analyzer.constexpr_frames.active());
    }

    #[test]
    fn failed_static_probe_releases_partial_arguments_without_publishing_them() {
        let source = SourceBuffer::new(
            br"
            struct S {int n; constexpr S(int n):n(n){}};
            int runtime();
            constexpr S make(S s, int n) {return S(s.n+n);}
            constexpr S before(3);
            S dynamic=make(before,runtime());
            constexpr S after(7);
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"probe-rollback").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"before".as_slice(), b"dynamic", b"after"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let scope = analyzer.graph.global;
        analyzer.declarations(declarations, scope).unwrap();
        let syntax = analyzer.tree.expressions.mark();
        analyzer.finish_static_initializers().unwrap();
        for (name, bits) in names.into_iter().zip([Some(3), None, Some(7)]) {
            let binding = analyzer.graph.own(scope, name, Filter::Ordinary).unwrap();
            let entity = analyzer.graph.binding(binding).entity;
            let value = analyzer.graph.aggregate_constants.get(entity);
            if let Some(bits) = bits {
                assert!(value.unwrap().values().any(|(_, _, _, value)|
                    matches!(value, Some(Value::Integer(value)) if value.bits == bits)));
            } else {
                assert!(value.is_none());
                assert!(analyzer.graph.lower_array_image(entity).is_none());
            }
        }
        assert_eq!(analyzer.tree.expressions.mark(), syntax);
        assert_eq!(analyzer.graph.aggregate_constants.complete.len(), 2);
        assert!(analyzer.graph.aggregate_constants.constructing.is_empty());
        assert!(analyzer.constant_evaluation.is_none());
        assert!(!analyzer.constexpr_frames.active());
    }

    #[test]
    fn completed_objects_survive_calls_and_copies_without_replaying_syntax() {
        let source = SourceBuffer::new(
            br"
            struct S {int n; constexpr S(int v):n(v){} };
            constexpr S increment(S s) {s.n+=1; return s;}
            constexpr S a(3);
            constexpr S b=a;
            constexpr S c=increment(a);
            constexpr S elements[2]={S(7),increment(S(8))};
            constexpr int n=increment(a).n+elements[1].n;
        "
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"published-values").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let names: Vec<_> = [b"a", b"b", b"c"]
            .into_iter()
            .map(|name| cursor.intern_name(name).unwrap())
            .collect();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let global = analyzer.graph.global;
        analyzer.declarations(declarations, global).unwrap();
        for (name, bits) in names.into_iter().zip([3, 3, 4]) {
            let binding = analyzer.graph.own(global, name, Filter::Ordinary).unwrap();
            let value = analyzer
                .graph
                .aggregate_constants
                .get(analyzer.graph.binding(binding).entity)
                .unwrap();
            let values: Vec<_> = value
                .values()
                .filter_map(|(_, _, _, value)| match value {
                    Some(Value::Integer(value)) => Some(value.bits),
                    _ => None,
                })
                .collect();
            assert_eq!(values, [bits]);
        }
        let declaration = *analyzer.tree.decl_lists.get(declarations).last().unwrap();
        let ast::DeclKind::Simple { declarators, .. } =
            analyzer.tree.declarations.get(declaration).kind
        else {
            panic!("simple declaration");
        };
        let initializer = analyzer.tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let expr = analyzer.single_initializer(initializer).unwrap();
        let mark = analyzer.tree.expressions.mark();
        for _ in 0..2 {
            assert_eq!(
                analyzer
                    .constant_value(expr, global, true)
                    .unwrap()
                    .unwrap()
                    .bits,
                13
            );
            assert_eq!(analyzer.tree.expressions.mark(), mark);
            assert_eq!(analyzer.graph.aggregate_constants.complete.len(), 4);
            assert!(analyzer.graph.aggregate_constants.constructing.is_empty());
            assert!(analyzer.constant_evaluation.is_none());
            assert!(!analyzer.constexpr_frames.active());
        }
    }
    #[test]
    fn published_packed_values_keep_static_images_without_synthetic_syntax() {
        let source = SourceBuffer::new(
            br#"
            #pragma pack(push,1)
            struct Bits { unsigned a:5; unsigned b:30;
                constexpr Bits(unsigned a,unsigned b):a(a),b(b){} };
            #pragma pack(pop)
            constexpr const Bits& r=Bits(3,7);
            static_assert(r.a==3 && r.b==7, "published semantic value");
            int main(){return r.a+r.b;}
        "#
            .to_vec(),
        )
        .unwrap();
        let mut unit =
            SyntaxUnit::new(Preprocessor::with_name(&source, b"unrepresented-storage").unwrap());
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let name = cursor.intern_name(b"r").unwrap();
        let mark = tree.expressions.mark();
        let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
        analyzer.set_lowering();
        let marker = 0u8;
        analyzer.stack_base = std::ptr::from_ref(&marker).addr();
        let graph = analyzer.analyze(declarations).unwrap();
        let reference = graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity;
        let address = graph.constant_addresses.value(reference).unwrap();
        let Root::Published(id) = address.root else {
            panic!("published backing");
        };
        assert!(graph.aggregate_constants.temporary(id).image.is_some());
        assert!(graph.static_temporary_value(id, 0).is_some());
        assert!(
            graph
                .lower_variable(reference)
                .unwrap()
                .initializer
                .is_some()
        );
        assert_eq!(tree.expressions.mark(), mark);
    }
}
