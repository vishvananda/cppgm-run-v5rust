use super::{
    encoder::{Encoder, Target},
    graph::{Graph, Modifier, Node, NodeId, Qualifiers},
    vocabulary::Builtin,
};
use crate::support::stack::StackBudget;

#[test]
fn standard_class_catalog_preserves_arguments_tags_and_explicit_fact_identity() {
    use super::graph::Abbreviation;
    let mut graph = Graph::default();
    let std = graph.intern(Node::Std, &[]).unwrap();
    let char = graph.intern(Node::Builtin(Builtin::Char), &[]).unwrap();
    let int = graph.intern(Node::Builtin(Builtin::Int), &[]).unwrap();
    let text = graph.string(b"char_traits").unwrap();
    let traits = graph.name(Some(std), text, &[]).unwrap();
    let traits_char = graph.intern(Node::Template(traits), &[char]).unwrap();
    let text = graph.string(b"allocator").unwrap();
    let allocator = graph.name(Some(std), text, &[]).unwrap();
    let allocator_char = graph.abbreviated_template_id(allocator, &[char]).unwrap();
    let explicit = graph.intern(Node::Template(allocator), &[char]).unwrap();
    assert_ne!(explicit, allocator_char);
    assert_eq!(graph.get(explicit), Node::Template(allocator));
    let text = graph.string(b"tag").unwrap();
    let cases: &[(&[u8], &[NodeId], &[u8])] = &[
        (b"allocator", &[int], b"SaIiE"),
        (b"basic_string", &[int], b"SbIiE"),
        (b"basic_string", &[char, traits_char, allocator_char], b"Ss"),
        (b"basic_istream", &[char, traits_char], b"Si"),
        (b"basic_ostream", &[char, traits_char], b"So"),
        (b"basic_iostream", &[char, traits_char], b"Sd"),
    ];
    for &(spelling, arguments, expected) in cases {
        let terminal = graph.string(spelling).unwrap();
        let name = graph.name(Some(std), terminal, &[]).unwrap();
        let class = graph.abbreviated_template_id(name, arguments).unwrap();
        assert_eq!(
            class,
            graph.abbreviated_template_id(name, arguments).unwrap()
        );
        let mut output = Vec::new();
        Encoder::encode(
            &graph,
            Target::Type(class),
            &mut output,
            StackBudget::default(),
        )
        .unwrap();
        assert_eq!(output, expected);
        let tagged = graph.tagged(name, &[text]).unwrap();
        let tagged = graph.abbreviated_template_id(tagged, arguments).unwrap();
        assert_ne!(tagged, class);
        let abbreviation = match graph.get(tagged) {
            Node::Template(templ) => templ,
            Node::Abbreviation(_) => tagged,
            _ => panic!("catalog did not abbreviate tagged name"),
        };
        assert_eq!(graph.items(abbreviation), &[text]);
        let nested = graph.name(Some(name), terminal, &[]).unwrap();
        let nested = graph.abbreviated_template_id(nested, arguments).unwrap();
        assert!(matches!(graph.get(nested), Node::Template(parent)
            if matches!(graph.get(parent), Node::Name { .. })));
    }
    let tagged_traits = graph.tagged(traits, &[text]).unwrap();
    let tagged_traits = graph
        .intern(Node::Template(tagged_traits), &[char])
        .unwrap();
    let terminal = graph.string(b"basic_string").unwrap();
    let string = graph.name(Some(std), terminal, &[]).unwrap();
    let expanded = graph
        .abbreviated_template_id(string, &[char, tagged_traits, allocator_char])
        .unwrap();
    assert!(matches!(graph.get(expanded), Node::Template(templ)
        if graph.get(templ) == Node::Abbreviation(Abbreviation::BasicString)));
}

