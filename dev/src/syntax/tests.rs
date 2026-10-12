//! Ownership and speculation invariants independent of the dump vocabulary.
use super::ast::{InitializerKind, QualifierKind, SpecifierKind};
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

fn class_syntax(
    tree: &super::ast::Tree,
    declaration: super::ast::DeclId,
) -> &super::ast::ClassSyntax {
    let DeclKind::Class(specifier) = tree.declarations.get(declaration).kind else {
        panic!("class declaration")
    };
    let SpecifierKind::Class(class) = specifier.kind else {
        panic!("class specifier")
    };
    tree.classes.get(class)
}

fn source_offset(source: &[u8], spelling: &[u8]) -> crate::support::source::SourceOffset {
    crate::support::source::SourceOffset(
        u32::try_from(
            source
                .windows(spelling.len())
                .position(|part| part == spelling)
                .unwrap(),
        )
        .unwrap(),
    )
}

#[test]
fn speculation_boundaries_preserve_enclosing_angle_mode() {
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"class-boundary-angle").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.angle = true;
    parser.speculation = 1;
    let result: Result<(), ParseError> = parser.speculation_boundary(|parser| {
        assert!(parser.angle);
        assert_eq!(parser.speculation, 0);
        parser.decline_operand()
    });
    assert!(result.is_err());
    assert!(parser.angle);
    assert_eq!(parser.speculation, 1);
}

#[test]
fn committed_lambda_bodies_restore_the_enclosing_parameter_scope() {
    let source = SourceBuffer::new(
        b"void f(int x); template<int N>struct P{}; P<([]{void helper(int v);return 0;}())> p;"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-parameter-scope").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let parameter = parser.parameter_scope.unwrap();
    parser.declaration().unwrap();
    assert_eq!(parser.parameter_scope, Some(parameter));
    assert_eq!(parser.deferred_lambdas, 1);
}

#[test]
fn template_and_pack_mention_counters_wrap_at_target_width() {
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"mention-counter-wrap").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.template_mentions = u32::MAX;
    parser.pack_mentions = u32::MAX;
    parser.observe_name_binding(Some(super::scopes::Binding {
        category: Category::Type,
        pack: true,
        parameter: true,
        scope: None,
        specialization: 0,
        alternatives: super::scopes::Categories::default(),
        guesses: super::scopes::Guesses::default(),
    }));
    assert_eq!(parser.template_mentions, 0);
    assert_eq!(parser.pack_mentions, 0);
}

#[test]
fn name_boundaries_leave_scope_tokens_for_their_owning_grammar() {
    let source = SourceBuffer::new(b"C::+1".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"name-boundary").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let (name, _) = parser.name().unwrap();
    assert_eq!(
        parser
            .tree
            .name_lists
            .get(parser.tree.names.get(name).components)
            .len(),
        1
    );
    assert!(parser.at(crate::lex::pp_lexer::Punctuator::Scope).unwrap());
}

#[test]
fn simple_operator_names_do_not_allocate_a_conversion_lookup_scope() {
    let source = SourceBuffer::new(b"struct C{}; C::operator+".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"operator-scope-demand").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let environment = parser.scopes.environment();
    parser.name().unwrap();
    assert_eq!(parser.scopes.environment(), environment);
}

#[test]
fn decltype_prefixes_do_not_invent_conflicted_categories() {
    let source = SourceBuffer::new(b"struct C{}; decltype(C())::Unknown".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"decltype-prefix-facts").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.conflicts.active = true;
    parser.conflicts.uses = u32::MAX;
    let (name, binding) = parser.name().unwrap();
    assert!(binding.is_none());
    assert_eq!(
        parser.tree.names.get(name).prefix_state,
        super::ast::PrefixState::Dependent
    );
    assert_eq!(parser.conflicts.uses, u32::MAX);
    assert!(parser.guessed_component(None).is_none());
    assert_eq!(parser.conflicts.uses, 0);
}

#[test]
fn class_attributes_do_not_inherit_the_declared_templates_category() {
    let source =
        SourceBuffer::new(b"template<class T> struct alignas(sizeof(struct Word*)) C{};".to_vec())
            .unwrap();
    let input = Preprocessor::with_name(&source, b"class-attribute-template-context").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let word = parser.cursor.intern_name(b"Word").unwrap();
    let class = parser.cursor.intern_name(b"C").unwrap();
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
    assert_eq!(
        parser.scopes.lookup(class).unwrap().category,
        Category::TypeTemplate
    );
}

#[test]
fn replacement_class_scopes_keep_the_previous_members_reachable() {
    let source =
        SourceBuffer::new(b"struct C{using Old=int;}; struct C{using New=int;};".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"class-scope-replacement").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let class = parser.cursor.intern_name(b"C").unwrap();
    let old = parser.cursor.intern_name(b"Old").unwrap();
    let first = parser.scopes.lookup(class).unwrap().scope.unwrap();
    parser.declaration().unwrap();
    let replacement = parser.scopes.lookup(class).unwrap().scope.unwrap();
    assert_ne!(replacement, first);
    assert_eq!(
        parser
            .scopes
            .lookup_specialization(replacement, old, false, false)
            .unwrap()
            .category,
        Category::Type
    );
}

#[test]
fn qualified_definitions_keep_the_members_declared_category() {
    let source =
        SourceBuffer::new(b"struct A{template<class T> struct C;}; struct A::C{};".to_vec())
            .unwrap();
    let input = Preprocessor::with_name(&source, b"qualified-class-category").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let owner = parser.cursor.intern_name(b"A").unwrap();
    let class = parser.cursor.intern_name(b"C").unwrap();
    let owner = parser.scopes.lookup(owner).unwrap().scope.unwrap();
    let before = parser.scopes.own(owner, class).unwrap();
    parser.declaration().unwrap();
    let after = parser.scopes.own(owner, class).unwrap();
    assert_eq!(after.scope, before.scope);
    assert_eq!(after.category, Category::TypeTemplate);
    assert_eq!(
        parser
            .scopes
            .own(after.scope.unwrap(), class)
            .unwrap()
            .category,
        Category::Type
    );
}

#[test]
fn failed_namespace_and_template_heads_restore_the_enclosing_scope() {
    for bytes in [
        b"namespace N{int x=; }".as_slice(),
        b"template<class T, +> int x;",
    ] {
        let source = SourceBuffer::new(bytes.to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"declaration-scope-unwind").unwrap();
        let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
            input,
            NumberDialect::Cxx11,
        )));
        let enclosing = parser.scopes.current;
        assert!(parser.declaration().is_err());
        assert_eq!(parser.scopes.current, enclosing);
        assert_eq!(parser.active_nesting(), 0);
    }
}

#[test]
fn template_initializer_types_are_ordinary_while_the_outer_variable_is_a_template() {
    let source =
        SourceBuffer::new(b"template<class T> int x=sizeof(struct word*);".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"initializer-template-demand").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let x = parser.cursor.intern_name(b"x").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    assert_eq!(
        parser.scopes.lookup(x).unwrap().category,
        Category::FunctionTemplate
    );
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
    assert!(!parser.pending_template);
}

#[test]
fn enum_defining_template_declarations_do_not_make_their_objects_templates() {
    let source = SourceBuffer::new(b"template<class T> enum E{value} object;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"defined-type-template-facts").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let object = parser.cursor.intern_name(b"object").unwrap();
    assert_eq!(
        parser.scopes.lookup(object).unwrap().category,
        Category::Value
    );
}

#[test]
fn qualified_bounds_share_their_block_and_store_the_effective_member_context() {
    let source = SourceBuffer::new(
        b"namespace N{} int (N::arr)[sizeof(struct ghost*)][sizeof(ghost)];".to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"bound-context-ownership").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let declaration = parser.declaration().unwrap();
    let DeclKind::Simple { declarators, .. } = parser.tree.declarations.get(declaration).kind
    else {
        panic!("simple")
    };
    let d = parser.tree.init_declarators.get(declarators)[0].declarator;
    let node = parser.tree.declarators.get(d);
    assert!(node.member_scope.is_some());
    let super::ast::Suffix::Array(Some(bound)) = parser.tree.suffixes.get(node.suffixes)[1] else {
        panic!("array")
    };
    assert!(matches!(
        parser.tree.expressions.get(bound).kind,
        super::ast::ExprKind::Sizeof { ty: Some(_), .. }
    ));
    let ghost = parser.cursor.intern_name(b"ghost").unwrap();
    assert!(parser.scopes.lookup(ghost).is_none());
}

#[test]
fn body_scope_is_selected_from_the_first_direct_function_clause() {
    let source = SourceBuffer::new(b"int f(int C)(int Y);".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"function-clause-scope").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let body = parser.function_scope.unwrap();
    let last = parser.parameter_scope.unwrap();
    assert_ne!(body, last);
    let c = parser.cursor.intern_name(b"C").unwrap();
    let y = parser.cursor.intern_name(b"Y").unwrap();
    assert_eq!(
        parser.scopes.own(body, c).unwrap().category,
        Category::Value
    );
    assert!(parser.scopes.own(body, y).is_none());
    assert_eq!(
        parser.scopes.own(last, y).unwrap().category,
        Category::Value
    );
}

#[test]
fn ordinary_function_qualifiers_use_outer_names_and_asm_retains_keyword_location() {
    let bytes = br#"struct C{}; void f(int C) noexcept(sizeof(C)); int x asm("symbol");"#;
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"qualifier-lookup-location").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let declaration = parser.declaration().unwrap();
    let DeclKind::Simple { declarators, .. } = parser.tree.declarations.get(declaration).kind
    else {
        panic!("simple declaration")
    };
    let d = parser.tree.init_declarators.get(declarators)[0].declarator;
    let super::ast::Suffix::Function { qualifiers, .. } = parser
        .tree
        .suffixes
        .get(parser.tree.declarators.get(d).suffixes)[0]
    else {
        panic!("function suffix")
    };
    let QualifierKind::Noexcept(Some(value)) = parser.tree.qualifiers.get(qualifiers)[0].kind
    else {
        panic!("noexcept")
    };
    assert!(matches!(
        parser.tree.expressions.get(value).kind,
        super::ast::ExprKind::Sizeof { ty: Some(_), .. }
    ));
    let declaration = parser.declaration().unwrap();
    let DeclKind::Simple { declarators, .. } = parser.tree.declarations.get(declaration).kind
    else {
        panic!("simple declaration")
    };
    let d = parser.tree.init_declarators.get(declarators)[0].declarator;
    let attribute = parser
        .tree
        .attributes
        .get(parser.tree.declarators.get(d).attributes)[0];
    assert_eq!(attribute.location, source_offset(bytes, b"asm"));
}

