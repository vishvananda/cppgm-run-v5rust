//! Ownership and speculation invariants independent of the dump vocabulary.
use super::{
    ParseError, SyntaxUnit,
    ast::DeclKind,
    cursor::{Cursor, TokenKind},
    parser::Parser,
    scopes::Category,
};
use crate::{
    lex::{PostTokenizer, number::NumberDialect},
    preprocess::unit::Preprocessor,
    support::source::SourceBuffer,
};

#[test]
fn failed_outer_attempt_rewinds_committed_inner_scope_and_all_syntax() {
    let source = SourceBuffer::new(b"namespace N { using A = int; }".to_vec()).unwrap();
    let preprocessor = Preprocessor::with_name(&source, b"transaction").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        preprocessor,
        NumberDialect::Cxx11,
    )));
    let before = parser.tree.declarations.mark();
    let before_names = parser.tree.names.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.attempt(Parser::declaration)?.unwrap();
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.cursor.position, 0);
    assert_eq!(parser.tree.declarations.mark(), before);
    assert_eq!(parser.tree.names.mark(), before_names);
    let TokenKind::Identifier(namespace_name) = parser.cursor.peek(1).unwrap().kind else {
        panic!("namespace identifier");
    };
    assert!(parser.scopes.lookup(namespace_name).is_none());
    let declaration = parser.declaration().unwrap();
    assert!(matches!(
        parser.tree.declarations.get(declaration).kind,
        DeclKind::Namespace { .. }
    ));
    assert_eq!(
        parser.scopes.lookup(namespace_name).unwrap().category,
        Category::Namespace
    );
}