#[test]
fn floating_expression_literals_preserve_ieee_width_and_bits() {
    let mut graph = Graph::default();
    for (builtin, bits, expected) in [
        (Builtin::Float, 0x3f80_0000, "DTLf3f800000EE"),
        (Builtin::Float, 1, "DTLf00000001EE"),
        (Builtin::Float, 0x8000_0000, "DTLf80000000EE"),
        (
            Builtin::Double,
            0x3ff0_0000_0000_0000,
            "DTLd3ff0000000000000EE",
        ),
        (Builtin::Double, 1, "DTLd0000000000000001EE"),
        (
            Builtin::Double,
            0x7ff8_0000_0000_1234,
            "DTLd7ff8000000001234EE",
        ),
    ] {
        let ty = graph.intern(Node::Builtin(builtin), &[]).unwrap();
        let literal = graph
            .intern(Node::FloatingLiteral { ty, bits }, &[])
            .unwrap();
        assert_eq!(
            literal,
            graph
                .intern(Node::FloatingLiteral { ty, bits }, &[])
                .unwrap()
        );
        let decltype = graph
            .intern(
                Node::Decltype {
                    expression: literal,
                    id_expression: false,
                },
                &[],
            )
            .unwrap();
        let mut output = Vec::new();
        Encoder::encode(
            &graph,
            Target::Type(decltype),
            &mut output,
            StackBudget::default(),
        )
        .unwrap();
        assert_eq!(output, expected.as_bytes());
    }
}

#[test]
fn parameter_identity_retains_kind_level_and_substitution_role() {
    use super::graph::ParameterKind;
    let mut graph = Graph::default();
    let mut parameters = Vec::new();
    for level in [0, 1, 2] {
        for kind in [
            ParameterKind::Type,
            ParameterKind::TypePack,
            ParameterKind::Template,
            ParameterKind::TemplatePack,
        ] {
            let node = Node::TemplateParam {
                index: 0,
                level,
                substitutable: true,
                kind,
            };
            let id = graph.intern(node, &[]).unwrap();
            assert!(!parameters.contains(&id));
            assert_eq!(id, graph.intern(node, &[]).unwrap());
            parameters.push(id);
        }
    }
    // Identical spellings of distinct kinds must not share a dictionary entry.
    // A later occurrence of each same canonical parameter must reuse its entry.
    let arguments = parameters
        .iter()
        .chain(&parameters)
        .copied()
        .collect::<Vec<_>>();
    let name = graph.string(b"f").unwrap();
    let name = graph.name(None, name, &[]).unwrap();
    let template = graph.intern(Node::Template(name), &arguments).unwrap();
    let mut output = Vec::new();
    Encoder::encode(
        &graph,
        Target::Entity(template),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(
        output,
        b"_Z1fIT_T_T_T_TL0__TL0__TL0__TL0__TL1__TL1__TL1__TL1__S0_S1_S2_S3_S4_S5_S6_S7_S8_S9_SA_SB_E"
    );
    let value = graph
        .intern(
            Node::TemplateParam {
                index: 0,
                level: 0,
                substitutable: false,
                kind: ParameterKind::Type,
            },
            &[],
        )
        .unwrap();
    assert_ne!(value, parameters[0]);
    let value = graph.intern(Node::ExpressionArgument(value), &[]).unwrap();
    let template = graph
        .intern(
            Node::Template(name),
            &[parameters[0], value, parameters[0], value],
        )
        .unwrap();
    output.clear();
    Encoder::encode(
        &graph,
        Target::Entity(template),
        &mut output,
        StackBudget::default(),
    )
    .unwrap();
    assert_eq!(output, b"_Z1fIT_XT_ES0_XT_EE");
}

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
    let param = graph
        .intern(Node::FunctionParam { index: 0, level: 0 }, &[])
        .unwrap();
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

#[test]
fn reference_temporary_sequences_cover_base36_boundaries() {
    let mut graph = Graph::default();
    let terminal = graph.string(b"r").unwrap();
    let owner = graph.name(None, terminal, &[]).unwrap();
    for (sequence, suffix) in [
        (0, ""),
        (1, "0"),
        (36, "Z"),
        (37, "10"),
        (1297, "100"),
        (u32::MAX, "1Z141Z2"),
    ] {
        let mut bytes = Vec::new();
        Encoder::encode(
            &graph,
            Target::ReferenceTemporary { owner, sequence },
            &mut bytes,
            StackBudget::default(),
        )
        .unwrap();
        assert_eq!(bytes, format!("_ZGR1r{suffix}_").as_bytes());
    }
}