#[test]
fn failed_parameter_clause_restores_scope_and_transactional_parameter_identity() {
    let source = SourceBuffer::new(b"(int x, +)".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"parameter-scope-unwind").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let outer = parser.scopes.current;
    assert!(parser.parameter_clause_member(true, None).is_err());
    assert_eq!(parser.scopes.current, outer);
    assert!(parser.parameter_scope.is_none());
    assert_eq!(parser.active_nesting(), 0);
}

#[test]
fn condition_name_is_visible_in_its_initializer_and_block_scope_unwinds() {
    let source = SourceBuffer::new(b"struct C{}; if(int C=sizeof(C)) {C *x;}".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"condition-publication").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let enclosing = parser.scopes.current;
    let statement = parser
        .statement_reading(
            super::conflicts::StatementReader::Control,
            super::storage::List::default(),
        )
        .unwrap();
    let super::ast::StmtKind::If { condition, .. } = parser.tree.statements.get(statement).kind
    else {
        panic!("if statement")
    };
    let super::ast::Condition::Declaration { initializer, .. } = condition else {
        panic!("condition declaration")
    };
    let InitializerKind::Assign(value) = initializer.kind else {
        panic!("initializer")
    };
    assert!(matches!(
        parser.tree.expressions.get(value).kind,
        super::ast::ExprKind::Sizeof {
            ty: None,
            operand: Some(_),
            ..
        }
    ));
    assert_eq!(parser.scopes.current, enclosing);
    let c = parser.cursor.intern_name(b"C").unwrap();
    assert_eq!(parser.scopes.lookup(c).unwrap().category, Category::Type);
}

#[test]
fn failed_template_lists_retry_under_another_category_reading() {
    let source = SourceBuffer::new(b"using G=Unknown; <G::V*>".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"template-category-key").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.cursor.retain();
    let position = parser.cursor.position;
    parser.conflicts.active = true;
    parser.conflicts.take = Some(Category::Value);
    assert!(parser.optional_template_arguments(false).unwrap().is_none());
    assert_eq!(parser.cursor.position, position);
    let failures = parser.angles.cached_queries().0;
    parser.angle = true;
    assert!(parser.optional_template_arguments(false).unwrap().is_none());
    assert_eq!(parser.angles.cached_queries().0, failures);
    parser.conflicts.take = Some(Category::Type);
    parser.conflicts.active = false;
    assert!(parser.optional_template_arguments(false).unwrap().is_none());
    assert_eq!(parser.cursor.position, position);
    parser.conflicts.active = true;
    let arguments = parser.optional_template_arguments(false).unwrap().unwrap();
    assert!(matches!(
        parser.tree.template_arguments.get(arguments)[0].kind,
        super::ast::TemplateArgumentKind::Type(_)
    ));
    parser.cursor.release();
}

#[test]
fn failed_member_template_lists_do_not_suppress_ordinary_lists() {
    let source = SourceBuffer::new(b"<int> value".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"template-call-key").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.cursor.retain();
    let position = parser.cursor.position;
    assert!(parser.optional_template_arguments(true).unwrap().is_none());
    assert_eq!(parser.cursor.position, position);
    assert!(parser.optional_template_arguments(false).unwrap().is_some());
    parser.cursor.release();
}

#[test]
fn committed_statements_bound_angle_memo_storage_but_attempts_retain_queries() {
    let source = SourceBuffer::new(format!("{{{}}}", "f<X>();".repeat(500)).into_bytes()).unwrap();
    let input = Preprocessor::with_name(&source, b"statement-angle-lifetime").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.compound_statement().unwrap();
    let (_, fences) = parser.angles.cached_queries();
    assert!(fences > 0 && fences < 4);
    let source = SourceBuffer::new(format!("{{{}}}", "f<X>();".repeat(20)).into_bytes()).unwrap();
    let input = Preprocessor::with_name(&source, b"tentative-angle-lifetime").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let result: Option<()> = parser
        .attempt(|parser| {
            parser.compound_statement()?;
            assert!(parser.angles.cached_queries().1 >= 20);
            parser.fail()
        })
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn replay_exhaustion_uses_synthetic_eof_and_restores_the_physical_cursor() {
    let source = SourceBuffer::new(b"int x;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"replay-end-location").unwrap();
    let mut cursor = Cursor::new(PostTokenizer::from_input(input, NumberDialect::Cxx11));
    cursor.retain();
    while !matches!(cursor.peek(0).unwrap().kind, TokenKind::End) {
        cursor.advance().unwrap();
    }
    let physical = cursor.peek(0).unwrap();
    assert!(physical.location.0 > 0);
    assert_eq!(cursor.peek(1).unwrap().location.0, 0);
    assert_eq!(cursor.peek(0).unwrap().location, physical.location);
    let position = cursor.absolute_position().unwrap();
    let run = cursor.capture(0, 0).unwrap();
    cursor.push_run(run);
    while !matches!(cursor.peek(0).unwrap().kind, TokenKind::End) {
        cursor.advance().unwrap();
    }
    for ahead in [0, 1, 8] {
        let end = cursor.peek(ahead).unwrap();
        assert!(matches!(end.kind, TokenKind::End));
        assert_eq!(end.location.0, 0);
    }
    cursor.pop_run().unwrap();
    assert_eq!(cursor.absolute_position().unwrap(), position);
    assert_eq!(cursor.peek(0).unwrap().location, physical.location);
    cursor.release();
}

#[test]
fn failed_braced_list_restores_angle_mode_without_an_enclosing_transaction() {
    let source = SourceBuffer::new(b"{1, +}".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"braced-angle-unwind").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.angle = true;
    assert!(matches!(
        parser.braced_list(),
        Err(ParseError::Syntax { .. })
    ));
    assert!(parser.angle);
    assert_eq!(parser.active_nesting(), 0);
    assert_eq!(parser.checkpoints, 0);
}

#[test]
fn recorded_token_ordinals_keep_the_low_target_bits_after_host_conversion() {
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"source-ordinal-width").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    for ordinal in [0, u64::from(u32::MAX), u64::from(u32::MAX) + 8] {
        if let Ok(position) = usize::try_from(ordinal) {
            parser.cursor.position = position;
            let expected = u32::from_le_bytes(ordinal.to_le_bytes()[..4].try_into().unwrap());
            assert_eq!(parser.token_ordinal().unwrap(), expected);
        }
    }
}

#[test]
fn literal_parentheses_do_not_start_tentative_type_readings() {
    for text in ["(42)", "sizeof(42)", "typeid(42)", "sizeof(((42)))"] {
        let source = SourceBuffer::new(text.as_bytes().to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"literal-type-demand").unwrap();
        let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
            input,
            NumberDialect::Cxx11,
        )));
        parser.expression().unwrap();
        assert_eq!(parser.checkpoints, 0);
        assert_eq!(parser.rewinds, 0);
        assert!(matches!(parser.token().unwrap().kind, TokenKind::End));
    }
}

#[test]
fn enum_forward_declarations_preserve_existing_type_member_scope() {
    let source = SourceBuffer::new(b"enum E {value}; enum E; enum E:int;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"enum-forward-owner").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let name = parser.cursor.intern_name(b"E").unwrap();
    let binding = parser.scopes.lookup(name).unwrap();
    assert!(binding.scope.is_some());
    let environment = parser.scopes.environment();
    for _ in 0..2 {
        parser.declaration().unwrap();
        let current = parser.scopes.lookup(name).unwrap();
        assert_eq!(current.category, binding.category);
        assert_eq!(current.scope, binding.scope);
        assert_eq!(parser.scopes.environment(), environment);
    }
}

#[test]
fn template_enclosed_enum_names_and_unscoped_enumerators_use_declaration_home() {
    for key in ["enum", "enum class", "enum struct"] {
        let source =
            SourceBuffer::new(format!("template<class T> {key} word {{member}};").into_bytes())
                .unwrap();
        let input = Preprocessor::with_name(&source, b"enum-template-owner").unwrap();
        let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
            input,
            NumberDialect::Cxx11,
        )));
        parser.declaration().unwrap();
        let name = parser.cursor.intern_name(b"word").unwrap();
        let member = parser.cursor.intern_name(b"member").unwrap();
        let binding = parser
            .scopes
            .own(super::scopes::Scopes::global(), name)
            .unwrap();
        assert_eq!(binding.category, Category::Type);
        let scope = binding.scope.unwrap();
        assert_eq!(
            parser.scopes.own(scope, member).unwrap().category,
            Category::Value
        );
        assert_eq!(
            parser
                .scopes
                .own(super::scopes::Scopes::global(), member)
                .is_some(),
            key == "enum"
        );
    }
}

