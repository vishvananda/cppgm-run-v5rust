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
    let source = SourceBuffer::new(b"enum E { x };".to_vec()).unwrap();
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