#[test]
fn abandoned_parameter_names_do_not_shadow_the_following_type_declaration() {
    let source = SourceBuffer::new(b"typedef int T; int x(int(T) + 1); T y;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"parameter-rollback").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let root = unit.declarations.unwrap();
    assert_eq!(unit.parser.tree.decl_lists.get(root).len(), 3);
    let mut output = Vec::new();
    unit.print(&mut output).unwrap();
    assert!(
        output
            .windows(b"decl-specifier TT_IDENTIFIER:T".len())
            .any(|window| window == b"decl-specifier TT_IDENTIFIER:T")
    );
}

#[test]
fn token_compaction_preserves_absolute_diagnostic_order_and_bounds_the_window() {
    let mut bytes = Vec::new();
    for _ in 0..6000 {
        bytes.extend_from_slice(b"int value;\n");
    }
    bytes.extend_from_slice(b"int broken(;");
    let source = SourceBuffer::new(bytes).unwrap();
    let input = Preprocessor::with_name(&source, b"compaction").unwrap();
    let mut unit = SyntaxUnit::new(input);
    let error = unit.parse().unwrap_err();
    assert!(matches!(error, ParseError::Syntax { .. }));
    assert_eq!(
        unit.describe(&error).unwrap(),
        b"compaction:6001:12: syntax error"
    );
    assert!(unit.parser.cursor.peak_window <= 4098);
    assert_eq!(unit.parser.tree.declarations.mark(), 6000);
}

#[test]
fn iterative_dump_and_render_do_not_recurse_on_left_associated_chains() {
    let mut bytes = b"void f() {".to_vec();
    for _ in 0..1999 {
        bytes.extend_from_slice(b"value+");
    }
    bytes.extend_from_slice(b"value;}\n");
    let source = SourceBuffer::new(bytes).unwrap();
    let input = Preprocessor::with_name(&source, b"chain").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    unit.print(&mut std::io::sink()).unwrap();
    assert!(unit.parser.peak_nesting < 20);
}

#[test]
fn aliases_retain_enum_member_categories_and_unnamed_namespaces_reopen() {
    let source = SourceBuffer::new(b"namespace { enum class E { SomeType }; } namespace { using Alias = E; typedef E Other; } void f() { Alias::SomeType * value; Other::SomeType * value; }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"name-categories").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let root = unit.parser.tree.decl_lists.get(unit.declarations.unwrap());
    let DeclKind::Function { body, .. } = unit.parser.tree.declarations.get(root[2]).kind else {
        panic!("function");
    };
    let super::ast::Body::Parsed(body) = *unit.parser.tree.bodies.get(body) else {
        panic!("parsed body");
    };
    let super::ast::StmtKind::Compound(items) = unit.parser.tree.statements.get(body.compound).kind
    else {
        panic!("body");
    };
    for &item in unit.parser.tree.stmt_lists.get(items) {
        assert!(matches!(
            unit.parser.tree.statements.get(item).kind,
            super::ast::StmtKind::Expression(Some(_))
        ));
    }
    assert!(
        unit.parser
            .scopes
            .unnamed(super::scopes::Scopes::global())
            .is_some()
    );
}

#[test]
fn handlers_share_parameter_scope_restore_siblings_and_rewind_nested_lists() {
    use super::ast::{ExceptionDeclaration, StmtKind};
    let source = SourceBuffer::new(b"typedef int C; void f() try { try {} catch (int C) { C * value; } } catch (C& value) {} catch (...) {} C next;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"handler-ownership").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let handler_count = parser.tree.handlers.mark();
    let position = parser.cursor.position;
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.attempt(Parser::declaration)?.unwrap();
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.cursor.position, position);
    // Every scratch/list slab, including inner committed handler lists, rolls back.
    assert_eq!(parser.tree.handlers.mark(), handler_count);
    let declaration = parser.declaration().unwrap();
    let DeclKind::Function { body, .. } = parser.tree.declarations.get(declaration).kind else {
        panic!("function");
    };
    let super::ast::Body::Parsed(body) = *parser.tree.bodies.get(body) else {
        panic!("parsed body");
    };
    let handlers = parser.tree.handlers.get(body.handlers.unwrap());
    assert_eq!(handlers.len(), 2);
    assert!(matches!(
        handlers[0].exception,
        ExceptionDeclaration::Typed { .. }
    ));
    assert!(matches!(
        handlers[1].exception,
        ExceptionDeclaration::Ellipsis
    ));
    let StmtKind::Compound(items) = parser.tree.statements.get(body.compound).kind else {
        panic!("function body");
    };
    let StmtKind::Try { handlers, .. } = parser
        .tree
        .statements
        .get(parser.tree.stmt_lists.get(items)[0])
        .kind
    else {
        panic!("nested try");
    };
    let handler = parser.tree.handlers.get(handlers)[0];
    let StmtKind::Compound(items) = parser.tree.statements.get(handler.body).kind else {
        panic!("catch body");
    };
    assert!(matches!(
        parser
            .tree
            .statements
            .get(parser.tree.stmt_lists.get(items)[0])
            .kind,
        StmtKind::Expression(Some(_))
    ));
    parser.declaration().unwrap();
}

#[test]
fn allocation_modes_retain_type_parentheses_placement_and_initializers() {
    use super::ast::{ExprKind, Initializer, StmtKind, Suffix};
    let source = SourceBuffer::new(b"void f(void* storage) { new (int); new (storage) int; new (char*)(); new int[3][4](1); new (storage) (int(*)[3]); }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"allocation-modes").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let DeclKind::Function { body, .. } = tree
        .declarations
        .get(tree.decl_lists.get(unit.declarations.unwrap())[0])
        .kind
    else {
        panic!("function");
    };
    let super::ast::Body::Parsed(body) = *tree.bodies.get(body) else {
        panic!("parsed body");
    };
    let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
        panic!("body");
    };
    let allocations: Vec<_> = tree
        .stmt_lists
        .get(items)
        .iter()
        .map(|&item| {
            let StmtKind::Expression(Some(expr)) = tree.statements.get(item).kind else {
                panic!("expression");
            };
            tree.expressions.get(expr).kind
        })
        .collect();
    assert!(matches!(
        allocations[0],
        ExprKind::New {
            parenthesized_type: true,
            placement: None,
            initializer: None,
            ..
        }
    ));
    assert!(matches!(
        allocations[1],
        ExprKind::New {
            parenthesized_type: false,
            placement: Some(_),
            initializer: None,
            ..
        }
    ));
    assert!(matches!(
        allocations[2],
        ExprKind::New {
            parenthesized_type: true,
            initializer: Some(Initializer::Parenthesized(_)),
            ..
        }
    ));
    let ExprKind::New {
        ty,
        parenthesized_type: false,
        initializer: Some(_),
        ..
    } = allocations[3]
    else {
        panic!("array new");
    };
    let declarator = tree.declarators.get(tree.types.get(ty).declarator.unwrap());
    assert!(declarator.abstract_form);
    let suffixes = tree.suffixes.get(declarator.suffixes);
    assert_eq!(suffixes.len(), 2);
    assert!(
        suffixes
            .iter()
            .all(|suffix| matches!(suffix, Suffix::Array(Some(_))))
    );
    assert!(matches!(
        allocations[4],
        ExprKind::New {
            parenthesized_type: true,
            placement: Some(_),
            ..
        }
    ));
}

#[test]
fn qualified_special_names_and_decltype_render_from_typed_components() {
    let source = SourceBuffer::new(b"namespace N { int value; } decltype(N::value)::Other object; void f() { N::operator new; N::operator int; N::operator ::Y*; N::operator and; N::operator\"\"if; p->~decltype(N::value)(); int x = a ? throw b : throw; noexcept(::delete[] p); }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"structured-names").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let mut output = Vec::new();
    unit.print(&mut output).unwrap();
    for label in [
        b"N::operator new".as_slice(),
        b"N::operator int",
        b"N::operator::Y*",
        b"N::operator and",
        b"N::operator\"\"if",
        b"~decltype(N::value)",
        b"qualified-type-name Other",
        b"throw-expression",
    ] {
        assert!(output.windows(label.len()).any(|window| window == label));
    }
}

#[test]
fn class_completion_parses_each_body_with_later_member_categories() {
    use super::ast::{Body, Specifier, StmtKind};
    let source = SourceBuffer::new(b"struct Base { typedef int inherited; }; struct Outer : Base { void f() { Slot * p; inherited q; struct Local { void g() { Later value; } typedef int Later; }; } struct Slot {}; };".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"class-completion").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let root = tree.decl_lists.get(unit.declarations.unwrap());
    let DeclKind::Class(Specifier::Class(outer)) = tree.declarations.get(root[1]).kind else {
        panic!("outer class");
    };
    let members = tree.decl_lists.get(tree.classes.get(outer).members);
    let DeclKind::Function { body, .. } = tree.declarations.get(members[0]).kind else {
        panic!("inline method");
    };
    let Body::Parsed(body) = *tree.bodies.get(body) else {
        panic!("completed body");
    };
    let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
        panic!("body");
    };
    assert_eq!(tree.stmt_lists.get(items).len(), 3);
    assert!(
        tree.stmt_lists
            .get(items)
            .iter()
            .all(|&id| matches!(tree.statements.get(id).kind, StmtKind::Declaration(_)))
    );
    assert_eq!(unit.parser.cursor.runs_mark(), 0);
    assert!(unit.parser.deferred.is_empty());
    assert!(unit.parser.scopes.complete(tree.classes.get(outer).scope));
}

#[test]
fn failed_class_attempt_rewinds_bodies_scope_completion_and_all_member_lists() {
    let source = SourceBuffer::new(
        b"struct C { C() : field{1} {} void f() { later * p; } int field:3; struct later {}; };"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"class-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let classes = parser.tree.classes.mark();
    let bodies = parser.tree.bodies.mark();
    let fields = parser.tree.bit_fields.mark();
    let initializers = parser.tree.mem_initializers.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.attempt(Parser::declaration)?.unwrap();
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.tree.classes.mark(), classes);
    assert_eq!(parser.tree.bodies.mark(), bodies);
    assert_eq!(parser.tree.bit_fields.mark(), fields);
    assert_eq!(parser.tree.mem_initializers.mark(), initializers);
    assert_eq!(parser.class_depth, 0);
    assert!(parser.deferred.is_empty());
    assert_eq!(parser.cursor.runs_mark(), 0);
    let TokenKind::Identifier(name) = parser.cursor.peek(1).unwrap().kind else {
        panic!("class name");
    };
    assert!(parser.scopes.lookup(name).is_none());
    parser.declaration().unwrap();
    let scope = parser.scopes.lookup(name).unwrap().scope.unwrap();
    assert!(parser.scopes.complete(scope));
}

#[test]
fn successive_classes_release_body_tokens_and_keep_the_pull_window_bounded() {
    let mut bytes = Vec::new();
    for index in 0..2000 {
        bytes.extend_from_slice(
            format!("struct C{index} {{ void f() {{ Later p; }} typedef int Later; }};\n")
                .as_bytes(),
        );
    }
    let source = SourceBuffer::new(bytes).unwrap();
    let input = Preprocessor::with_name(&source, b"class-scaling").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    assert_eq!(unit.parser.tree.bodies.mark(), 2000);
    assert_eq!(unit.parser.cursor.runs_mark(), 0);
    assert!(unit.parser.cursor.peak_run_tokens < 10);
    assert!(unit.parser.cursor.peak_window < 4120);
}