#[test]
fn enum_template_id_does_not_overwrite_the_primary_template_binding() {
    let source =
        SourceBuffer::new(b"template<class T> struct P {}; enum P<int>{value};".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"enum-template-id-owner").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let name = parser.cursor.intern_name(b"P").unwrap();
    let binding = parser.scopes.lookup(name).unwrap();
    parser.declaration().unwrap();
    let current = parser.scopes.lookup(name).unwrap();
    assert_eq!(current.category, Category::TypeTemplate);
    assert_eq!(current.scope, binding.scope);
}

#[test]
fn known_keyword_dispatch_and_namespace_reopening_do_not_speculate_or_republish() {
    let source = SourceBuffer::new(b"namespace N{} namespace N{}".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"namespace-publication").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let environment = parser.scopes.environment();
    let checkpoints = parser.checkpoints;
    parser.declaration().unwrap();
    assert_eq!(parser.scopes.environment(), environment);
    assert_eq!(parser.checkpoints, checkpoints);
    assert_eq!(checkpoints, 0);
}

#[test]
fn predefined_extended_type_groups_keep_sign_and_complex_continuations() {
    let source =
        SourceBuffer::new(b"unsigned __int128 *a; _Complex __float128 b;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"extended-specifiers").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let roots = tree.decl_lists.get(unit.declarations.unwrap());
    for declaration in roots {
        let DeclKind::Simple { specifiers, .. } = tree.declarations.get(*declaration).kind else {
            panic!("simple declaration")
        };
        let specs = tree.specs.get(specifiers);
        assert_eq!(specs.len(), 2);
        let SpecifierKind::Named(name) = specs[1].kind else {
            panic!("named extended type")
        };
        assert!(
            tree.names
                .get(name)
                .binding
                .is_some_and(|binding| binding.category == Category::Type)
        );
    }
}

#[test]
fn attributed_empty_declarations_keep_the_unconsumed_attribute_start() {
    let bytes = b"[[unknown]]; __attribute__((used));";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"attributed-empty").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let roots = tree.decl_lists.get(unit.declarations.unwrap());
    assert_eq!(roots.len(), 2);
    assert_eq!(
        tree.declarations.get(roots[0]).location,
        source_offset(bytes, b"[[")
    );
    assert_eq!(
        tree.declarations.get(roots[1]).location,
        source_offset(bytes, b"__attribute__")
    );
    assert!(
        roots
            .iter()
            .all(|id| matches!(tree.declarations.get(*id).kind, DeclKind::Empty))
    );
}

#[test]
fn namespace_alias_lookahead_and_type_mode_reject_without_publishing_a_binding() {
    for bytes in [
        b"namespace [[gnu::abi_tag(\"v\")]] Alias=N;".as_slice(),
        b"namespace Alias=typename C;",
        b"namespace Alias=::operator+;",
    ] {
        let source = SourceBuffer::new(bytes.to_vec()).unwrap();
        let input = Preprocessor::with_name(&source, b"alias-boundary").unwrap();
        let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
            input,
            NumberDialect::Cxx11,
        )));
        let alias = parser.cursor.intern_name(b"Alias").unwrap();
        let outcome = parser.attempt(Parser::declaration).unwrap();
        assert!(outcome.is_none());
        assert_eq!(parser.cursor.position, 0);
        assert!(
            parser
                .scopes
                .own(super::scopes::Scopes::global(), alias)
                .is_none()
        );
        assert_eq!(parser.tree.declarations.mark(), 0);
    }
}

#[test]
fn syntax_keyword_overlays_leave_the_shared_phase_seven_tool_unchanged() {
    use crate::lex::{PostTokenKind, vocabulary::Keyword};
    let source = SourceBuffer::new(b"__inline __restrict__ __null".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"keyword-overlay").unwrap();
    let mut post = PostTokenizer::from_input(input, NumberDialect::Cxx11);
    for _ in 0..3 {
        assert!(matches!(
            post.next_token().unwrap().kind,
            PostTokenKind::Identifier
        ));
    }
    let input = Preprocessor::with_name(&source, b"keyword-overlay").unwrap();
    let mut cursor = Cursor::new(PostTokenizer::from_input(input, NumberDialect::Cxx11));
    let first = cursor.peek(0).unwrap();
    assert!(
        matches!(first.kind, TokenKind::Simple(op) if op.token.as_keyword()==Some(Keyword::Inline))
    );
    let second = cursor.peek(1).unwrap();
    assert!(matches!(second.kind, TokenKind::Literal(_)));
    assert_eq!(
        second.location,
        source_offset(b"__inline __restrict__ __null", b"__null")
    );
    assert!(matches!(cursor.peek(2).unwrap().kind, TokenKind::End));
    assert_eq!(cursor.tokens_read, 4);
}

#[test]
fn synthetic_null_literals_share_owned_spelling_and_typed_target_data() {
    use crate::lex::{LiteralForm, PostTokenKind};
    use crate::types::FundamentalType;
    let source = SourceBuffer::new(b"__null __null".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"null-literal").unwrap();
    let mut cursor = Cursor::new(PostTokenizer::from_input(input, NumberDialect::Cxx11));
    let TokenKind::Literal(first) = cursor.peek(0).unwrap().kind else {
        panic!("literal")
    };
    let TokenKind::Literal(second) = cursor.peek(1).unwrap().kind else {
        panic!("literal")
    };
    assert!(
        matches!(first.kind, PostTokenKind::Literal {form:LiteralForm::Integer,ty:FundamentalType::LongInt,data} if data.as_slice()==[0;8])
    );
    assert_eq!(cursor.spellings.get(first.spelling), b"0L");
    assert_eq!(cursor.spellings.get(first.data), [0; 8]);
    assert!(std::ptr::eq(
        cursor.spellings.get(first.spelling).as_ptr(),
        cursor.spellings.get(second.spelling).as_ptr()
    ));
    assert!(std::ptr::eq(
        cursor.spellings.get(first.data).as_ptr(),
        cursor.spellings.get(second.data).as_ptr()
    ));
}

#[test]
fn stack_budget_failures_are_terminal_and_transactions_restore_typed_state() {
    use crate::support::stack::{StackBudget, StackLimit};
    let source = SourceBuffer::new(b"int x;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"stack-transaction").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let start = parser.location().unwrap();
    parser.stack_base = Some(0);
    parser.stack_budget = StackBudget::from_limit(StackLimit::Bytes(0)).unwrap();
    let failure = parser.attempt(Parser::declaration).unwrap_err();
    assert!(
        matches!(failure, ParseError::CommittedSyntax {location, message:"constructs nested too deeply"} if location==start)
    );
    assert_eq!(parser.active_nesting(), 0);
    assert_eq!(parser.speculation, 0);
    assert_eq!(parser.cursor.position, 0);
    assert_eq!(parser.tree.declarations.mark(), 0);
    assert_eq!(parser.transaction_storage().0, 0);
    parser.stack_base = None;
    parser.stack_budget = StackBudget::default();
    parser.declarations(false).unwrap();
    assert_eq!(parser.active_nesting(), 0);
}

#[test]
fn native_byte_budget_unwinds_before_exhausting_the_callers_stack() {
    use crate::support::stack::{StackBudget, StackLimit};
    let outcome = std::thread::Builder::new()
        .stack_size(8 << 20)
        .spawn(|| {
            let mut bytes = b"int x=".to_vec();
            bytes.extend(std::iter::repeat_n(b'(', 2000));
            bytes.push(b'1');
            bytes.extend(std::iter::repeat_n(b')', 2000));
            bytes.push(b';');
            let source = SourceBuffer::new(bytes).unwrap();
            let input = Preprocessor::with_name(&source, b"physical-stack-budget").unwrap();
            let mut unit = SyntaxUnit::new(input);
            unit.set_stack_budget(StackBudget::from_limit(StackLimit::Bytes(128 << 10)).unwrap());
            let error = unit.parse().unwrap_err();
            assert!(matches!(
                error,
                ParseError::CommittedSyntax {
                    message: "constructs nested too deeply",
                    ..
                }
            ));
            assert_eq!(unit.parser.active_nesting(), 0);
            assert_eq!(unit.parser.transaction_storage().0, 0);
            assert!(unit.parser.peak_nesting > 0);
        })
        .unwrap();
    outcome.join().unwrap();
}

#[test]
fn nested_parser_transactions_reuse_storage_and_release_all_marks() {
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"transaction-storage-lifetime").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let mut capacity = None;
    for _ in 0..1000 {
        let outer: Option<()> = parser
            .attempt(|parser| {
                assert_eq!(parser.transaction_storage().0, 1);
                let inner = parser.attempt(|parser| {
                    assert_eq!(parser.transaction_storage().0, 2);
                    parser.angle = true;
                    Ok(())
                })?;
                assert!(inner.is_some());
                assert!(parser.angle);
                assert_eq!(parser.transaction_storage().0, 1);
                parser.fail()
            })
            .unwrap();
        assert!(outer.is_none());
        assert!(!parser.angle);
        let storage = parser.transaction_storage();
        assert_eq!(storage.0, 0);
        assert_eq!(*capacity.get_or_insert(storage.1), storage.1);
    }
}

