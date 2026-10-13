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

#[test]
fn base_paths_reuse_identity_preserve_operand_types_and_keep_trivial_base_demand_local() {
    use super::{
        construction::ConstructorState, expression_facts::ValueConversion, function_demands::Demand,
    };
    use crate::syntax::ast::{Body, DeclKind, StmtKind};
    let source = SourceBuffer::new(b"struct B{int x;};struct M:B{};struct D:M{};B*a(D*d){return d;}B*b(D*d){return d;}int member(const D&d){return d.B::x;}void init(){D d;D e;}".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"base-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let mut paths = Vec::new();
    for index in [3, 4] {
        let DeclKind::Function { body, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[index])
            .kind
        else {
            panic!("function expected");
        };
        let Body::Parsed(body) = tree.bodies.get(body) else {
            panic!("body expected");
        };
        let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
            panic!("compound expected");
        };
        let StmtKind::Return(Some(expression)) =
            tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
        else {
            panic!("return expected");
        };
        let facts = graph.expression_facts.get(expression).unwrap();
        let conversion = facts.conversion.unwrap();
        assert_eq!(conversion.second, ValueConversion::DerivedToBase);
        assert_ne!(facts.ty, conversion.result);
        assert_eq!(graph.expression_type(expression), Some(facts.ty));
        paths.push(facts.base_path.unwrap());
    }
    assert_eq!(paths[0], paths[1]);
    assert_eq!(graph.base_paths.get(paths[0]).len(), 2);
    let mut classes = Vec::new();
    for spelling in [b"B".as_slice(), b"M", b"D"] {
        let name = cursor.intern_name(spelling).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        assert!(matches!(
            graph.entity(class).layout,
            super::layout::LayoutState::NotStarted
        ));
        let ConstructorState::Ready(plan) = graph.entity(class).default_construction else {
            panic!("demanded base plan expected");
        };
        classes.push((class, plan));
    }
    assert_eq!(
        graph.constructors.get(classes[2].1).bases,
        vec![classes[1].1]
    );
    assert_eq!(
        graph.constructors.get(classes[1].1).bases,
        vec![classes[0].1]
    );
    assert_eq!(graph.entity(classes[2].0).bases[0].class, classes[1].0);
    let trailing = super::function_demands::trailing(&graph);
    assert!(matches!(trailing.as_slice(), [Demand::Constructor(id)] if *id == classes[2].1));
}

#[test]
fn repeated_nonvirtual_subobjects_saturate_without_enumerating_paths_or_demanding_layout() {
    let mut source = String::from("struct B{};struct L0:B{};struct R0:B{};");
    for index in 1..64 {
        use std::fmt::Write;
        let previous = index - 1;
        write!(source, "struct L{index}:L{previous},R{previous}{{}};struct R{index}:L{previous},R{previous}{{}};").unwrap();
    }
    analyze(source.as_bytes(), |graph, cursor| {
        let mut classes = Vec::new();
        for spelling in [b"B".as_slice(), b"L0", b"L63"] {
            let name = cursor.intern_name(spelling).unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            assert!(matches!(
                graph.entity(class).layout,
                super::layout::LayoutState::NotStarted
            ));
            assert!(matches!(
                graph.entity(class).default_construction,
                super::construction::ConstructorState::NotStarted
            ));
            classes.push(class);
        }
        assert_eq!(graph.base_count(classes[1], classes[0]), 1);
        assert_eq!(graph.base_count(classes[2], classes[0]), 2);
        assert_eq!(graph.base_count(classes[0], classes[2]), 0);
    });
}

