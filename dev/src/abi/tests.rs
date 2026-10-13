use super::{
    encoder::{Encoder, Target},
    graph::{Graph, Modifier, Node, NodeId, Qualifiers},
    vocabulary::Builtin,
};
use crate::support::stack::StackBudget;

#[test]
fn canonical_cv_and_member_function_owners_survive_store_growth() {
    assert_eq!(std::mem::size_of::<Option<NodeId>>(), 4);
    let mut graph = Graph::default();
    let int = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    let c = graph
        .modified(Modifier::Qualified(Qualifiers::CONST), int)
        .unwrap();
    let cv = graph
        .modified(Modifier::Qualified(Qualifiers::VOLATILE), c)
        .unwrap();
    let v = graph
        .modified(Modifier::Qualified(Qualifiers::VOLATILE), int)
        .unwrap();
    let vc = graph
        .modified(Modifier::Qualified(Qualifiers::CONST), v)
        .unwrap();
    assert_eq!(cv, vc);
    let function = graph
        .intern(
            Node::FunctionType {
                result: int,
                owner: None,
                qualifiers: Qualifiers::default(),
                variadic: false,
            },
            &[int, cv],
        )
        .unwrap();
    let qualified_function = graph
        .modified(Modifier::Qualified(Qualifiers::CONST), function)
        .unwrap();
    assert!(
        matches!(graph.get(qualified_function), Node::FunctionType { qualifiers, .. } if qualifiers.contains(Qualifiers::CONST))
    );
    let a = graph.string(b"A").unwrap();
    let a = graph.name(None, a, &[]).unwrap();
    let b = graph.string(b"B").unwrap();
    let b = graph.name(None, b, &[]).unwrap();
    let ma = graph.member_pointer(a, function).unwrap();
    let mb = graph.member_pointer(b, function).unwrap();
    assert_ne!(ma, mb);
    for index in 0..4096 {
        graph.string(index.to_string().as_bytes()).unwrap();
    }
    assert_eq!(graph.items(function), &[int, cv]);
    assert_eq!(ma, graph.member_pointer(a, function).unwrap());
    assert_eq!(
        cv,
        graph
            .modified(
                Modifier::Qualified(Qualifiers::CONST.union(Qualifiers::VOLATILE)),
                int
            )
            .unwrap()
    );
}