#[test]
fn source_facts_preserve_specifier_initializer_and_declarator_boundaries() {
    use super::ast::{InitializerKind, SpecifierKind};
    let bytes = b"static const unsigned long left = 1, *right(2);";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"source-facts").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let DeclKind::Simple {
        specifiers,
        declarators,
    } = tree
        .declarations
        .get(tree.decl_lists.get(unit.declarations.unwrap())[0])
        .kind
    else {
        panic!("simple declaration")
    };
    for (specifier, spelling) in tree.specs.get(specifiers).iter().zip([
        b"static".as_slice(),
        b"const",
        b"unsigned",
        b"long",
    ]) {
        assert_eq!(specifier.location, source_offset(bytes, spelling));
        assert!(matches!(specifier.kind, SpecifierKind::Keyword(_)));
    }
    let declarations = tree.init_declarators.get(declarators);
    assert_eq!(
        (declarations[0].tokens.begin, declarations[0].tokens.end),
        (4, 7)
    );
    assert_eq!(
        (declarations[1].tokens.begin, declarations[1].tokens.end),
        (8, 13)
    );
    let first = declarations[0].initializer.unwrap();
    assert_eq!(first.location, source_offset(bytes, b"="));
    assert!(matches!(first.kind, InitializerKind::Assign(_)));
    let second = declarations[1].initializer.unwrap();
    assert_eq!(second.location, source_offset(bytes, b"("));
    assert!(matches!(second.kind, InitializerKind::Parenthesized(_)));
}

#[test]
fn parameter_qualifier_name_enum_and_constructor_locations_remain_typed() {
    use super::ast::{Body, InitializerKind, QualifierKind, SpecifierKind, Suffix};
    let bytes = b"struct C{auto f(int value=1) const & noexcept(value) -> int; inline C():member(2) {} int member;}; [[gnu::used]] enum E{red,blue=3};";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"located-syntax").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let roots = tree.decl_lists.get(unit.declarations.unwrap());
    let DeclKind::Class(specifier) = tree.declarations.get(roots[0]).kind else {
        panic!("class declaration")
    };
    let SpecifierKind::Class(class) = specifier.kind else {
        panic!("class specifier")
    };
    let members = tree.decl_lists.get(tree.classes.get(class).members);
    let DeclKind::Simple { declarators, .. } = tree.declarations.get(members[0]).kind else {
        panic!("method declaration")
    };
    let method = tree
        .declarators
        .get(tree.init_declarators.get(declarators)[0].declarator);
    let Suffix::Function {
        parameters,
        qualifiers,
    } = tree.suffixes.get(method.suffixes)[0]
    else {
        panic!("function suffix")
    };
    assert_eq!(parameters.location, source_offset(bytes, b"(int"));
    let parameter = tree.parameters.get(parameters.parameters)[0];
    assert_eq!(parameter.location, source_offset(bytes, b"int value"));
    let default = parameter.default.unwrap();
    assert_eq!(default.location, source_offset(bytes, b"1)"));
    assert!(matches!(default.kind, InitializerKind::Assign(_)));
    for (qualifier, spelling) in
        tree.qualifiers
            .get(qualifiers)
            .iter()
            .zip([b"const".as_slice(), b"&", b"noexcept", b"->"])
    {
        assert_eq!(qualifier.location, source_offset(bytes, spelling));
    }
    assert!(matches!(
        tree.qualifiers.get(qualifiers)[2].kind,
        QualifierKind::Noexcept(Some(_))
    ));
    let QualifierKind::Trailing(ty) = tree.qualifiers.get(qualifiers)[3].kind else {
        panic!("trailing type")
    };
    assert_eq!(
        tree.types.get(ty).location.0,
        source_offset(bytes, b"-> int").0 + 3
    );
    let DeclKind::SpecialMember {
        name,
        declarator,
        body: Some(body),
        ..
    } = tree.declarations.get(members[1]).kind
    else {
        panic!("constructor")
    };
    assert_eq!(tree.names.get(name).location, source_offset(bytes, b"C():"));
    assert_eq!(
        tree.declarators.get(declarator).location,
        tree.names.get(name).location
    );
    let Body::Parsed(body) = *tree.bodies.get(body) else {
        panic!("parsed body")
    };
    assert_eq!(
        body.initializer_location,
        Some(source_offset(bytes, b":member"))
    );
    assert_eq!(
        tree.declarations.get(roots[1]).location,
        source_offset(bytes, b"enum")
    );
    let DeclKind::Enum { enumerators, .. } = tree.declarations.get(roots[1]).kind else {
        panic!("enum")
    };
    assert_eq!(
        tree.enumerators.get(enumerators)[0].location,
        source_offset(bytes, b"red")
    );
    assert_eq!(
        tree.enumerators.get(enumerators)[1].location,
        source_offset(bytes, b"blue")
    );
}

#[test]
fn class_packing_and_token_spans_survive_lookahead_and_deferred_replay() {
    use super::ast::{Body, StmtKind, TokenSpan};
    let bytes = b"#pragma pack(push,1)\nstruct A {void f(){struct Local{};}} __attribute__((packed));\n#pragma pack(push,4)\nstruct B {};\n#pragma pack(pop)\nstruct C {};\n#pragma pack(pop)\nstruct D {};";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"packing-facts").unwrap();
    let mut unit = SyntaxUnit::new(input);
    let mut tokens = Vec::new();
    for ahead in 0.. {
        let token = unit.parser.cursor.peek(ahead).unwrap();
        tokens.push(token);
        if matches!(token.kind, TokenKind::End) {
            break;
        }
    }
    assert_eq!(unit.parser.cursor.post.input().pack_alignment(), 0);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let roots = tree.decl_lists.get(unit.declarations.unwrap());
    for (index, (name, alignment, end_spelling)) in [
        (b"A".as_slice(), 1, b"} __".as_slice()),
        (b"B", 4, b"};\n#pragma pack(pop)\nstruct C"),
        (b"C", 1, b"};\n#pragma pack(pop)\nstruct D"),
        (b"D", 0, b"};"),
    ]
    .into_iter()
    .enumerate()
    {
        let class = class_syntax(tree, roots[index]);
        assert_eq!(class.packing, alignment);
        assert!(!class.has_base_clause);
        let TokenSpan { begin, end } = class.tokens;
        assert_eq!(
            tokens[usize::try_from(begin).unwrap()].location,
            class.location
        );
        let closing = if name == b"D" {
            bytes.len() - 2
        } else {
            source_offset(bytes, end_spelling).0 as usize
        };
        assert_eq!(
            tokens[usize::try_from(end - 1).unwrap()].location.0 as usize,
            closing
        );
    }
    let outer = class_syntax(tree, roots[0]);
    let DeclKind::Function { body, .. } = tree
        .declarations
        .get(tree.decl_lists.get(outer.members)[0])
        .kind
    else {
        panic!("member function")
    };
    let Body::Parsed(body) = *tree.bodies.get(body) else {
        panic!("parsed member")
    };
    assert_eq!(
        tree.statements.get(body.compound).location,
        source_offset(bytes, b"{struct Local")
    );
    let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
        panic!("compound")
    };
    let StmtKind::Declaration(local) = tree.statements.get(tree.stmt_lists.get(items)[0]).kind
    else {
        panic!("local declaration")
    };
    let local = class_syntax(tree, local);
    assert_eq!(local.packing, 1);
    assert_eq!(local.location, source_offset(bytes, b"struct Local"));
    let root = tree.decl_lists.get(unit.declarations.unwrap())[0];
    assert!(matches!(
        tree.declarations.get(root).kind,
        DeclKind::Class(super::ast::Specifier {
            kind: SpecifierKind::Class(_),
            ..
        })
    ));
}

#[test]
fn macro_and_attribute_locations_preserve_shared_names_and_first_token_packing() {
    use super::ast::{SpecifierKind, TypeSyntax};
    let bytes = b"struct _Pragma(\"pack(2)\") M{}; struct N{}; typename N::word const";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"first-token-facts").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let first = parser.declaration().unwrap();
    let second = parser.declaration().unwrap();
    assert_eq!(class_syntax(&parser.tree, first).packing, 0);
    assert_eq!(class_syntax(&parser.tree, second).packing, 2);
    let start = parser.location().unwrap();
    let (name, _) = parser.name().unwrap();
    assert_eq!(
        parser.tree.names.get(name).location,
        source_offset(bytes, b"N::word")
    );
    let ty = parser.type_from_name(name, start).unwrap();
    let TypeSyntax {
        location,
        specifiers,
        ..
    } = parser.tree.types.get(ty);
    assert_eq!(*location, source_offset(bytes, b"typename"));
    let specs = parser.tree.specs.get(*specifiers);
    assert_eq!(specs[0].location, *location);
    assert!(matches!(specs[0].kind, SpecifierKind::Named(id) if id==name));
    assert_eq!(specs[1].location, source_offset(bytes, b"const"));
}

