use super::*;

#[test]
fn expanded_spelling_length_wraps_in_the_targets_32_bit_space() {
    let maximum = u32::MAX as usize;
    assert_eq!(token_spelling_length(maximum), maximum);
    assert_eq!(token_spelling_length(maximum + 1), 0);
    assert_eq!(token_spelling_length(maximum + 7), 6);
    assert_eq!(token_spelling_length(maximum * 3), maximum - 2);
}

#[test]
fn source_views_and_translated_spans_keep_physical_identity() {
    let source = SourceBuffer::new(b"plain a\\\nb \\u00AA".to_vec()).unwrap();
    let mut lexer = PpLexer::new(&source).unwrap();
    let token = lexer.next_token().unwrap();
    assert_eq!(token.spelling.as_ptr(), source.bytes().as_ptr());
    assert_eq!((token.begin.0, token.end.0), (0, 5));
    assert_eq!(token.punctuator, None);
    lexer.next_token().unwrap();
    let token = lexer.next_token().unwrap();
    assert_eq!(token.spelling, b"ab");
    assert_eq!((token.begin.0, token.end.0), (6, 10));
    lexer.next_token().unwrap();
    let token = lexer.next_token().unwrap();
    assert_eq!(token.spelling, "ª".as_bytes());
    assert_eq!((token.begin.0, token.end.0), (11, 17));
}

#[test]
fn alternative_spellings_preserve_semantic_punctuator_identity() {
    for (left, right) in [
        ("[", "<:"),
        ("#", "%:"),
        ("##", "%:%:"),
        ("&&", "and"),
        ("!", "not"),
    ] {
        let a = SourceBuffer::new(left.as_bytes().to_vec()).unwrap();
        let b = SourceBuffer::new(right.as_bytes().to_vec()).unwrap();
        let mut first = PpLexer::new(&a).unwrap();
        let mut second = PpLexer::new(&b).unwrap();
        let x = first.next_token().unwrap();
        let y = second.next_token().unwrap();
        assert_eq!(x.kind, PpTokenKind::PreprocessingOpOrPunc);
        assert_eq!(x.punctuator, y.punctuator);
        assert_eq!(x.spelling, left.as_bytes());
        assert_eq!(y.spelling, right.as_bytes());
    }
}

#[test]
fn eof_is_idempotent_after_implicit_and_spliced_final_newlines() {
    for input in [b"".as_slice(), b"a", b"a\\\n", b"a??/\n", b"\xef\xbb\xbf"] {
        let source = SourceBuffer::new(input.to_vec()).unwrap();
        let mut lexer = PpLexer::new(&source).unwrap();
        while lexer.next_token().unwrap().kind != PpTokenKind::EndOfFile {}
        for _ in 0..3 {
            let token = lexer.next_token().unwrap();
            assert_eq!(token.kind, PpTokenKind::EndOfFile);
            assert!(token.spelling.is_empty());
            assert_eq!(token.begin, token.end);
            assert_eq!(token.end.index(), source.bytes().len());
        }
    }
}

#[test]
fn raw_strings_restore_physical_text_and_match_delimiters_by_character() {
    let input = "u8R\"ª(??/\n\\u00AA )ªx )ª\"_suffix";
    let source = SourceBuffer::new(input.as_bytes().to_vec()).unwrap();
    let mut lexer = PpLexer::new(&source).unwrap();
    let token = lexer.next_token().unwrap();
    assert_eq!(token.kind, PpTokenKind::UserDefinedStringLiteral);
    assert_eq!(token.spelling, input.as_bytes());
    assert_eq!(token.spelling.as_ptr(), source.bytes().as_ptr());
    let source = SourceBuffer::new(b"R\"\xaa(\x80)\xaa\"".to_vec()).unwrap();
    let mut lexer = PpLexer::new(&source).unwrap();
    assert_eq!(
        lexer.next_token().unwrap().spelling,
        "R\"ª(€)ª\"".as_bytes()
    );
}

