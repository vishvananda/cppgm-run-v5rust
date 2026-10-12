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
    let super::ast::StmtKind::Compound(items) = unit.parser.tree.statements.get(body).kind else {
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