#[test]
fn abandoned_source_records_rewind_while_pack_observations_remain_live() {
    let bytes = b"#pragma pack(2)\nstruct C{int member=1;};\n#pragma pack(8)\nstruct D{};";
    let source = SourceBuffer::new(bytes.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"source-fact-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let result: Option<()> = parser
        .attempt(|parser| {
            let declaration = parser.declaration()?;
            assert_eq!(class_syntax(&parser.tree, declaration).packing, 2);
            parser.fail()
        })
        .unwrap();
    assert!(result.is_none());
    assert_eq!(parser.tree.classes.mark(), 0);
    assert_eq!(
        parser.tree.specs.mark(),
        super::storage::Lists::<super::ast::Specifier>::default().mark()
    );
    let declarations = parser.declarations(false).unwrap();
    let roots = parser.tree.decl_lists.get(declarations);
    assert_eq!(class_syntax(&parser.tree, roots[0]).packing, 2);
    assert_eq!(class_syntax(&parser.tree, roots[1]).packing, 8);
}

#[test]
fn conflicting_statements_retain_typed_readings_and_share_one_lambda_body() {
    use super::ast::{Body, ExprKind, LambdaBody, ReadingCategory, StmtKind};
    let source = SourceBuffer::new(
        b"template<class T> struct P{typedef int word;}; template<> struct P<int>{int word;}; void f(){P<int>::word ([](){return 1;}());}".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"conflicting-readings").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let parser = &unit.parser;
    let tree = &parser.tree;
    let declarations = tree.decl_lists.get(unit.declarations.unwrap());
    let DeclKind::Function { body, .. } = tree.declarations.get(declarations[2]).kind else {
        panic!("function");
    };
    let Body::Parsed(body) = tree.bodies.get(body) else {
        panic!("body");
    };
    let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
        panic!("compound");
    };
    let StmtKind::Ambiguous {
        name,
        component,
        readings,
    } = tree.statements.get(tree.stmt_lists.get(items)[0]).kind
    else {
        panic!("ambiguous");
    };
    assert_eq!(component, 1);
    assert_eq!(
        tree.name_lists.get(tree.names.get(name).components).len(),
        2
    );
    let readings = tree.statement_readings.get(readings);
    assert_eq!(readings.len(), 2);
    assert_eq!(readings[0].category, ReadingCategory::Type);
    assert_eq!(readings[1].category, ReadingCategory::Value);
    let mut bodies = Vec::new();
    for reading in readings {
        let StmtKind::Expression(Some(expr)) = tree.statements.get(reading.statement).kind else {
            panic!("expression");
        };
        let (ExprKind::FunctionalCast { arguments, .. } | ExprKind::Call { arguments, .. }) =
            tree.expressions.get(expr).kind
        else {
            panic!("call");
        };
        let ExprKind::Call { callee, .. } =
            tree.expressions.get(tree.expr_lists.get(arguments)[0]).kind
        else {
            panic!("lambda call");
        };
        let ExprKind::Lambda(lambda) = tree.expressions.get(callee).kind else {
            panic!("lambda");
        };
        let LambdaBody::Parsed(body) = tree.lambdas.get(lambda).body else {
            panic!("parsed lambda");
        };
        bodies.push(body);
    }
    assert_eq!(bodies[0], bodies[1]);
    assert_eq!(parser.deferred_lambdas, 1);
    assert_eq!(parser.shared_lambdas.mark(), 0);
    assert!(parser.shared_lambdas.region.is_none());
}

#[test]
fn specialization_categories_are_keyed_by_lexical_home_and_survive_only_as_hints_on_undo() {
    let source = SourceBuffer::new(
        b"namespace A{template<class T>struct P{typedef int word;};template<>struct P<int>{int word;};} namespace B{template<class T>struct P{typedef int word;};template<>struct P<int>{typedef int word;};}".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"category-homes").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let a = parser.cursor.intern_name(b"A").unwrap();
    let b = parser.cursor.intern_name(b"B").unwrap();
    let p = parser.cursor.intern_name(b"P").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    let result: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            parser.fail()
        })
        .unwrap();
    assert!(result.is_none());
    assert!(parser.scopes.lookup(a).is_none());
    assert!(parser.scopes.has_conflicts());
    parser.declarations(false).unwrap();
    let a = parser.scopes.lookup(a).unwrap().scope.unwrap();
    let b = parser.scopes.lookup(b).unwrap().scope.unwrap();
    let primary_a = parser
        .scopes
        .lookup_specialization(a, p, true, false)
        .unwrap()
        .scope
        .unwrap();
    let primary_b = parser
        .scopes
        .lookup_specialization(b, p, true, false)
        .unwrap()
        .scope
        .unwrap();
    let member_a = parser
        .scopes
        .lookup_specialization(primary_a, word, false, true)
        .unwrap();
    let member_b = parser
        .scopes
        .lookup_specialization(primary_b, word, false, true)
        .unwrap();
    assert!(member_a.alternatives.contains(Category::Value));
    assert!(member_b.alternatives.is_empty());
    assert!(parser.scopes.member_categories_differ(primary_a, word));
    assert!(!parser.scopes.member_categories_differ(primary_b, word));
}

#[test]
fn ambiguous_for_initializer_precedes_the_loop_in_its_owned_block() {
    use super::ast::{Body, StmtKind};
    let source = SourceBuffer::new(
        b"template<class T>struct P{typedef int word;}; template<>struct P<int>{int word;}; void f(){for(P<int>::word *p=0;p;++p){}}".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"for-reading-owner").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let declaration = tree.decl_lists.get(unit.declarations.unwrap())[2];
    let DeclKind::Function { body, .. } = tree.declarations.get(declaration).kind else {
        panic!("function");
    };
    let Body::Parsed(body) = tree.bodies.get(body) else {
        panic!("body");
    };
    let StmtKind::Compound(items) = tree.statements.get(body.compound).kind else {
        panic!("compound");
    };
    let first = tree.stmt_lists.get(items)[0];
    let StmtKind::Compound(items) = tree.statements.get(first).kind else {
        panic!("for-init block");
    };
    let items = tree.stmt_lists.get(items);
    assert_eq!(items.len(), 2);
    assert!(matches!(
        tree.statements.get(items[0]).kind,
        StmtKind::Ambiguous { .. }
    ));
    assert!(matches!(
        tree.statements.get(items[1]).kind,
        StmtKind::For {
            declaration: None,
            init: None,
            ..
        }
    ));
}

#[test]
fn abandoned_reading_regions_drop_shared_handles_before_syntax_reuse() {
    use super::conflicts::StatementReader;
    let source = SourceBuffer::new(
        b"template<class T>struct P{typedef int word;}; template<>struct P<int>{int word;}; P<int>::word ([](){return 1;}());".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"reading-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let statements = parser.tree.statements.mark();
    let readings = parser.tree.statement_readings.mark();
    let lambdas = parser.tree.lambdas.mark();
    let result: Option<()> = parser
        .attempt(|parser| {
            parser.speculation_boundary(|parser| {
                parser.read_statement(StatementReader::Ordinary, super::storage::List::default())
            })?;
            assert_eq!(parser.deferred_lambdas, 1);
            parser.fail()
        })
        .unwrap();
    assert!(result.is_none());
    assert_eq!(parser.tree.statements.mark(), statements);
    assert_eq!(parser.tree.statement_readings.mark(), readings);
    assert_eq!(parser.tree.lambdas.mark(), lambdas);
    assert_eq!(parser.shared_lambdas.mark(), 0);
    assert!(parser.shared_lambdas.region.is_none());
    parser
        .read_statement(StatementReader::Ordinary, super::storage::List::default())
        .unwrap();
    assert_eq!(parser.deferred_lambdas, 2);
}

#[test]
fn qualified_member_definitions_complete_the_specializations_forward_identity() {
    let source = SourceBuffer::new(
        b"template<class T>struct P; template<>struct P<int>{struct Q;}; struct P<int>::Q{typedef int word;};".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"specialized-forward").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let p = parser.cursor.intern_name(b"P").unwrap();
    let q = parser.cursor.intern_name(b"Q").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    let primary = parser.scopes.lookup(p).unwrap().scope.unwrap();
    let forward = parser
        .scopes
        .lookup_specialization(primary, q, true, true)
        .unwrap()
        .scope
        .unwrap();
    assert!(!parser.scopes.complete(forward));
    parser.declaration().unwrap();
    assert!(parser.scopes.complete(forward));
    assert_eq!(
        parser
            .scopes
            .lookup_specialization(forward, word, false, false)
            .unwrap()
            .category,
        Category::Type
    );
    assert_eq!(
        parser
            .scopes
            .lookup_specialization(primary, q, true, true)
            .unwrap()
            .scope,
        Some(forward)
    );
    parser.scopes.current = forward;
    assert!(parser.scopes.current_instantiation(primary));
}

#[test]
fn unnamed_parameter_packs_remain_distinct_from_varargs_despite_identical_dump_views() {
    use super::ast::{Suffix, TemplateArgumentKind};
    let source = SourceBuffer::new(
        b"template<class...T>struct Wrapper{}; template<class...T>void f(T...); template<class T>void g(T...); template<class...T>void h(T...,Wrapper<T...>...);".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"pack-facts").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    for (index, expected) in [(1, [true]), (2, [false])] {
        let declaration = tree.decl_lists.get(unit.declarations.unwrap())[index];
        let DeclKind::Template { declaration, .. } = tree.declarations.get(declaration).kind else {
            panic!("template");
        };
        let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
            panic!("declaration");
        };
        let declarator = tree.init_declarators.get(declarators)[0].declarator;
        let Suffix::Function { parameters, .. } =
            tree.suffixes.get(tree.declarators.get(declarator).suffixes)[0]
        else {
            panic!("function");
        };
        let parameter = tree.parameters.get(parameters.parameters)[0];
        assert_eq!(
            parameter
                .declarator
                .is_some_and(|declarator| tree.declarators.get(declarator).pack),
            expected[0]
        );
        assert_eq!(parameters.ellipsis, !expected[0]);
    }
    let declaration = tree.decl_lists.get(unit.declarations.unwrap())[3];
    let DeclKind::Template { declaration, .. } = tree.declarations.get(declaration).kind else {
        panic!("template");
    };
    let DeclKind::Simple { declarators, .. } = tree.declarations.get(declaration).kind else {
        panic!("declaration");
    };
    let declarator = tree.init_declarators.get(declarators)[0].declarator;
    let Suffix::Function { parameters, .. } =
        tree.suffixes.get(tree.declarators.get(declarator).suffixes)[0]
    else {
        panic!("function");
    };
    let items = tree.parameters.get(parameters.parameters);
    assert!(tree.declarators.get(items[0].declarator.unwrap()).pack);
    assert!(items[1].declarator.is_none());
    assert!(parameters.ellipsis);
    let super::ast::SpecifierKind::Named(name) = tree.specs.get(items[1].specifiers)[0].kind else {
        panic!("name");
    };
    let arguments = tree.name_lists.get(tree.names.get(name).components)[0]
        .arguments
        .unwrap();
    let TemplateArgumentKind::Type(ty) = tree.template_arguments.get(arguments)[0].kind else {
        panic!("type argument");
    };
    assert!(
        tree.declarators
            .get(tree.types.get(ty).declarator.unwrap())
            .pack
    );
}