#[test]
fn literal_ucns_preserve_structure_and_decode_extended_characters() {
    let source = SourceBuffer::new(b"\"\\u0022\\u000A\\u00AA\"".to_vec()).unwrap();
    let mut lexer = PpLexer::new(&source).unwrap();
    assert_eq!(
        lexer.next_token().unwrap().spelling,
        "\"\\u0022\\u000Aª\"".as_bytes()
    );
}

#[test]
fn dialect_options_preserve_token_text_and_explicit_header_demand() {
    let source = SourceBuffer::for_token_text(b"??/\n".to_vec()).unwrap();
    let options = PpLexerOptions {
        token_text: true,
        ..PpLexerOptions::default()
    };
    let mut lexer = PpLexer::with_options(&source, options).unwrap();
    assert_eq!(lexer.next_token().unwrap().spelling, b"?");
    assert_eq!(lexer.next_token().unwrap().spelling, b"?");
    assert_eq!(lexer.next_token().unwrap().spelling, b"/");
    let source = SourceBuffer::new(b" <a> <b>".to_vec()).unwrap();
    let options = PpLexerOptions {
        header_names_on_request: true,
        ..PpLexerOptions::default()
    };
    let mut lexer = PpLexer::with_options(&source, options).unwrap();
    lexer.request_header_name();
    assert_eq!(
        lexer.next_token().unwrap().kind,
        PpTokenKind::WhitespaceSequence
    );
    assert_eq!(lexer.next_token().unwrap().kind, PpTokenKind::HeaderName);
    lexer.next_token().unwrap();
    assert_eq!(
        lexer.next_token().unwrap().punctuator,
        Some(Punctuator::Less)
    );
}

#[test]
fn binary_exponents_are_explicit_and_location_base_uses_target_arithmetic() {
    let source = SourceBuffer::new(b"0x1p+2".to_vec()).unwrap();
    let mut lexer = PpLexer::new(&source).unwrap();
    assert_eq!(lexer.next_token().unwrap().spelling, b"0x1p");
    let options = PpLexerOptions {
        binary_exponents: true,
        location_base: u32::MAX,
        ..PpLexerOptions::default()
    };
    let mut lexer = PpLexer::with_options(&source, options).unwrap();
    let token = lexer.next_token().unwrap();
    assert_eq!(token.spelling, b"0x1p+2");
    assert_eq!((token.begin.0, token.end.0), (u32::MAX, 5));
}

#[test]
fn utf8_scalar_boundaries_roundtrip_without_character_storage() {
    for code in [
        0,
        0x7f,
        0x80,
        0x7ff,
        0x800,
        0xd7ff,
        0xe000,
        0xffff,
        0x10000,
        0x0010_ffff,
    ] {
        let scalar = char::from_u32(code).unwrap();
        let mut encoded = [0; 4];
        let bytes = scalar.encode_utf8(&mut encoded).as_bytes();
        let source = SourceBuffer::new(bytes.to_vec()).unwrap();
        let c = TranslationReader::new(&source, true)
            .logical(SourceOffset(0))
            .unwrap();
        assert_eq!(c.code, code);
        assert_eq!(c.end.index(), bytes.len());
    }
}

#[test]
fn source_cursor_advances_by_physical_bytes_and_rejects_backtracking() {
    use crate::preprocess::source::SourceLineCursor;
    let source = SourceBuffer::new("\u{feff}a\nβ\nz".as_bytes().to_vec()).unwrap();
    let mut cursor = SourceLineCursor::new(&source);
    for (offset, line, column) in [(3, 1, 1), (5, 2, 1), (7, 2, 3), (8, 3, 1)] {
        let position = cursor.advance_to(SourceOffset(offset)).unwrap();
        assert_eq!((position.line, position.column), (line, column));
    }
    assert!(cursor.advance_to(SourceOffset(7)).is_err());
    assert!(cursor.advance_to(SourceOffset(99)).is_err());
}
