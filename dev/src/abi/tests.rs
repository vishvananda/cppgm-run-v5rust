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