#[test]
fn named_trait_and_cast_readings_share_names_and_do_not_consume_alias_records() {
    use super::ast::{ExprKind, Specifier};
    let source = SourceBuffer::new(
        b"struct C{typedef int word;}; template<class T> struct P:T{}; sizeof(P<P<C>>(1)), typeid(P<P<C>>(1)), (P<P<C>>(1));"
            .to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"shared-operands").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let names = parser.tree.names.mark();
    let expression = parser.expression().unwrap();
    assert_eq!(parser.tree.names.mark() - names, 9);
    assert!(parser.specialization_records.is_empty());
    let ExprKind::Binary { left, .. } = parser.tree.expressions.get(expression).kind else {
        panic!("expected comma");
    };
    let ExprKind::Binary { left, .. } = parser.tree.expressions.get(left).kind else {
        panic!("expected comma");
    };
    let ExprKind::Sizeof {
        operand: Some(operand),
        ty: None,
        parenthesized: true,
    } = parser.tree.expressions.get(left).kind
    else {
        panic!("expected expression operand");
    };
    let ExprKind::FunctionalCast { specifiers, .. } = parser.tree.expressions.get(operand).kind
    else {
        panic!("expected functional cast");
    };
    assert!(matches!(
        parser.tree.specs.get(specifiers),
        [Specifier {
            kind: SpecifierKind::Named(_),
            ..
        }]
    ));
}

#[test]
fn failed_type_reading_preserves_its_shared_name_and_rewinds_only_new_derivations() {
    let source =
        SourceBuffer::new(b"struct C{}; template<class T> struct P:T{}; P<P<C>> const *".to_vec())
            .unwrap();
    let input = Preprocessor::with_name(&source, b"shared-type-rewind").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let location = parser.location().unwrap();
    let (name, _) = parser.name().unwrap();
    let names = parser.tree.names.mark();
    let types = parser.tree.types.mark();
    let declarators = parser.tree.declarators.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            let ty = parser.type_from_name(name, location)?;
            assert_eq!(parser.tree.types.get(ty).specialization, 0);
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.tree.names.mark(), names);
    assert_eq!(parser.tree.types.mark(), types);
    assert_eq!(parser.tree.declarators.mark(), declarators);
    assert!(parser.specialization_records.is_empty());
    let ty = parser.type_from_name(name, location).unwrap();
    assert_eq!(parser.tree.names.mark(), names);
    assert!(parser.tree.types.get(ty).declarator.is_some());
}

