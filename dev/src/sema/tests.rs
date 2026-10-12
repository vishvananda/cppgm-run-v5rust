//! Ownership and canonical identity regressions, independent of tool dump bytes.
use super::{
    Analyzer, Error,
    graph::{BindingKind, Graph},
    lookup::Filter,
    types::{Cv, RefKind, Scalar, TypeKind, Types},
};
use crate::{
    preprocess::unit::Preprocessor,
    support::{source::SourceBuffer, stack::StackBudget},
    syntax::{SyntaxUnit, cursor::Cursor},
    types::FundamentalType,
};

fn analyze(source: &[u8], check: impl FnOnce(&Graph, &mut Cursor<'_>)) {
    let source = SourceBuffer::new(source.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"semantic-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let graph = Analyzer::new(tree, cursor, StackBudget::default())
        .unwrap()
        .analyze(declarations)
        .unwrap();
    check(&graph, cursor);
}
#[test]
fn anonymous_union_injection_shares_members_and_owns_one_storage_slot() {
    analyze(
        b"struct C { union { int a; long b; }; char c; }; int n[sizeof(C)];",
        |graph, cursor| {
            let name = cursor.intern_name(b"C").unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            let scope = graph.entity(class).scope.unwrap();
            let members = &graph.entity(class).members;
            assert_eq!(members.len(), 2);
            let TypeKind::Class(union) = graph.types.get(graph.entity(members[0]).ty.unwrap()).kind
            else {
                panic!("unnamed storage must have the union's type");
            };
            let union_scope = graph.entity(union).scope.unwrap();
            for spelling in [b"a".as_slice(), b"b".as_slice()] {
                let name = cursor.intern_name(spelling).unwrap();
                let outer = graph.binding(graph.own(scope, name, Filter::Ordinary).unwrap());
                let inner = graph.binding(graph.own(union_scope, name, Filter::Ordinary).unwrap());
                assert_eq!(outer.entity, inner.entity);
                assert_eq!(outer.ty, inner.ty);
            }
            assert_eq!(graph.entity(members[1]).member_offset, Some(8));
        },
    );
}
#[test]
fn anonymous_names_remain_presentation_without_tag_or_constructor_publication() {
    analyze(
        b"using S=struct {}; S x; typedef enum {a} E; void f(){S();}",
        |graph, cursor| {
            let name = cursor.intern_name(b"S").unwrap();
            let binding = graph.binding(graph.own(graph.global, name, Filter::Prefix).unwrap());
            let TypeKind::Class(class) = graph.types.get(binding.ty.unwrap()).kind else {
                panic!("alias must retain its original class identity");
            };
            assert!(graph.entity(class).name.is_none());
            assert!(graph.entity(class).anonymous_class.is_some());
            assert!(graph.entity(class).implicit_default.is_none());
            for spelling in [b"S".as_slice(), b"E".as_slice(), b"x".as_slice()] {
                let name = cursor.intern_name(spelling).unwrap();
                assert!(graph.scope(graph.global).index[&name].tag.is_none());
            }
        },
    );
}
#[test]
fn canonical_types_preserve_source_parameters_and_adjust_nested_functions() {
    let mut types = Types::default();
    let int = types.scalar(Scalar::Basic(FundamentalType::Int)).unwrap();
    let const_int = types.qualify(int, Cv::CONST).unwrap();
    let callback = types
        .intern(
            TypeKind::Function {
                result: int,
                parameters: vec![const_int],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )
        .unwrap();
    let callback_bare = types
        .intern(
            TypeKind::Function {
                result: int,
                parameters: vec![int],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )
        .unwrap();
    let callback_pointer = types.pointer(callback_bare).unwrap();
    let array = types
        .intern(
            TypeKind::Array {
                element: int,
                bound: 3,
            },
            Cv::default(),
        )
        .unwrap();
    let int_pointer = types.pointer(int).unwrap();
    let a = types
        .intern(
            TypeKind::Function {
                result: int,
                parameters: vec![callback, array],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )
        .unwrap();
    let b = types
        .intern(
            TypeKind::Function {
                result: int,
                parameters: vec![callback_pointer, int_pointer],
                variadic: false,
                cv: Cv::default(),
                reference: None,
            },
            Cv::default(),
        )
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        types.canonical_function(a).unwrap(),
        types.canonical_function(b).unwrap()
    );
    assert!(
        matches!(&types.get(a).kind, TypeKind::Function { parameters, .. } if parameters == &[callback, array])
    );
    let left = types.reference(int, RefKind::Lvalue).unwrap();
    assert_eq!(types.reference(left, RefKind::Rvalue).unwrap(), left);
    assert_eq!(types.qualify(left, Cv::CONST).unwrap(), left);
    let qualified_array = types.qualify(array, Cv::CONST).unwrap();
    assert!(
        matches!(types.get(qualified_array).kind, TypeKind::Array { element, bound: 3 } if element == const_int)
    );
    let mut arrays = int;
    for _ in 0..10_000 {
        arrays = types
            .intern(
                TypeKind::Array {
                    element: arrays,
                    bound: 1,
                },
                Cv::default(),
            )
            .unwrap();
    }
    let mut qualified = types.qualify(arrays, Cv::CONST).unwrap();
    for _ in 0..10_000 {
        let TypeKind::Array { element, bound: 1 } = types.get(qualified).kind else {
            panic!("array chain");
        };
        qualified = element;
    }
    assert_eq!(qualified, const_int);
    let mut chain = a;
    for _ in 0..10_000 {
        chain = types.pointer(chain).unwrap();
    }
    assert_eq!(
        types.canonical_function(chain).unwrap(),
        types.canonical_function(chain).unwrap()
    );
}
#[test]
fn source_bindings_share_completed_array_entity_and_retain_resolved_facts() {
    analyze(
        b"extern int a[]; int a[12]; extern int a[]; using A = int[12]; A* p;",
        |graph, cursor| {
            let a = cursor.intern_name(b"a").unwrap();
            let scope = graph.scope(graph.global);
            let declarations: Vec<_> = scope
                .bindings
                .iter()
                .map(|&id| graph.binding(id))
                .filter(|b| b.name == Some(a))
                .collect();
            assert_eq!(declarations.len(), 3);
            assert!(
                declarations
                    .iter()
                    .all(|b| b.entity == declarations[0].entity)
            );
            assert!(matches!(
                graph
                    .types
                    .get(graph.entity(declarations[0].entity).ty.unwrap())
                    .kind,
                TypeKind::Array { bound: 12, .. }
            ));
            assert_eq!(graph.declared_bindings.iter().flatten().count(), 4);
            assert!(graph.resolved_names.iter().flatten().next().is_some());
            assert_eq!(graph.type_facts.iter().flatten().count(), 1);
        },
    );
}
#[test]
fn directive_cycles_and_common_namespace_lookup_preserve_inner_shadowing() {
    analyze(b"namespace A { typedef int T; } namespace B { typedef char T; using namespace A; T x; } namespace A { using namespace B; typedef long U; } using namespace B; U y;", |graph, cursor| {
        let b = cursor.intern_name(b"B").unwrap(); let x = cursor.intern_name(b"x").unwrap();
        let namespace = graph.binding(graph.own(graph.global, b, Filter::Namespace).unwrap());
        let b = graph.entity(namespace.entity).scope.unwrap();
        let x = graph.binding(graph.own(b, x, Filter::Ordinary).unwrap());
        assert!(matches!(graph.types.get(x.ty.unwrap()).kind, TypeKind::Scalar(Scalar::Basic(FundamentalType::Char))));
        let y = cursor.intern_name(b"y").unwrap(); let y = graph.binding(graph.own(graph.global, y, Filter::Ordinary).unwrap());
        assert_eq!(y.kind, BindingKind::Variable);
        assert!(matches!(graph.types.get(y.ty.unwrap()).kind, TypeKind::Scalar(Scalar::Basic(FundamentalType::LongInt))));
    });
}
#[test]
fn member_bodies_wait_for_class_completion_and_elaborated_names_use_namespace_home() {
    analyze(b"struct C { void f(){ Later x; } using Later = int; struct Hidden* p; }; struct Hidden { int x; };", |graph, cursor| {
        let c = cursor.intern_name(b"C").unwrap(); let hidden = cursor.intern_name(b"Hidden").unwrap();
        let class = graph.binding(graph.own(graph.global, c, Filter::Elaborated).unwrap());
        let class = graph.entity(class.entity).scope.unwrap();
        assert!(graph.own(class, hidden, Filter::Elaborated).is_none());
        assert!(graph.own(graph.global, hidden, Filter::Elaborated).is_some());
        assert_eq!(graph.scope(class).children.len(), 1);
        let function = graph.scope(class).children[0]; let block = graph.scope(function).children[0];
        let x = cursor.intern_name(b"x").unwrap();
        let x = graph.binding(graph.own(block, x, Filter::Ordinary).unwrap());
        assert!(matches!(graph.types.get(x.ty.unwrap()).kind, TypeKind::Scalar(Scalar::Basic(FundamentalType::Int))));
    });
}
#[test]
fn missing_group_remains_distinct_from_a_language_rejection() {
    let source = SourceBuffer::new(b"template<class T> struct C {};".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"missing-semantic-group").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let result = Analyzer::new(tree, cursor, StackBudget::default())
        .unwrap()
        .analyze(declarations);
    assert!(matches!(result, Err(Error::Missing(_))));
}

#[test]
fn implicit_constructor_demand_is_deduplicated_and_respects_unevaluated_and_initializer_contexts() {
    for (source, constructors) in [
        (
            b"struct C {}; void f(){ sizeof(C()); C c = C(); }".as_slice(),
            0,
        ),
        (b"struct C {}; void f(){ C(); C(); }".as_slice(), 1),
        (b"struct C {}; void f(){ new C; }".as_slice(), 1),
    ] {
        analyze(source, |graph, cursor| {
            let c = cursor.intern_name(b"C").unwrap();
            let class = graph.binding(graph.own(graph.global, c, Filter::Elaborated).unwrap());
            let scope = graph.entity(class.entity).scope.unwrap();
            assert_eq!(graph.scope(scope).children.len(), constructors);
        });
    }
}

#[test]
fn initialization_completes_arrays_and_definition_checks_reject_incomplete_objects() {
    analyze(
        b"int a[] = {1, 2, 3}; char s[] = \"ab\";",
        |graph, cursor| {
            for spelling in [b"a".as_slice(), b"s".as_slice()] {
                let name = cursor.intern_name(spelling).unwrap();
                let binding =
                    graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
                assert!(matches!(
                    graph
                        .types
                        .get(graph.entity(binding.entity).ty.unwrap())
                        .kind,
                    TypeKind::Array { bound: 3, .. }
                ));
            }
        },
    );
    for (input, diagnostic) in [
        (b"struct C; C x;".as_slice(), "variable of incomplete type"),
        (b"int a[];".as_slice(), "variable of incomplete type"),
        (
            b"void f(){} void f(){}".as_slice(),
            "redefinition of a function",
        ),
        (
            b"namespace N{} int N::x;".as_slice(),
            "qualified definition does not match a declaration",
        ),
    ] {
        let source = SourceBuffer::new(input.to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"declaration-invariants").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let result = Analyzer::new(tree, cursor, StackBudget::default())
            .unwrap()
            .analyze(declarations);
        assert!(matches!(result, Err(Error::Diagnostic { message, .. }) if message == diagnostic));
    }
}

#[test]
fn enum_aliases_and_using_bindings_share_finished_constant_identity() {
    use super::constants::ConstantState;
    analyze(b"namespace N { enum class E : unsigned char { A=255 }; } using N::E; using N::E::A; const E value = A;", |graph, cursor| {
        let name = cursor.intern_name(b"E").unwrap();
        let e = graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
        let enum_scope = graph.entity(e.entity).scope.unwrap();
        let name = cursor.intern_name(b"A").unwrap();
        let original = graph.binding(graph.own(enum_scope, name, Filter::Ordinary).unwrap());
        let imported = graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
        assert_eq!(original.entity, imported.entity);
        let ConstantState::Value(value) = graph.entity(imported.entity).constant else { panic!("missing enumerator value"); };
        assert_eq!(value.bits, 255);
        assert_eq!(Some(value.ty), e.ty);
        let name = cursor.intern_name(b"value").unwrap();
        let object = graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
        let ConstantState::Value(object) = graph.entity(object.entity).constant else { panic!("missing object value"); };
        assert_eq!(object.ty, value.ty);
        assert_eq!(object.bits, value.bits);
        assert!(graph.constant_facts.iter().flatten().any(|fact| fact.ty == value.ty));
    });
}

#[test]
fn prototype_queries_keep_source_arrays_and_parameter_object_cv() {
    analyze(b"int f(int values[4], int (&bytes)[sizeof(values)], const int value, decltype(value) copy);", |graph, cursor| {
        let name = cursor.intern_name(b"f").unwrap();
        let function = graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
        let TypeKind::Function { parameters, .. } = &graph.types.get(function.ty.unwrap()).kind else { panic!("missing function type"); };
        assert!(matches!(graph.types.get(parameters[0]).kind, TypeKind::Array { bound: 4, .. }));
        let TypeKind::Reference { element, .. } = graph.types.get(parameters[1]).kind else { panic!("missing reference parameter"); };
        assert!(matches!(graph.types.get(element).kind, TypeKind::Array { bound: 8, .. }));
        assert_eq!(graph.types.get(parameters[2]).cv, Cv::CONST);
        assert_eq!(parameters[2], parameters[3]);
    });
}

#[test]
fn target_integral_operations_distinguish_wrap_from_undefined_results() {
    use super::integral::IntegralKind;
    use crate::lex::pp_lexer::Punctuator as P;
    for bits in [8, 16, 32, 64] {
        let unsigned = IntegralKind {
            bits,
            signed: false,
        };
        let signed = IntegralKind { bits, signed: true };
        let mask = u64::MAX >> (64 - bits);
        assert_eq!(unsigned.operation(P::Add, mask, 1), Some((0, true)));
        assert_eq!(
            signed.operation(P::Add, mask >> 1, 1),
            Some((signed.minimum().cast_unsigned(), false))
        );
        assert_eq!(
            signed.operation(P::Divide, signed.minimum().cast_unsigned(), u64::MAX),
            Some((0, false))
        );
        assert_eq!(
            signed.shift(P::ShiftLeft, 1, u64::from(bits - 1), signed),
            (signed.minimum().cast_unsigned(), true)
        );
        assert_eq!(
            signed.shift(P::ShiftLeft, 1, u64::from(bits), signed),
            (0, false)
        );
        assert!(!unsigned.represents(u64::MAX, signed));
    }
}

#[test]
fn decoded_string_elements_keep_character_width_and_signedness() {
    use super::constants::ConstantState;
    analyze(b"const int narrow = \"\\xff\"[0]; const int wide = u\"\\xff\"[0]; const int zero = \"ab\"[2];", |graph, cursor| {
        for (name, bits) in [(b"narrow".as_slice(), u64::MAX), (b"wide", 255), (b"zero", 0)] {
            let name = cursor.intern_name(name).unwrap();
            let object = graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap());
            let ConstantState::Value(value) = graph.entity(object.entity).constant else { panic!("missing character value"); };
            assert_eq!(value.bits, bits);
            assert!(matches!(graph.types.get(value.ty).kind, TypeKind::Scalar(Scalar::Basic(FundamentalType::Int))));
        }
    });
}

#[test]
fn exception_sets_are_entity_facts_outside_canonical_function_identity() {
    use super::exceptions::{ExceptionSet, ExceptionState};
    analyze(b"void a() throw(int, const int, double); void a() throw(double,int); void b() noexcept; void c();", |graph, cursor| {
        let binding = |cursor: &mut Cursor<'_>, name| {
            let name = cursor.intern_name(name).unwrap();
            graph.binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
        };
        let a = binding(cursor,b"a"); let b = binding(cursor,b"b"); let c = binding(cursor,b"c");
        assert_eq!(a.ty,b.ty); assert_eq!(b.ty,c.ty);
        assert!(matches!(&graph.entity(a.entity).exceptions, ExceptionState::Value(ExceptionSet::Dynamic(types)) if types.len()==2));
        assert!(matches!(graph.entity(b.entity).exceptions, ExceptionState::Value(ExceptionSet::None)));
        assert!(matches!(graph.entity(c.entity).exceptions, ExceptionState::Value(ExceptionSet::All)));
    });
}

#[test]
fn deferred_exception_lookup_limits_namespaces_and_completes_class_members() {
    use super::exceptions::{ExceptionSet, ExceptionState};
    analyze(b"const int n=1; namespace N{void f() noexcept(n); const int n=0;} struct C{void f() noexcept(n); static const int n=1;};", |graph,cursor| {
        for owner in [b"N".as_slice(), b"C"] {
            let name = cursor.intern_name(owner).unwrap();
            let owner = graph.binding(graph.own(graph.global,name,Filter::Prefix).unwrap());
            let scope = graph.entity(owner.entity).scope.unwrap();
            let name = cursor.intern_name(b"f").unwrap();
            let function = graph.binding(graph.own(scope,name,Filter::Ordinary).unwrap());
            assert!(matches!(graph.entity(function.entity).exceptions,ExceptionState::Value(ExceptionSet::None)));
            assert!(graph.scope(scope).children.is_empty());
        }
    });
}

#[test]
fn layout_is_demanded_once_and_records_only_ordinary_data_member_offsets() {
    use super::layout::LayoutState;
    analyze(b"struct A{char c; int n;}; struct B{A a[2]; char c; static int ignored;}; struct U{int n;}; int size[sizeof(B)];", |graph,cursor| {
        let name=cursor.intern_name(b"B").unwrap();
        let binding=graph.binding(graph.own(graph.global,name,Filter::Elaborated).unwrap());
        let entity=graph.entity(binding.entity);
        assert!(matches!(entity.layout, LayoutState::Value{size:20,alignment:4}));
        assert_eq!(entity.members.len(),2);
        assert_eq!(graph.entity(entity.members[0]).member_offset,Some(0));
        assert_eq!(graph.entity(entity.members[1]).member_offset,Some(16));
        let name=cursor.intern_name(b"U").unwrap();
        let binding=graph.binding(graph.own(graph.global,name,Filter::Elaborated).unwrap());
        assert!(matches!(graph.entity(binding.entity).layout,LayoutState::NotStarted));
    });
}

#[test]
fn missing_layout_and_constant_call_owners_remain_status_86_boundaries() {
    for source in [
        b"struct C{virtual void f();}; int a[sizeof(C)];".as_slice(),
        b"constexpr int f(){return 1;} void g() noexcept(f());",
    ] {
        let source = SourceBuffer::new(source.to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"missing-query-owner").unwrap();
        let mut unit = SyntaxUnit::new(input);
        unit.parse().unwrap();
        let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
        let result = Analyzer::new(tree, cursor, StackBudget::default())
            .unwrap()
            .analyze(declarations);
        assert!(matches!(result, Err(Error::Missing(_))));
    }
}

#[test]
fn exception_parameters_retain_source_type_and_adjusted_object_identity() {
    use super::exceptions::{ExceptionSet, ExceptionState};
    analyze(b"void f(int a[3]) noexcept(sizeof(decltype(a))==12 && sizeof(a)==8); void f(int a[3]) noexcept;", |graph,cursor| {
        let name = cursor.intern_name(b"a").unwrap();
        let mut found = false;
        for &binding in graph.resolved_names.iter().flatten() {
            let binding = graph.binding(binding);
            if binding.name == Some(name) && binding.kind == BindingKind::Parameter {
                found = true;
                assert!(matches!(graph.types.get(binding.ty.unwrap()).kind, TypeKind::Array{bound:3,..}));
                assert!(matches!(graph.types.get(graph.entity(binding.entity).ty.unwrap()).kind, TypeKind::Pointer(_)));
            }
        }
        assert!(found);
        let name = cursor.intern_name(b"f").unwrap();
        let binding = graph.binding(graph.own(graph.global,name,Filter::Ordinary).unwrap());
        assert!(matches!(graph.entity(binding.entity).exceptions, ExceptionState::Value(ExceptionSet::None)));
    });
}

#[test]
fn failed_exception_query_restores_nested_worklists_and_context() {
    let source =
        SourceBuffer::new(b"void f() noexcept(1 + sizeof(1 + missing));".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"exception-query-rollback").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    let marker = 0u8;
    analyzer.stack_base = std::ptr::from_ref(&marker).addr();
    analyzer
        .declarations(declarations, analyzer.graph.global)
        .unwrap();
    assert!(matches!(
        analyzer.finish_unit_exceptions(),
        Err(Error::Diagnostic {
            message: "undeclared name",
            ..
        })
    ));
    assert!(analyzer.exception_work.is_empty());
    assert!(analyzer.constant_spine.is_empty());
    assert!(!analyzer.evaluating_noexcept);
    assert!(analyzer.graph.lookup_limit.is_none());
}