#[test]
fn long_type_and_prefix_chains_are_iterative_and_substitutions_are_per_name() {
    let mut graph = Graph::default();
    let mut ty = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    for _ in 0..10_000 {
        ty = graph.modified(Modifier::Pointer, ty).unwrap();
    }
    let mut output = Vec::new();
    Encoder::encode(
        &graph,
        Target::Type(ty),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(output.len(), 10_001);
    assert!(output[..10_000].iter().all(|c| *c == b'P'));
    assert_eq!(output[10_000], b'i');

    let text = graph.string(b"x").unwrap();
    let mut name = graph.name(None, text, &[]).unwrap();
    for _ in 0..10_000 {
        name = graph.name(Some(name), text, &[]).unwrap();
    }
    output.clear();
    Encoder::encode(
        &graph,
        Target::Type(name),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    let first = output.clone();
    output.clear();
    Encoder::encode(
        &graph,
        Target::Type(name),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(first, output);
    assert_eq!(output.len(), 20_004);
}

#[test]
fn tags_and_values_use_canonical_structure_instead_of_definition_binders() {
    let mut graph = Graph::default();
    let a = graph.string(b"alpha").unwrap();
    let z = graph.string(b"zeta").unwrap();
    let text = graph.string(b"C").unwrap();
    let first = graph.name(None, text, &[z, a, a]).unwrap();
    let second = graph.name(None, text, &[a, z]).unwrap();
    assert_eq!(first, second);
    let int = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    let one = graph
        .intern(Node::Literal { ty: int, bits: 1 }, &[])
        .unwrap();
    let same = graph
        .intern(Node::Literal { ty: int, bits: 1 }, &[])
        .unwrap();
    let different = graph
        .intern(Node::Literal { ty: int, bits: 2 }, &[])
        .unwrap();
    assert_eq!(one, same);
    assert_ne!(one, different);
    let array_two = graph.modified(Modifier::Array(2), int).unwrap();
    let array_three = graph.modified(Modifier::Array(3), int).unwrap();
    assert_ne!(array_two, array_three);
}

#[test]
fn expression_identity_retains_operands_and_id_expression_category() {
    use super::vocabulary::Operation;
    let mut graph = Graph::default();
    let int = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    let long = graph.intern(Node::Builtin(Builtin::Long), &[]).unwrap();
    let name = graph.string(b"trait").unwrap();
    let first = graph
        .intern(Node::VendorExpression(name), &[int, long])
        .unwrap();
    let same = graph
        .intern(Node::VendorExpression(name), &[int, long])
        .unwrap();
    let reverse = graph
        .intern(Node::VendorExpression(name), &[long, int])
        .unwrap();
    assert_eq!(first, same);
    assert_ne!(first, reverse);
    let param = graph.intern(Node::FunctionParam(0), &[]).unwrap();
    let one = graph
        .intern(Node::Literal { ty: int, bits: 1 }, &[])
        .unwrap();
    let two = graph
        .intern(Node::Literal { ty: int, bits: 2 }, &[])
        .unwrap();
    let add_one = graph
        .intern(Node::Operation(Operation::Add), &[param, one])
        .unwrap();
    let add_two = graph
        .intern(Node::Operation(Operation::Add), &[param, two])
        .unwrap();
    let bound_one = graph
        .modified(Modifier::ExpressionArray(add_one), int)
        .unwrap();
    let bound_two = graph
        .modified(Modifier::ExpressionArray(add_two), int)
        .unwrap();
    assert_ne!(bound_one, bound_two);
    let bare = graph
        .intern(
            Node::Decltype {
                expression: param,
                id_expression: true,
            },
            &[],
        )
        .unwrap();
    let parenthesized = graph
        .intern(
            Node::Decltype {
                expression: param,
                id_expression: false,
            },
            &[],
        )
        .unwrap();
    assert_ne!(bare, parenthesized);
    for index in 0..4096 {
        graph.string(index.to_string().as_bytes()).unwrap();
    }
    assert_eq!(graph.items(add_one), &[param, one]);
    assert_eq!(graph.items(first), &[int, long]);
    let mut out = Vec::new();
    Encoder::encode(
        &graph,
        Target::Type(bound_one),
        &mut out,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(out, b"Aplfp_Li1E_i");
}

#[test]
fn local_scope_identity_and_closure_signature_survive_store_growth() {
    let mut graph = Graph::default();
    let int = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    let text = graph.string(b"f").unwrap();
    let name = graph.name(None, text, &[]).unwrap();
    let encoding = graph
        .intern(
            Node::Function {
                name,
                result: None,
                qualifiers: Qualifiers::default(),
                variadic: false,
            },
            &[int],
        )
        .unwrap();
    let scope = Node::FunctionScope {
        encoding,
        default_argument: 0,
    };
    let first = graph.intern(scope, &[]).unwrap();
    assert_eq!(first, graph.intern(scope, &[]).unwrap());
    let default_argument = graph
        .intern(
            Node::FunctionScope {
                encoding,
                default_argument: 2,
            },
            &[],
        )
        .unwrap();
    assert_ne!(first, default_argument);
    let closure = graph
        .intern(
            Node::ClosureName {
                number: 1,
                variadic: true,
            },
            &[int],
        )
        .unwrap();
    let local = Node::Name {
        parent: Some(default_argument),
        terminal: closure,
        internal: false,
        occurrence: 11,
    };
    let ty = graph.intern(local, &[]).unwrap();
    let other = graph
        .intern(
            Node::Name {
                parent: Some(default_argument),
                terminal: closure,
                internal: false,
                occurrence: 10,
            },
            &[],
        )
        .unwrap();
    assert_ne!(ty, other);
    for index in 0..4096 {
        graph.string(index.to_string().as_bytes()).unwrap();
    }
    assert_eq!(graph.items(closure), &[int]);
    assert_eq!(ty, graph.intern(local, &[]).unwrap());
    let mut output = Vec::new();
    Encoder::encode(
        &graph,
        Target::Type(ty),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(output, b"Z1fiEd0_UlizE0___10_");
}