#[test]
fn parameter_base_facts_and_alias_records_survive_commit_and_rewind_together() {
    use super::scopes::Scopes;
    let source = SourceBuffer::new(
        b"struct C {typedef int word;}; template<class T> struct P:T{}; using A=P<C>; typedef A B;"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"parameter-base-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let p = parser.cursor.intern_name(b"P").unwrap();
    let a = parser.cursor.intern_name(b"A").unwrap();
    let b = parser.cursor.intern_name(b"B").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            parser.declaration()?;
            parser.declaration()?;
            assert!(parser.scopes.lookup(a).unwrap().specialization != 0);
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert!(parser.scopes.lookup(p).is_none());
    assert!(parser.scopes.lookup(a).is_none());
    assert!(parser.scopes.lookup(b).is_none());
    assert!(parser.specialization_records.is_empty());
    assert!(parser.template_head.is_none());
    parser.declaration().unwrap();
    let scope = parser.scopes.lookup(p).unwrap().scope.unwrap();
    assert_eq!(parser.scopes.parameter_bases(scope).len(), 1);
    assert_eq!(parser.scopes.parameter_bases(scope)[0].index, 0);
    assert!(!parser.scopes.parameter_bases(scope)[0].pack);
    parser.declaration().unwrap();
    parser.declaration().unwrap();
    let record = parser.scopes.lookup(a).unwrap().specialization;
    assert_eq!(parser.scopes.lookup(b).unwrap().specialization, record);
    let arguments = parser.recorded_arguments(record).unwrap();
    assert_eq!(
        parser
            .parameter_base_lookup(scope, arguments, word, false, 0)
            .unwrap()
            .category,
        Category::Type
    );
    parser.scopes.current = scope;
    // Unqualified lookup in the template never substitutes its base.
    assert!(parser.scopes.lookup(word).is_none());
    parser.scopes.current = Scopes::global();
    let mut later = Vec::new();
    parser.scopes.detach(0, &mut later);
    assert!(parser.scopes.parameter_bases(scope).is_empty());
    parser.scopes.reattach(&later);
    assert_eq!(parser.scopes.parameter_bases(scope).len(), 1);
    assert!(parser.scopes.lookup(b).is_some());
}

#[test]
fn parameter_base_order_packs_and_record_capacity_are_explicit() {
    let mut bytes = b"struct C {typedef int word;}; struct D {int word;}; template<class... T> struct P:T...{};".to_vec();
    for index in 0..256 {
        bytes.extend_from_slice(format!("using A{index}=P<C>;").as_bytes());
    }
    let source = SourceBuffer::new(bytes).unwrap();
    let input = Preprocessor::with_name(&source, b"parameter-base-records").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declarations(false).unwrap();
    assert_eq!(parser.specialization_records.len(), 255);
    let a254 = parser.cursor.intern_name(b"A254").unwrap();
    let a255 = parser.cursor.intern_name(b"A255").unwrap();
    assert_eq!(parser.scopes.lookup(a254).unwrap().specialization, 255);
    assert_eq!(parser.scopes.lookup(a255).unwrap().specialization, 0);
    let p = parser.cursor.intern_name(b"P").unwrap();
    let scope = parser.scopes.lookup(p).unwrap().scope.unwrap();
    assert!(parser.scopes.parameter_bases(scope)[0].pack);
}

#[test]
fn rolled_back_import_can_be_inserted_again_without_leaking_lookup_state() {
    let source =
        SourceBuffer::new(b"namespace N { typedef int word; } using namespace N;".to_vec())
            .unwrap();
    let input = Preprocessor::with_name(&source, b"import-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    assert!(parser.scopes.lookup(word).is_none());
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert!(parser.scopes.lookup(word).is_none());
    parser.declaration().unwrap();
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
    let predefined = parser.cursor.intern_name(b"nullptr_t").unwrap();
    assert_eq!(
        parser.scopes.lookup(predefined).unwrap().category,
        Category::Type
    );
}

#[test]
fn forward_scope_parent_is_restored_when_its_definition_is_abandoned() {
    let source = SourceBuffer::new(
        b"struct C {typedef int word; struct X * p;}; struct X {void f(){word * p;}};".to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"forward-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let x = parser.cursor.intern_name(b"X").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    let global = parser.scopes.current;
    let scope = parser.scopes.lookup(x).unwrap().scope.unwrap();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert!(!parser.scopes.complete(scope));
    parser.scopes.current = scope;
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
    parser.scopes.current = global;
    parser.declaration().unwrap();
    parser.scopes.current = scope;
    assert!(parser.scopes.lookup(word).is_none());
}

#[test]
fn deep_category_search_uses_reusable_work_storage_and_stops_at_cycles() {
    use super::scopes::{Binding, ScopeKind, Scopes};
    let source = SourceBuffer::new(Vec::new()).unwrap();
    let input = Preprocessor::with_name(&source, b"lookup-depth").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let word = parser.cursor.intern_name(b"word").unwrap();
    let missing = parser.cursor.intern_name(b"missing").unwrap();
    let first = parser.scopes.push_kind(ScopeKind::AnonymousClass).unwrap();
    parser.scopes.declare(
        word,
        Binding {
            pack: false,
            alternatives: super::scopes::Categories::default(),
            guesses: super::scopes::Guesses::default(),
            specialization: 0,
            parameter: false,
            category: Category::Type,
            scope: None,
        },
    );
    let mut previous = first;
    for _ in 0..10_000 {
        parser.scopes.current = Scopes::global();
        let scope = parser.scopes.push_kind(ScopeKind::AnonymousClass).unwrap();
        parser.scopes.add_edge(previous);
        previous = scope;
    }
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
    parser.scopes.declare(
        missing,
        Binding {
            pack: false,
            alternatives: super::scopes::Categories::default(),
            guesses: super::scopes::Guesses::default(),
            specialization: 0,
            parameter: false,
            category: Category::Value,
            scope: None,
        },
    );
    parser.scopes.current = first;
    parser.scopes.add_edge(previous);
    assert_eq!(
        parser.scopes.lookup(missing).unwrap().category,
        Category::Value
    );
    assert_eq!(parser.scopes.lookup(word).unwrap().category, Category::Type);
}

#[test]
fn primary_casts_retain_type_facts_even_when_dump_labels_are_calls() {
    use super::ast::{Body, ExprKind, Specifier, StmtKind};
    let source = SourceBuffer::new(
        b"struct C {}; int value; void f(){ ::C(); typename C::word(); decltype(value)(1); Y(); }"
            .to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"cast-facts").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    let tree = &unit.parser.tree;
    let declarations = tree.decl_lists.get(unit.declarations.unwrap());
    let DeclKind::Function { body, .. } = tree.declarations.get(declarations[2]).kind else {
        panic!("function");
    };
    let Body::Parsed(body) = tree.bodies.get(body) else {
        panic!("parsed body");
    };
    let StmtKind::Compound(statements) = tree.statements.get(body.compound).kind else {
        panic!("compound");
    };
    let statements = tree.stmt_lists.get(statements);
    for (index, statement) in statements.iter().enumerate() {
        let StmtKind::Expression(Some(expression)) = tree.statements.get(*statement).kind else {
            panic!("expression statement");
        };
        let expression = tree.expressions.get(expression);
        if index == 3 {
            assert!(matches!(expression.kind, ExprKind::Call { .. }));
        } else {
            let ExprKind::FunctionalCast { specifiers, .. } = expression.kind else {
                panic!("typed cast");
            };
            if index == 2 {
                assert!(matches!(
                    tree.specs.get(specifiers),
                    [Specifier {
                        kind: SpecifierKind::Decltype(_),
                        ..
                    }]
                ));
            } else {
                assert!(matches!(
                    tree.specs.get(specifiers),
                    [Specifier {
                        kind: SpecifierKind::Named(_),
                        ..
                    }]
                ));
            }
        }
    }
}

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
            initializer: Some(Initializer {
                kind: InitializerKind::Parenthesized(_),
                ..
            }),
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
    let DeclKind::Class(Specifier {
        kind: SpecifierKind::Class(outer),
        ..
    }) = tree.declarations.get(root[1]).kind
    else {
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

#[test]
fn template_transaction_rewinds_heads_arguments_environment_and_deferred_bodies() {
    use super::ast::{Specifier, TemplateParameterKind};
    let source = SourceBuffer::new(
        b"template<class T, int N=1> struct C { T f(){return T(N);} }; C<int,2> value;".to_vec(),
    )
    .unwrap();
    let input = Preprocessor::with_name(&source, b"template-transaction").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.token().unwrap();
    let environment = parser.scopes.environment();
    let parameters = parser.tree.template_parameters.mark();
    let arguments = parser.tree.template_arguments.mark();
    let heads = parser.tree.declarations.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            parser.declaration()?;
            assert!(parser.deferred.is_empty());
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.scopes.environment(), environment);
    assert_eq!(parser.tree.template_parameters.mark(), parameters);
    assert_eq!(parser.tree.template_arguments.mark(), arguments);
    assert_eq!(parser.tree.declarations.mark(), heads);
    assert!(!parser.pending_template);
    assert!(!parser.angle);
    let c = parser.cursor.intern_name(b"C").unwrap();
    assert!(parser.scopes.lookup(c).is_none());
    let declaration = parser.declaration().unwrap();
    let DeclKind::Template {
        parameters,
        declaration,
    } = parser.tree.declarations.get(declaration).kind
    else {
        panic!("template head");
    };
    let parameters = parser.tree.template_parameters.get(parameters.parameters);
    assert!(matches!(
        parameters[0].kind,
        TemplateParameterKind::Type { .. }
    ));
    assert!(matches!(
        parameters[1].kind,
        TemplateParameterKind::NonType { .. }
    ));
    assert!(matches!(
        parser.tree.declarations.get(declaration).kind,
        DeclKind::Class(Specifier {
            kind: SpecifierKind::Class(_),
            ..
        })
    ));
    assert_eq!(
        parser.scopes.lookup(c).unwrap().category,
        Category::TypeTemplate
    );
    let t = parser.cursor.intern_name(b"T").unwrap();
    assert!(parser.scopes.lookup(t).is_none());
    parser.declaration().unwrap();
}

#[test]
fn failed_angle_query_is_reused_only_in_its_name_environment() {
    use super::{ast::TemplateArgumentKind, scopes::Binding};
    let source = SourceBuffer::new(b"x<word*>()".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"angle-environment").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.token().unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    parser.scopes.declare(
        word,
        Binding {
            pack: false,
            alternatives: super::scopes::Categories::default(),
            guesses: super::scopes::Guesses::default(),
            specialization: 0,
            category: Category::Value,
            scope: None,
            parameter: false,
        },
    );
    let before = parser.scopes.environment();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            let (name, _) = parser.name()?;
            assert!(!parser.name_has_arguments(name));
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.scopes.environment(), before);
    parser.scopes.declare(
        word,
        Binding {
            pack: false,
            alternatives: super::scopes::Categories::default(),
            guesses: super::scopes::Guesses::default(),
            specialization: 0,
            category: Category::Type,
            scope: None,
            parameter: false,
        },
    );
    assert_ne!(parser.scopes.environment(), before);
    let (name, _) = parser.name().unwrap();
    let last = parser
        .tree
        .name_lists
        .get(parser.tree.names.get(name).components)
        .last()
        .unwrap();
    let arguments = parser.tree.template_arguments.get(last.arguments.unwrap());
    let TemplateArgumentKind::Type(ty) = arguments[0].kind else {
        panic!("typed argument");
    };
    let declarator = parser.tree.types.get(ty).declarator.unwrap();
    assert_eq!(
        parser
            .tree
            .pointers
            .get(parser.tree.declarators.get(declarator).pointers)
            .len(),
        1
    );
}

#[test]
fn relational_angle_fences_bound_speculation_on_long_undeclared_chains() {
    let mut text = b"void f(){a0".to_vec();
    for index in 1..=4096 {
        text.extend_from_slice(format!("<a{index}").as_bytes());
    }
    text.extend_from_slice(b";}");
    let source = SourceBuffer::new(text).unwrap();
    let input = Preprocessor::with_name(&source, b"relational-fences").unwrap();
    let mut unit = SyntaxUnit::new(input);
    unit.parse().unwrap();
    assert!(unit.parser.checkpoints < 4096 * 4);
    let mut output = Vec::new();
    unit.print(&mut output).unwrap();
    assert!(output.ends_with(b"id-expression a4096\n"));
}

#[test]
fn attributes_retain_typed_operands_sites_and_owner_order() {
    use super::ast::{AttributeForm, AttributeKind, AttributeSite, Specifier};
    let source = SourceBuffer::new(br#"
        struct C {};
        [[gnu::abi_tag("lead")]] alignas(C*) int [[gnu::vector_size(16)]] values[3] [[gnu::aligned(32)]];
        struct [[gnu::abi_tag("class")]] D { [[noreturn]] void f(int [[gnu::aligned(8)]] a) [[gnu::noinline]] noexcept; };
        [[gnu::abi_tag("declaration")]] struct [[gnu::abi_tag("forward")]] F;
        [[gnu::noinline]] void g(){ if(int [[gnu::aligned(8)]] n=1){} }
    "#.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"attribute-owners").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    parser.declaration().unwrap();
    let declaration = parser.declaration().unwrap();
    let node = parser.tree.declarations.get(declaration);
    let attributes = parser.tree.attributes.get(node.attributes);
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].kind, AttributeKind::AbiTag);
    assert_eq!(attributes[0].form, AttributeForm::Standard);
    assert_eq!(attributes[1].kind, AttributeKind::Alignas);
    assert!(attributes[1].ty.is_some());
    assert!(attributes[1].expression.is_none());
    let DeclKind::Simple {
        specifiers,
        declarators,
    } = node.kind
    else {
        panic!("expected object");
    };
    assert!(matches!(
        parser.tree.specs.get(specifiers)[1].kind,
        SpecifierKind::Vector {
            extended: false,
            ..
        }
    ));
    let declarator = parser.tree.init_declarators.get(declarators)[0].declarator;
    let attributes = parser
        .tree
        .attributes
        .get(parser.tree.declarators.get(declarator).attributes);
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].site, AttributeSite::Type);
    assert!(attributes[0].expression.is_some());
    let declaration = parser.declaration().unwrap();
    let DeclKind::Class(Specifier {
        kind: SpecifierKind::Class(class),
        ..
    }) = parser.tree.declarations.get(declaration).kind
    else {
        panic!("expected class");
    };
    let class = parser.tree.classes.get(class);
    assert_eq!(parser.tree.attributes.get(class.attributes).len(), 1);
    let member = parser
        .tree
        .declarations
        .get(parser.tree.decl_lists.get(class.members)[0]);
    assert_eq!(parser.tree.attributes.get(member.attributes).len(), 1);
    assert_eq!(
        parser.tree.attributes.get(member.attributes)[0].kind,
        AttributeKind::NoReturn
    );
    let DeclKind::Simple { declarators, .. } = member.kind else {
        panic!("expected member declaration");
    };
    let declarator = parser.tree.init_declarators.get(declarators)[0].declarator;
    let attributes = parser
        .tree
        .attributes
        .get(parser.tree.declarators.get(declarator).attributes);
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].kind, AttributeKind::NoInline);
    assert_eq!(attributes[0].site, AttributeSite::Type);
    let declaration = parser.declaration().unwrap();
    let node = parser.tree.declarations.get(declaration);
    assert!(matches!(node.kind, DeclKind::Simple { .. }));
    let attributes = parser.tree.attributes.get(node.attributes);
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].site, AttributeSite::Class);
    assert_eq!(attributes[1].site, AttributeSite::Entity);
    let declaration = parser.declaration().unwrap();
    let node = parser.tree.declarations.get(declaration);
    assert_eq!(parser.tree.attributes.get(node.attributes).len(), 1);
    assert_eq!(
        parser.tree.attributes.get(node.attributes)[0].kind,
        AttributeKind::NoInline
    );
}