#[test]
fn elided_copies_retain_selection_and_original_facts_without_body_demand() {
    use super::{copy_initialization::CopyOrigin, expression_facts::ValueCategory};
    use crate::syntax::ast::{DeclKind, InitializerKind};
    let source = SourceBuffer::new(b"struct Hidden{};struct C{int x;C(const C&){Hidden h;}};C make();C a=make();const C b((make()));struct D{int x;};D get();D d=get();D e=get();".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"copy-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let mut origins = Vec::new();
    for index in [3, 4, 7, 8] {
        let DeclKind::Simple { declarators, .. } = tree
            .declarations
            .get(tree.decl_lists.get(declarations)[index])
            .kind
        else {
            panic!("object declaration expected");
        };
        let declaration = tree.init_declarators.get(declarators)[0];
        let initializer = declaration.initializer.unwrap();
        let expression = match initializer.kind {
            InitializerKind::Assign(id) => id,
            InitializerKind::Parenthesized(arguments) => tree.expr_lists.get(arguments)[0],
            _ => panic!("copy initializer expected"),
        };
        let facts = graph.expression_facts.get(expression).unwrap();
        assert_eq!(facts.category, ValueCategory::Prvalue);
        assert!(facts.conversion.is_none());
        assert_eq!(
            graph.expression_type(graph.expression_facts.root(expression)),
            Some(facts.ty)
        );
        let selected = facts.elided_copy.unwrap();
        assert!(!selected.binding.temporary);
        let TypeKind::Reference { element, .. } = graph.types.get(selected.binding.target).kind
        else {
            panic!("selected constructor parameter is not a reference");
        };
        assert_eq!(
            graph.types.get(element).kind,
            graph.types.get(facts.ty).kind
        );
        origins.push(selected.origin);
        let binding =
            graph.binding(graph.declared_bindings[declaration.declarator.index()].unwrap());
        let variable = binding.entity;
        assert_eq!(graph.entity(variable).ty, binding.ty);
        assert_eq!(
            graph.types.get(binding.ty.unwrap()).cv,
            if index == 4 { Cv::CONST } else { Cv::default() }
        );
        assert!(graph.entity(variable).constructor.is_none());
    }
    assert_eq!(origins[0], origins[1]);
    assert_eq!(origins[2], origins[3]);
    assert!(matches!(origins[0], CopyOrigin::Declared(_)));
    let CopyOrigin::Implicit { class, kind } = origins[2] else {
        panic!("implicit move expected");
    };
    assert_eq!(kind, RefKind::Rvalue);
    assert!(graph.entity(class).implicit_copies.is_some());
    assert!(matches!(
        graph.entity(class).default_construction,
        super::construction::ConstructorState::NotStarted
    ));
    // The written copy body was analyzed, but its Hidden dependency remains
    // owned by that undemanded function. Elision must never make it a root.
    assert!(super::function_demands::trailing(&graph).is_empty());
}

fn analyze(source: &[u8], check: impl FnOnce(&mut Graph, &mut Cursor<'_>)) {
    let source = SourceBuffer::new(source.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"semantic-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut graph = Analyzer::new(tree, cursor, StackBudget::default())
        .unwrap()
        .analyze(declarations)
        .unwrap();
    check(&mut graph, cursor);
}
#[test]
fn member_pointer_identity_keeps_owner_and_function_qualifiers_after_parameter_adjustment() {
    analyze(
        b"struct C;struct D;using Owner=C;using A=int(Owner::*)(const int)const&;using B=int(C::*)(int)const&;using OtherOwner=int(D::*)(int)const&;using OtherCv=int(C::*)(int)&;using OtherRef=int(C::*)(int)const&&;",
        |graph, cursor| {
            let mut types = Vec::new();
            for spelling in [b"A".as_slice(), b"B", b"OtherOwner", b"OtherCv", b"OtherRef"] {
                let name = cursor.intern_name(spelling).unwrap();
                let binding = graph.own(graph.global, name, Filter::Prefix).unwrap();
                let ty = graph.binding(binding).ty.unwrap();
                types.push(graph.types.canonical_function(ty).unwrap());
            }
            assert_eq!(types[0], types[1]);
            for &other in &types[2..] {
                assert_ne!(types[0], other);
            }
            let TypeKind::MemberPointer { owner, element } = graph.types.get(types[0]).kind else {
                panic!("alias lost its member-pointer identity");
            };
            assert!(matches!(graph.types.get(owner).kind, TypeKind::Class(_)));
            let TypeKind::Function { cv, reference, .. } = graph.types.get(element).kind else {
                panic!("member pointer lost its function type");
            };
            assert_eq!(cv, Cv::CONST);
            assert_eq!(reference, Some(RefKind::Lvalue));
        },
    );
}
#[test]
fn member_pointer_size_queries_retain_constants_without_replacing_the_syntax_view() {
    use crate::syntax::ast::{DeclKind, ExprKind, InitializerKind};
    let source = SourceBuffer::new(
        b"struct C;using M=int(C::*)();unsigned long a=sizeof(M);unsigned long b=alignof(M);"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"query-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    for (&declaration, expected) in tree.decl_lists.get(declarations)[2..].iter().zip([16, 8]) {
        let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
            panic!("query declaration expected");
        };
        let initializer = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap();
        let InitializerKind::Assign(query) = initializer.kind else {
            panic!("query initializer expected");
        };
        assert!(matches!(
            tree.expressions.get(query).kind,
            ExprKind::Sizeof { .. } | ExprKind::TypeTrait { .. }
        ));
        let facts = graph.expression_facts.get(query).unwrap();
        assert_eq!(facts.query_value, Some(expected));
        assert!(facts.folded.is_none());
        assert_eq!(graph.expression_type(query), Some(facts.ty));
    }
    let name = cursor.intern_name(b"C").unwrap();
    let class = graph
        .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
        .entity;
    assert!(!graph.entity(class).complete);
    assert!(graph.entity(class).scope.is_none());
}
#[test]
fn control_and_handler_bindings_have_local_owners_and_do_not_leak() {
    analyze(
        b"void f(int n){if(int c=n){int a=c;}else int b=c;for(int i=0;i<n;++i){int j=i;}try{}catch(const int& e){int k=e;}}",
        |graph, cursor| {
            let f = cursor.intern_name(b"f").unwrap();
            let binding = graph.own(graph.global, f, Filter::Ordinary).unwrap();
            let function = graph.entity(graph.binding(binding).entity).scope.unwrap();
            let body = graph.scope(function).children[0];
            let children = &graph.scope(body).children;
            assert_eq!(children.len(), 4);
            let condition = children[0];
            let handler = children[3];
            for (owner, spelling) in [(condition, b"c".as_slice()), (handler, b"e".as_slice())] {
                let name = cursor.intern_name(spelling).unwrap();
                let binding = graph.own(owner, name, Filter::Ordinary).unwrap();
                assert_eq!(graph.binding(binding).kind, BindingKind::Variable);
                assert!(graph.own(body, name, Filter::Ordinary).is_none());
                for &child in &graph.scope(owner).children {
                    assert_eq!(graph.scope(child).parent, Some(owner));
                    assert!(graph.scope(child).guarded);
                    assert!(graph.own(child, name, Filter::Ordinary).is_none());
                }
            }
        },
    );
}
#[test]
fn special_members_share_signatures_without_colliding_with_kind_or_template_shape() {
    use super::graph::MemberKind;
    analyze(
        b"struct C{C(int);template<class T>C(int p){}template<class T,class U>C(int);~C(){}operator int();typedef int I;};C::operator I(){}",
        |graph, cursor| {
            let name = cursor.intern_name(b"C").unwrap();
            let class = graph.binding(graph.own(graph.global, name, Filter::Prefix).unwrap()).entity;
            let scope = graph.entity(class).scope.unwrap();
            assert_eq!(graph.binding(graph.own(scope, name, Filter::Ordinary).unwrap()).entity, class);
            let mut pending = vec![scope];
            let mut constructors = Vec::new();
            let mut conversions = Vec::new();
            while let Some(scope) = pending.pop() {
                pending.extend(graph.scope(scope).children.iter().copied());
                for &id in &graph.scope(scope).bindings {
                    let binding = graph.binding(id);
                    let Some(key) = graph.entity(binding.entity).member_function else { continue };
                    match key.kind {
                        MemberKind::Constructor => constructors.push((binding.entity, key)),
                        MemberKind::Conversion => conversions.push((binding.entity, id)),
                        MemberKind::Destructor => assert!(graph.entity(binding.entity).special_name.is_some()),
                    }
                }
            }
            assert_eq!(constructors.len(), 3);
            assert!(constructors.iter().all(|(_, key)| key.signature == constructors[0].1.signature));
            assert!(constructors.iter().enumerate().all(|(i, a)| constructors.iter().skip(i+1).all(|b| a.0 != b.0 && a.1.shape != b.1.shape)));
            assert_eq!(conversions.len(), 2);
            assert_eq!(conversions[0].0, conversions[1].0);
            let first_label = graph.entity(conversions[0].0).special_name.unwrap();
            assert_ne!(graph.special_labels[&conversions[0].1], graph.special_labels[&conversions[1].1]);
            assert!(conversions.iter().any(|(_, id)| graph.special_labels[id] == first_label));
            let body = graph.entity(conversions[0].0).scope.unwrap();
            assert_eq!(graph.scope(body).owner, Some(conversions[0].0));
        },
    );
}
#[test]
fn template_exception_patterns_retain_the_original_declaration_environment() {
    use super::exceptions::ExceptionState;
    analyze(
        b"struct C{template<class T>C(T p)noexcept(sizeof(T)+unknown);};",
        |graph, cursor| {
            let name = cursor.intern_name(b"C").unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            let scope = graph.entity(class).scope.unwrap();
            let head = graph.scope(scope).children[0];
            let function = graph
                .scope(head)
                .bindings
                .iter()
                .copied()
                .find(|&id| graph.binding(id).kind == BindingKind::Function)
                .unwrap();
            let entity = graph.entity(graph.binding(function).entity);
            let ExceptionState::Template(pattern) = entity.exceptions else {
                panic!("missing retained template specification")
            };
            assert_eq!(pattern.scope, head);
            assert_eq!(pattern.ty, graph.binding(function).ty.unwrap());
            assert!(pattern.parameters.is_some());
            assert_eq!(graph.scope(head).children.len(), 0);
        },
    );
}
#[test]
fn constructor_signatures_do_not_hide_the_injected_class_and_share_redeclaration_facts() {
    analyze(
        b"struct C{C(const int);C(long);C *next;};C::C(int p){} void f(){C();new C;}",
        |graph, cursor| {
            let name = cursor.intern_name(b"C").unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            let scope = graph.entity(class).scope.unwrap();
            let injected = graph.binding(graph.own(scope, name, Filter::Ordinary).unwrap());
            assert_eq!(injected.kind, BindingKind::Type);
            assert_eq!(injected.entity, class);
            assert!(graph.entity(class).user_constructor.is_some());
            assert!(graph.entity(class).implicit_default.is_none());
            let declarations: Vec<_> = graph
                .scope(scope)
                .bindings
                .iter()
                .copied()
                .filter(|&id| graph.binding(id).kind == BindingKind::Function)
                .collect();
            assert_eq!(declarations.len(), 3);
            let first = graph.binding(declarations[0]);
            let second = graph.binding(declarations[1]);
            let definition = graph.binding(declarations[2]);
            assert_eq!(first.entity, definition.entity);
            assert_ne!(first.entity, second.entity);
            assert_ne!(first.ty, definition.ty);
            let signature = graph.types.canonical_function(first.ty.unwrap()).unwrap();
            assert_eq!(graph.constructor(class, signature), Some(declarations[2]));
            assert!(graph.entity(first.entity).defined);
            let body = graph.entity(first.entity).scope.unwrap();
            assert_eq!(graph.scope(body).owner, Some(first.entity));
            assert_eq!(graph.scope(body).parent, Some(scope));
            assert!(!graph.entity(second.entity).defined);
        },
    );
}
#[test]
fn qualified_static_definitions_preserve_member_identity_and_never_add_object_storage() {
    analyze(
        b"struct C{static int value;char field;};int C::value=2;int size[sizeof(C)];",
        |graph, cursor| {
            let name = cursor.intern_name(b"C").unwrap();
            let class = graph
                .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
                .entity;
            let scope = graph.entity(class).scope.unwrap();
            let value = cursor.intern_name(b"value").unwrap();
            let declarations: Vec<_> = graph
                .scope(scope)
                .bindings
                .iter()
                .copied()
                .filter(|&id| graph.binding(id).name == Some(value))
                .collect();
            assert_eq!(declarations.len(), 2);
            let declared = graph.binding(declarations[0]).entity;
            assert_eq!(declared, graph.binding(declarations[1]).entity);
            assert_eq!(
                graph.entity(declared).data_member,
                Some(super::graph::DataMemberKind::Static)
            );
            assert!(graph.entity(declared).defined);
            let members = &graph.entity(class).members;
            assert_eq!(members.len(), 1);
            assert_eq!(
                graph.entity(members[0]).data_member,
                Some(super::graph::DataMemberKind::Instance)
            );
            assert_eq!(graph.entity(members[0]).member_offset, Some(0));
            assert!(!members.contains(&declared));
        },
    );
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
fn semantic_storage_is_hidden_and_default_construction_is_demanded_once() {
    use crate::syntax::ast::DeclKind;
    let source = SourceBuffer::new(
        b"struct Unused {}; struct A {}; A array[2]; A first; A second; static union { int x; long y; }; int f(){return x;}"
            .to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"construction-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let unused = cursor.intern_name(b"Unused").unwrap();
    let unused = graph
        .binding(graph.own(graph.global, unused, Filter::Prefix).unwrap())
        .entity;
    assert!(matches!(
        graph.entity(unused).default_construction,
        super::construction::ConstructorState::NotStarted
    ));
    let mut constructors = Vec::new();
    for spelling in [b"array".as_slice(), b"first", b"second"] {
        let name = cursor.intern_name(spelling).unwrap();
        let entity = graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity;
        constructors.push(graph.entity(entity).constructor.unwrap());
    }
    assert_eq!(constructors[0], constructors[1]);
    assert_eq!(constructors[1], constructors[2]);
    assert_eq!(super::function_demands::trailing(&graph).len(), 2);
    let anonymous = tree.decl_lists.get(declarations)[5];
    assert!(matches!(
        tree.declarations.get(anonymous).kind,
        DeclKind::Simple { .. }
    ));
    let storage = graph.declaration_storage[anonymous.index()].unwrap();
    assert!(graph.binding(storage).name.is_none());
    assert!(!graph.scope(graph.global).bindings.contains(&storage));
    let name = cursor.intern_name(b"x").unwrap();
    let member = graph
        .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
        .entity;
    let class_scope = graph.entity(member).home.unwrap();
    let class = graph.scope(class_scope).owner.unwrap();
    assert_eq!(graph.entity(class).anonymous_storage, Some(storage));
    assert!(matches!(
        graph.entity(class).layout,
        super::layout::LayoutState::NotStarted
    ));
    let constructor = graph
        .entity(graph.binding(storage).entity)
        .constructor
        .unwrap();
    assert_eq!(graph.constructors.get(constructor).class, class);
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
    let source = SourceBuffer::new(b"template<int N> struct C {};".to_vec()).unwrap();
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

#[test]
fn template_scope_printing_and_namespace_publication_share_entity_identity() {
    use super::graph::ScopeKind;
    analyze(
        b"template<class T,template<class Inner>class TT>struct P{T x;};",
        |graph, cursor| {
            let p = cursor.intern_name(b"P").unwrap();
            let t = cursor.intern_name(b"T").unwrap();
            let inner = cursor.intern_name(b"Inner").unwrap();
            let parameters = graph.scope(graph.global).children[0];
            assert_eq!(graph.scope(parameters).kind, ScopeKind::TemplateParameters);
            assert!(graph.own(graph.global, t, Filter::Ordinary).is_none());
            assert!(graph.own(parameters, inner, Filter::Ordinary).is_none());
            let binding = graph.own(graph.global, p, Filter::Elaborated).unwrap();
            assert!(graph.scope(parameters).bindings.contains(&binding));
            assert!(graph.own(parameters, p, Filter::Ordinary).is_none());
            let entity = graph.binding(binding).entity;
            let class = graph.entity(entity).scope.unwrap();
            assert_eq!(graph.scope(class).parent, Some(parameters));
            assert_eq!(graph.scope(parameters).children, vec![class]);
            let x = cursor.intern_name(b"x").unwrap();
            let member = graph.binding(graph.own(class, x, Filter::Ordinary).unwrap());
            let parameter = graph.binding(graph.own(parameters, t, Filter::Ordinary).unwrap());
            assert_eq!(member.ty, parameter.ty);
            assert!(graph.entity(entity).template_head.is_some());
            assert!(graph.entity(entity).implicit_default.is_none());
        },
    );
}

#[test]
fn renamed_function_template_parameters_preserve_source_types_and_canonical_identity() {
    analyze(
        b"template<class T>void f(T);template<class U>void f(U p){}",
        |graph, cursor| {
            let scopes = &graph.scope(graph.global).children;
            let first = graph.scope(scopes[0]).bindings[1];
            let second = graph.scope(scopes[1]).bindings[1];
            let first = graph.binding(first);
            let second = graph.binding(second);
            assert_eq!(first.entity, second.entity);
            assert_ne!(first.ty, second.ty);
            let a = graph.types.canonical_function(first.ty.unwrap()).unwrap();
            let b = graph.types.canonical_function(second.ty.unwrap()).unwrap();
            assert_eq!(a, b);
            let f = cursor.intern_name(b"f").unwrap();
            assert_eq!(
                graph
                    .binding(graph.own(graph.global, f, Filter::Ordinary).unwrap())
                    .entity,
                first.entity
            );
        },
    );
}

#[test]
fn positional_template_parameter_identity_includes_inner_head_shape() {
    analyze(b"template<template<class>class A>struct P{};template<template<class X>class B>struct Q{};template<template<class,class>class C>struct R{};", |graph, _| {
        let mut types = Vec::new();
        for &scope in &graph.scope(graph.global).children {
            let binding = graph.binding(graph.scope(scope).bindings[0]);
            types.push(binding.ty.unwrap());
        }
        assert_ne!(types[0], types[1]);
        let a = graph.types.canonical_function(types[0]).unwrap();
        let b = graph.types.canonical_function(types[1]).unwrap();
        let c = graph.types.canonical_function(types[2]).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
    });
}

#[test]
fn standard_conversions_keep_rank_and_recursive_qualification_facts() {
    use super::expression_facts::{ConversionRank, ExprFacts, ValueCategory};
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"conversion-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, _) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    let int = analyzer
        .graph
        .types
        .scalar(Scalar::Basic(FundamentalType::Int))
        .unwrap();
    let short = analyzer.graph.types.scalar(Scalar::Short).unwrap();
    let cint = analyzer.graph.types.qualify(int, Cv::CONST).unwrap();
    let pointer = analyzer.graph.types.pointer(int).unwrap();
    let qualified_pointer = analyzer.graph.types.pointer(cint).unwrap();
    let nested_pointer = analyzer.graph.types.pointer(pointer).unwrap();
    let unsafe_deep_pointer = analyzer.graph.types.pointer(qualified_pointer).unwrap();
    let const_pointer_object = analyzer
        .graph
        .types
        .qualify(qualified_pointer, Cv::CONST)
        .unwrap();
    let deep_const_pointer = analyzer.graph.types.pointer(const_pointer_object).unwrap();
    assert!(
        analyzer
            .pointer_qualification(pointer, qualified_pointer)
            .unwrap()
    );
    assert!(
        !analyzer
            .pointer_qualification(qualified_pointer, pointer)
            .unwrap()
    );
    assert!(
        !analyzer
            .pointer_qualification(nested_pointer, unsafe_deep_pointer)
            .unwrap()
    );
    assert!(
        analyzer
            .pointer_qualification(nested_pointer, deep_const_pointer)
            .unwrap()
    );
    let facts = ExprFacts::new(short, ValueCategory::Lvalue);
    let conversion = analyzer
        .standard_conversion(facts, int, false)
        .unwrap()
        .unwrap();
    assert_eq!(conversion.rank, ConversionRank::Promotion);
    assert_eq!(facts.ty, short);
    let conversion = analyzer
        .standard_conversion(
            ExprFacts::new(pointer, ValueCategory::Lvalue),
            qualified_pointer,
            false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(conversion.rank, ConversionRank::Exact);
    assert!(conversion.qualification);
    let void = analyzer.graph.types.scalar(Scalar::Void).unwrap();
    let cvoid = analyzer.graph.types.qualify(void, Cv::CONST).unwrap();
    let void_pointer = analyzer.graph.types.pointer(void).unwrap();
    let const_void_pointer = analyzer.graph.types.pointer(cvoid).unwrap();
    let facts = ExprFacts::new(pointer, ValueCategory::Lvalue);
    let plain = analyzer
        .standard_conversion(facts, void_pointer, false)
        .unwrap()
        .unwrap();
    let qualified = analyzer
        .standard_conversion(facts, const_void_pointer, false)
        .unwrap()
        .unwrap();
    assert!(!plain.qualification);
    assert!(qualified.qualification);
    assert_eq!(
        analyzer.compare_sequences(plain, qualified).unwrap(),
        std::cmp::Ordering::Greater
    );
    let reference = analyzer
        .graph
        .types
        .intern(
            TypeKind::Reference {
                element: int,
                kind: super::types::RefKind::Lvalue,
            },
            Cv::default(),
        )
        .unwrap();
    assert!(
        analyzer
            .scalar_sequence(ExprFacts::new(int, ValueCategory::Xvalue), reference, false)
            .unwrap()
            .is_none()
    );
}

#[test]
fn resolved_call_keeps_source_operands_and_selected_sequences_on_original_nodes() {
    use super::expression_facts::{ConversionRank, LvalueTransform, ValueCategory};
    use crate::syntax::ast::{Body, DeclKind, ExprKind, StmtKind};
    let source = SourceBuffer::new(
        b"int g(int,long);int g(long,long);int f(short x){return g(x,1);}".to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"call-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let DeclKind::Function { body, .. } = tree
        .declarations
        .get(tree.decl_lists.get(declarations)[2])
        .kind
    else {
        panic!("function expected")
    };
    let Body::Parsed(body) = tree.bodies.get(body) else {
        panic!("parsed body expected")
    };
    let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
        panic!("compound expected")
    };
    let StmtKind::Return(Some(call)) = tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
    else {
        panic!("return expected")
    };
    let ExprKind::Call { callee, arguments } = tree.expressions.get(call).kind else {
        panic!("call expected")
    };
    let arguments = tree.expr_lists.get(arguments);
    let selected = graph.call_binding(call).unwrap();
    let function = graph
        .types
        .canonical_view(graph.binding(selected).ty.unwrap());
    let first_parameter = graph.types.function_parameter(function, 0).unwrap();
    assert!(matches!(
        graph.types.get(first_parameter).kind,
        TypeKind::Scalar(Scalar::Basic(FundamentalType::Int))
    ));
    let operand = graph.expression_facts.get(arguments[0]).unwrap();
    assert!(matches!(
        graph.types.get(operand.ty).kind,
        TypeKind::Scalar(Scalar::Short)
    ));
    assert_eq!(operand.category, ValueCategory::Lvalue);
    let sequence = operand.conversion.unwrap();
    assert_eq!(sequence.target, first_parameter);
    assert_eq!(sequence.rank, ConversionRank::Promotion);
    assert_eq!(sequence.first, LvalueTransform::Read);
    assert_eq!(
        graph
            .expression_facts
            .get(arguments[1])
            .unwrap()
            .conversion
            .unwrap()
            .rank,
        ConversionRank::Conversion
    );
    let target = graph.expression_facts.get(callee).unwrap();
    assert!(target.functions.is_none());
    assert_eq!(
        graph.binding(target.function.unwrap()).entity,
        graph.binding(selected).entity
    );
}

#[test]
fn overload_visibility_keeps_earlier_entities_across_interleaved_redeclarations() {
    use super::graph::{Binding, BindingKind};
    use crate::lex::vocabulary::Keyword;
    let mut graph = Graph::new();
    let source = SourceBuffer::new(b";".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"overload-visibility").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (_, cursor, _) = unit.semantic_inputs().unwrap();
    let name = cursor.intern_name(b"g").unwrap();
    let ty = graph
        .types
        .scalar(Scalar::Basic(FundamentalType::Int))
        .unwrap();
    let a = graph
        .add_entity(Some(ty), Some(name), Keyword::Int)
        .unwrap();
    let b = graph
        .add_entity(Some(ty), Some(name), Keyword::Int)
        .unwrap();
    let binding = |entity| Binding {
        kind: BindingKind::Function,
        name: Some(name),
        entity,
        ty: Some(ty),
        key: Keyword::Int,
    };
    graph.add_binding(graph.global, None, binding(a)).unwrap();
    let limit = graph.binding_limit();
    graph.add_binding(graph.global, None, binding(b)).unwrap();
    graph.add_binding(graph.global, None, binding(a)).unwrap();
    graph.lookup_limit = Some(limit);
    let visible: Vec<_> = graph.functions_at(graph.global, name.into()).collect();
    assert_eq!(visible.len(), 1);
    assert_eq!(graph.binding(visible[0]).entity, a);
    assert_eq!(
        graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity,
        a
    );
    graph.lookup_limit = None;
    assert_eq!(graph.functions_at(graph.global, name.into()).count(), 2);
}

#[test]
fn array_initialization_completes_identity_and_retains_source_operand_conversions() {
    use super::expression_facts::{ConversionRank, ValueCategory};
    use crate::syntax::ast::{DeclKind, ExprKind};
    let source =
        SourceBuffer::new(b"short x; long a[][2]={x,2,3,4}; int tail[1000000000]={1};".to_vec())
            .unwrap();
    let input = Preprocessor::with_name(&source, b"array-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let DeclKind::Simple { declarators, .. } = tree
        .declarations
        .get(tree.decl_lists.get(declarations)[1])
        .kind
    else {
        panic!("array declaration expected");
    };
    let declaration = tree.init_declarators.get(declarators)[0];
    let binding = graph.declared_bindings[declaration.declarator.index()].unwrap();
    let ty = graph.binding(binding).ty.unwrap();
    assert_eq!(graph.entity(graph.binding(binding).entity).ty, Some(ty));
    let TypeKind::Array {
        element: row,
        bound: 2,
    } = graph.types.get(ty).kind
    else {
        panic!("deduced bound expected");
    };
    let TypeKind::Array { element, bound: 2 } = graph.types.get(row).kind else {
        panic!("row bound expected");
    };
    let initializer = declaration.initializer.unwrap();
    let crate::syntax::ast::InitializerKind::Assign(list) = initializer.kind else {
        panic!("list expected");
    };
    let list_facts = graph.expression_facts.get(list).unwrap();
    assert_eq!(list_facts.ty, ty);
    let ExprKind::Braced { elements, .. } = tree.expressions.get(list).kind else {
        panic!("list expected");
    };
    let values = tree.expr_lists.get(elements);
    assert_eq!(values.len(), 4);
    let operand = graph.expression_facts.get(values[0]).unwrap();
    assert_eq!(operand.category, ValueCategory::Lvalue);
    assert!(matches!(
        graph.types.get(operand.ty).kind,
        TypeKind::Scalar(Scalar::Short)
    ));
    let conversion = operand.conversion.unwrap();
    assert_eq!(conversion.target, element);
    assert_eq!(conversion.rank, ConversionRank::Conversion);
    // The billion-element tail creates no per-element semantic records.
    assert_eq!(graph.expression_types.iter().flatten().count(), 7);
}

#[test]
fn member_designation_retains_shared_entity_and_binding_access_without_layout_demand() {
    use super::{expression_facts::ValueCategory, graph::Access};
    use crate::syntax::ast::{Body, DeclKind, ExprKind, StmtKind};
    let source = SourceBuffer::new(
        b"struct A{union{int x;};};int f(const A&a){return a.x;}class Hidden{union{int x;};};"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"member-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let DeclKind::Function { body, .. } = tree
        .declarations
        .get(tree.decl_lists.get(declarations)[1])
        .kind
    else {
        panic!("function expected")
    };
    let Body::Parsed(body) = tree.bodies.get(body) else {
        panic!("parsed body expected")
    };
    let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
        panic!("compound expected")
    };
    let StmtKind::Return(Some(expr)) = tree.statements.get(tree.stmt_lists.get(statements)[0]).kind
    else {
        panic!("return expected")
    };
    let ExprKind::Member { base, member, .. } = tree.expressions.get(expr).kind else {
        panic!("member expected")
    };
    let facts = graph.expression_facts.get(expr).unwrap();
    let binding = facts.member.unwrap();
    assert_eq!(graph.resolved_name(member), Some(binding));
    assert_eq!(facts.category, ValueCategory::Lvalue);
    assert_eq!(graph.types.get(facts.ty).cv, Cv::CONST);
    let member_entity = graph.binding(binding).entity;
    assert_eq!(
        graph.types.get(graph.entity(member_entity).ty.unwrap()).cv,
        Cv::default()
    );
    let base_ty = graph.expression_facts.get(base).unwrap().ty;
    assert!(matches!(graph.types.get(base_ty).kind, TypeKind::Class(_)));
    let x = cursor.intern_name(b"x").unwrap();
    for spelling in [b"A".as_slice(), b"Hidden"] {
        let name = cursor.intern_name(spelling).unwrap();
        let class = graph
            .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
            .entity;
        let scope = graph.entity(class).scope.unwrap();
        let injected = graph.own(scope, x, Filter::Ordinary).unwrap();
        let entity = graph.binding(injected).entity;
        let original_scope = graph.entity(entity).home.unwrap();
        let original = graph.own(original_scope, x, Filter::Ordinary).unwrap();
        assert_ne!(original, injected);
        assert_eq!(graph.binding(original).entity, entity);
        assert!(graph.binding_access(original) == Access::Public);
        assert!(
            graph.binding_access(injected)
                == if spelling == b"A" {
                    Access::Public
                } else {
                    Access::Private
                }
        );
        assert!(matches!(
            graph.entity(class).layout,
            super::layout::LayoutState::NotStarted
        ));
        assert!(matches!(
            graph.entity(class).default_construction,
            super::construction::ConstructorState::NotStarted
        ));
    }
}

#[test]
fn member_addresses_select_canonical_cv_identity_and_close_only_the_selected_demand_list() {
    use super::function_demands::{DefinitionEmission, Demand};
    use crate::syntax::ast::{DeclKind, ExprKind, InitializerKind};
    let source = SourceBuffer::new(
        b"struct Dependency{};struct Unused{};struct C{int f(){Unused u;return 0;}int f()const{Dependency d;return 1;}void query(){}};using P=int(C::*)()const;P first=&C::f;P second=&C::f;unsigned long query=sizeof(&C::query);".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"member-demand-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let mut selected = None;
    for &declaration in &tree.decl_lists.get(declarations)[4..6] {
        let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
            panic!("pointer variable expected");
        };
        let InitializerKind::Assign(address) = tree.init_declarators.get(declarators)[0]
            .initializer
            .unwrap()
            .kind
        else {
            panic!("address initializer expected");
        };
        let facts = graph.expression_facts.get(address).unwrap();
        let binding = facts.function.unwrap();
        assert!(facts.functions.is_none());
        assert!(selected.is_none_or(|prior| prior == binding));
        selected = Some(binding);
        let TypeKind::MemberPointer { owner, element } = graph.types.get(facts.ty).kind else {
            panic!("member pointer expected");
        };
        assert!(matches!(graph.types.get(owner).kind, TypeKind::Class(_)));
        assert!(matches!(
            graph.types.get(element).kind,
            TypeKind::Function { cv: Cv::CONST, .. }
        ));
        let ExprKind::Unary { operand, .. } = tree.expressions.get(address).kind else {
            panic!("address operand expected");
        };
        assert_eq!(graph.expression_facts.get(operand).unwrap().ty, element);
        let function = graph.entity(graph.binding(binding).entity);
        assert_eq!(function.definition_emission, DefinitionEmission::Demanded);
        let scope = function.scope.unwrap();
        assert_eq!(
            graph.scope(scope).owner,
            Some(graph.binding(binding).entity)
        );
        assert_eq!(graph.scope(scope).parent, function.home);
    }
    let trailing = super::function_demands::trailing(&graph);
    assert_eq!(trailing.len(), 2);
    assert!(matches!(trailing[0], Demand::Function { binding, .. } if Some(binding) == selected));
    assert!(matches!(trailing[1], Demand::Constructor(_)));
    let class_name = cursor.intern_name(b"C").unwrap();
    let class = graph
        .binding(graph.own(graph.global, class_name, Filter::Prefix).unwrap())
        .entity;
    assert!(matches!(
        graph.entity(class).layout,
        super::layout::LayoutState::NotStarted
    ));
    assert!(matches!(
        graph.entity(class).default_construction,
        super::construction::ConstructorState::NotStarted
    ));
}

#[test]
fn constructor_plans_keep_entity_identity_and_original_member_initializers() {
    use super::{
        construction::ConstructorState, graph::MemberKind, special_members::SpecialDefinition,
    };
    use crate::syntax::ast::InitializerKind;
    let source = SourceBuffer::new(b"struct C{long x;int y=7;C();C(int n):y(n),x(n){}};C c;C::C():x(3){} C d;struct Unused{int&r;Unused()=default;};".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"constructor-ownership").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let (tree, cursor, declarations) = unit.semantic_inputs().unwrap();
    let mut analyzer = Analyzer::new(tree, cursor, StackBudget::default()).unwrap();
    analyzer.set_semantics(true);
    let graph = analyzer.analyze(declarations).unwrap();
    let class_name = cursor.intern_name(b"C").unwrap();
    let class = graph
        .binding(graph.own(graph.global, class_name, Filter::Prefix).unwrap())
        .entity;
    let ConstructorState::Ready(plan) = graph.entity(class).default_construction else {
        panic!("default constructor was not memoized");
    };
    let selected = graph.constructors.get(plan);
    let binding = selected.binding.unwrap();
    let function = graph.binding(binding).entity;
    assert_eq!(binding, graph.function_origin(function).unwrap());
    assert_eq!(graph.entity(function).constructor, Some(plan));
    assert_eq!(
        graph.entity(function).member_function.unwrap().kind,
        MemberKind::Constructor
    );
    assert!(matches!(
        graph.entity(function).special_definition,
        Some(SpecialDefinition::Body(_))
    ));
    assert_eq!(selected.members.len(), 2);
    for (&member, &(target, _)) in graph.entity(class).members.iter().zip(&selected.members) {
        assert_eq!(member, target);
    }
    let x = selected.members[0].0;
    let x_init = selected.members[0].1.unwrap();
    let InitializerKind::Parenthesized(arguments) = x_init.kind else {
        panic!("written mem-initializer lost its original argument list");
    };
    let expression = tree.expr_lists.get(arguments)[0];
    let facts = graph.expression_facts.get(expression).unwrap();
    assert_eq!(
        facts.conversion.unwrap().target,
        graph.entity(x).ty.unwrap()
    );
    assert_ne!(facts.ty, facts.conversion.unwrap().target);
    let y = selected.members[1].0;
    let y_init = graph.entity(y).member_initializer.unwrap();
    let InitializerKind::Assign(original) = y_init.kind else {
        panic!("member default lost its original expression");
    };
    assert!(
        matches!(selected.members[1].1.unwrap().kind, InitializerKind::Assign(id) if id == original)
    );
    for spelling in [b"c".as_slice(), b"d"] {
        let name = cursor.intern_name(spelling).unwrap();
        let object = graph
            .binding(graph.own(graph.global, name, Filter::Ordinary).unwrap())
            .entity;
        assert_eq!(graph.entity(object).constructor, Some(plan));
    }
    let other = graph.entity(class).declared_constructors[1];
    let other_plan = graph
        .entity(graph.binding(other).entity)
        .constructor
        .unwrap();
    assert_ne!(other_plan, plan);
    let record = graph.constructors.get(other_plan);
    assert_eq!(record.members[0].0, x);
    assert_eq!(record.members[1].0, y);
    let name = cursor.intern_name(b"Unused").unwrap();
    let unused = graph
        .binding(graph.own(graph.global, name, Filter::Prefix).unwrap())
        .entity;
    assert!(matches!(
        graph.entity(unused).default_construction,
        ConstructorState::NotStarted
    ));
    let declaration = graph.entity(unused).declared_constructors[0];
    assert!(
        graph
            .entity(graph.binding(declaration).entity)
            .constructor
            .is_none()
    );
}