#[test]
fn failed_nested_attribute_declarations_restore_all_storage_and_scratch() {
    let source = SourceBuffer::new(br#"
        [[gnu::abi_tag("a", "b")]] struct [[gnu::packed]] C { alignas(int*) int x; void f(){ [[gnu::used]] int y; } };
        using word [[gnu::vector_size(16)]] = int;
    "#.to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"attribute-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let mark = parser.tree.attributes.mark();
    let expressions = parser.tree.expressions.mark();
    let names = parser.tree.names.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.declaration()?;
            assert!(parser.attempt(Parser::declaration)?.is_some());
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.tree.attributes.mark(), mark);
    assert_eq!(parser.tree.expressions.mark(), expressions);
    assert_eq!(parser.tree.names.mark(), names);
    assert_eq!(parser.tree.attributes.begin(), 0);
    assert!(parser.pending_attributes.is_none());
    assert!(parser.leading_attributes.is_empty());
    assert!(parser.deferred.is_empty());
    let c = parser.cursor.intern_name(b"C").unwrap();
    let word = parser.cursor.intern_name(b"word").unwrap();
    assert!(parser.scopes.lookup(c).is_none());
    assert!(parser.scopes.lookup(word).is_none());
    parser.declaration().unwrap();
    let declaration = parser.declaration().unwrap();
    assert_eq!(parser.tree.attributes.begin(), 0);
    assert!(parser.scopes.lookup(c).is_some());
    assert!(parser.scopes.lookup(word).is_some());
    let DeclKind::Alias { ty, .. } = parser.tree.declarations.get(declaration).kind else {
        panic!("expected alias");
    };
    assert!(matches!(
        parser.tree.specs.get(parser.tree.types.get(ty).specifiers)[1].kind,
        super::ast::SpecifierKind::Vector { .. }
    ));
}

#[test]
fn abandoned_elaborated_class_tags_preserve_enclosing_attribute_scratch() {
    use super::ast::AttributeKind;
    let source =
        SourceBuffer::new(b"[[gnu::used]] struct [[gnu::aligned(16)]] F;".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"class-tag-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let leading = parser.collect_attributes(false).unwrap();
    parser.tree.attributes.extend(leading);
    parser.pending_attributes = Some(0);
    let mark = parser.tree.attributes.mark();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.class_specifier(false, true)?;
            assert!(parser.pending_class_attributes());
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.tree.attributes.mark(), mark);
    assert!(!parser.pending_class_attributes());
    let retained = parser.tree.attributes.finish(0).unwrap();
    let attributes = parser.tree.attributes.get(retained);
    assert_eq!(attributes.len(), 1);
    assert_eq!(attributes[0].kind, AttributeKind::Used);
}

#[test]
fn lambda_facts_retain_capture_kinds_attributes_and_source_ordinals() {
    use super::ast::{AttributeKind, CaptureDefault, CaptureKind, ExprKind, LambdaBody, Qualifier};
    let source = SourceBuffer::new(
        b"[=, &x, args..., this](int x) __attribute__((noinline)) mutable noexcept(x) [[gnu::used]] -> int { return x; }".to_vec(),
    ).unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-facts").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let expression = parser.expression().unwrap();
    let ExprKind::Lambda(id) = parser.tree.expressions.get(expression).kind else {
        panic!("expected lambda");
    };
    let lambda = *parser.tree.lambdas.get(id);
    assert!(matches!(lambda.capture_default, Some(CaptureDefault::Copy)));
    let captures = parser.tree.captures.get(lambda.captures);
    assert_eq!(captures.len(), 3);
    assert!(matches!(captures[0].kind, CaptureKind::Reference(_)));
    assert!(matches!(captures[1].kind, CaptureKind::Copy(_)) && captures[1].pack);
    assert!(matches!(captures[2].kind, CaptureKind::This));
    assert_eq!(
        parser
            .tree
            .parameters
            .get(lambda.parameters.unwrap().parameters)
            .len(),
        1
    );
    assert!(lambda.is_mutable && lambda.trailing.is_some());
    assert!(matches!(
        lambda.exception,
        Some(Qualifier {
            kind: QualifierKind::Noexcept(Some(_)),
            ..
        })
    ));
    let attributes = parser.tree.attributes.get(lambda.attributes);
    assert_eq!(attributes.len(), 2);
    assert_eq!(attributes[0].kind, AttributeKind::NoInline);
    assert_eq!(attributes[1].kind, AttributeKind::Used);
    assert_eq!(lambda.clause_token, 11);
    assert!(lambda.body_token > lambda.clause_token);
    assert!(matches!(lambda.body, LambdaBody::Parsed(_)));
    assert!(parser.function_scope.is_none());
}

#[test]
fn abandoned_lambda_attempt_never_demands_a_body_and_restores_scratch() {
    use super::ast::{ExprKind, LambdaBody};
    let source = SourceBuffer::new(b"[&x](int C){ C * y; }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-rollback").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let lambdas = parser.tree.lambdas.mark();
    let captures = parser.tree.captures.mark();
    let statements = parser.tree.statements.mark();
    parser.token().unwrap();
    let scopes = parser.scopes.log_position();
    let outcome: Option<()> = parser
        .attempt(|parser| {
            let expression = parser.expression()?;
            let ExprKind::Lambda(id) = parser.tree.expressions.get(expression).kind else {
                panic!("expected lambda");
            };
            assert!(matches!(
                parser.tree.lambdas.get(id).body,
                LambdaBody::Deferred
            ));
            assert_eq!(parser.tree.statements.mark(), statements);
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert_eq!(parser.tree.lambdas.mark(), lambdas);
    assert_eq!(parser.tree.captures.mark(), captures);
    assert_eq!(parser.tree.statements.mark(), statements);
    assert_eq!(parser.scopes.log_position(), scopes);
    assert!(parser.lambdas.is_empty());
    assert_eq!(parser.deferred_lambdas, 0);
    parser.attempt(Parser::expression).unwrap().unwrap();
    assert_eq!(parser.deferred_lambdas, 1);
    assert!(parser.lambdas.is_empty());
}

#[test]
fn deferred_lambda_bodies_see_only_their_source_position_bindings() {
    use super::ast::{ExprKind, LambdaBody, StmtKind};
    use super::scopes::Binding;
    let source = SourceBuffer::new(b"[] { C * y; }, [] { C * y; }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-source-order").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let c = parser.cursor.intern_name(b"C").unwrap();
    let type_binding = Binding {
        pack: false,
        alternatives: super::scopes::Categories::default(),
        guesses: super::scopes::Guesses::default(),
        specialization: 0,
        category: Category::Type,
        parameter: false,
        scope: None,
    };
    parser.scopes.declare(c, type_binding);
    let (first, second) = parser
        .attempt(|parser| {
            let first = parser.assignment_expression()?;
            parser.expect(crate::lex::pp_lexer::Punctuator::Comma)?;
            parser.scopes.declare(
                c,
                Binding {
                    pack: false,
                    alternatives: super::scopes::Categories::default(),
                    guesses: super::scopes::Guesses::default(),
                    specialization: 0,
                    category: Category::Value,
                    ..type_binding
                },
            );
            let second = parser.assignment_expression()?;
            parser.scopes.declare(c, type_binding);
            Ok((first, second))
        })
        .unwrap()
        .unwrap();
    for (expression, declaration) in [(first, true), (second, false)] {
        let ExprKind::Lambda(id) = parser.tree.expressions.get(expression).kind else {
            panic!("expected lambda");
        };
        let LambdaBody::Parsed(body) = parser.tree.lambdas.get(id).body else {
            panic!("expected parsed body");
        };
        let StmtKind::Compound(items) = parser.tree.statements.get(body).kind else {
            panic!("expected compound");
        };
        let statement = parser.tree.stmt_lists.get(items)[0];
        assert_eq!(
            matches!(
                parser.tree.statements.get(statement).kind,
                StmtKind::Declaration(_)
            ),
            declaration
        );
    }
    assert_eq!(parser.scopes.lookup(c).unwrap().category, Category::Type);
    assert_eq!(parser.deferred_lambdas, 2);
    assert!(parser.lambdas.is_empty());
}

#[test]
fn committed_lambda_body_errors_cannot_be_swallowed_by_outer_attempts() {
    let source = SourceBuffer::new(b"[]{ return x + ; }".to_vec()).unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-committed-error").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let result = parser.attempt(Parser::expression);
    assert!(matches!(result, Err(ParseError::CommittedSyntax { .. })));
    assert_eq!(parser.deferred_lambdas, 1);
    assert!(parser.lambdas.is_empty());
    assert_eq!(parser.speculation, 0);
}

#[test]
fn skipped_lambda_bodies_use_iterative_delimiter_scratch_without_grammar_demand() {
    let nesting = 5000;
    let mut bytes = b"[]{".to_vec();
    bytes.extend(std::iter::repeat_n(b'(', nesting));
    bytes.extend_from_slice(b"0");
    bytes.extend(std::iter::repeat_n(b')', nesting));
    bytes.extend_from_slice(b";}");
    let source = SourceBuffer::new(bytes).unwrap();
    let input = Preprocessor::with_name(&source, b"lambda-balanced-demand").unwrap();
    let mut parser = Parser::new(Cursor::new(PostTokenizer::from_input(
        input,
        NumberDialect::Cxx11,
    )));
    let outcome: Option<()> = parser
        .attempt(|parser| {
            parser.expression()?;
            assert!(parser.peak_nesting < 10);
            assert_eq!(parser.tree.statements.mark(), 0);
            parser.fail()
        })
        .unwrap();
    assert!(outcome.is_none());
    assert!(parser.lambdas.is_empty());
    assert_eq!(parser.deferred_lambdas, 0);
}
