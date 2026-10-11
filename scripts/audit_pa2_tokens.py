#!/usr/bin/env python3
"""Compare PA2 typed facts directly, including facts absent from the CLI view.

The probes are built outside the checkout from frozen C++ owners and the Rust
library. They are test-only consumers; neither is a candidate compiler tool.
"""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import random
import subprocess

from audit_pa2_numbers import ROOT, REFERENCE, digest, invoke


CPP = r'''
#include <iostream>
#include "preprocess/source/source_buffer.h"
#include "preprocess/lexer/pp_lexer.h"
#include "lex/post_tokenizer.h"

void hex(const void* data, uint32_t size) {
    const auto* bytes = static_cast<const unsigned char*>(data);
    const char* digits = "0123456789ABCDEF";
    for (uint32_t i = 0; i < size; ++i)
        std::cout << digits[bytes[i] >> 4] << digits[bytes[i] & 15];
}

int main() {
    auto source = SourceBuffer::ReadFrom(std::cin);
    try {
        PPLexer lexer(source);
        PostTokenizer tokens(lexer);
        unsigned eof = 0;
        while (eof != 3) {
            const auto& t = tokens.next();
            std::cout << "SourceOffset(" << t.begin << ") SourceOffset(" << t.end << ") ";
            hex(t.source.text, t.source.length);
            std::cout << " | " << unsigned(t.kind);
            if (t.kind == PostTokenKind::Simple)
                std::cout << ' ' << TokenKindName(t.simple);
            if (t.kind == PostTokenKind::Literal || t.kind == PostTokenKind::UserDefinedLiteral) {
                std::cout << ' ' << unsigned(t.form) << ' ';
                hex(t.ud_suffix.text, t.ud_suffix.length);
                if (t.kind == PostTokenKind::UserDefinedLiteral &&
                    (t.form == LiteralForm::Integer || t.form == LiteralForm::Floating)) {
                    std::cout << ' ';
                    hex(t.source.text, t.source.length - t.ud_suffix.length);
                } else {
                    std::cout << ' ' << FundamentalTypeName(t.type) << ' ';
                    hex(t.data, t.size);
                    std::cout << ' ' << t.elements << ' ' << t.multicharacter;
                }
            }
            std::cout << '\n';
            eof += t.kind == PostTokenKind::EndOfFile;
        }
    } catch (const SourceError& e) {
        std::cout.flush();
        std::cerr << "ERROR: <stdin>:" << FormatSourceError(source, e) << '\n';
        return 1;
    }
}
'''

RUST = r'''
use cppgm_rust::lex::{LiteralForm, PostTokenError, PostTokenKind, PostTokenizer, number::NumberDialect};
use cppgm_rust::support::source::SourceBuffer;
use std::io::{self, Read};

fn hex(data: &[u8]) -> String {
    data.iter().map(|c| format!("{c:02X}")).collect()
}

fn main() -> std::process::ExitCode {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let source = SourceBuffer::new(input).unwrap();
    let mut tokens = match PostTokenizer::new(&source, NumberDialect::Cxx11) {
        Ok(tokens) => tokens,
        Err(e) => {
            eprintln!("ERROR: <stdin>:{}", source.format_error(&e));
            return std::process::ExitCode::FAILURE;
        }
    };
    let mut eof = 0;
    while eof != 3 {
        let t = match tokens.next_token() {
            Ok(t) => t,
            Err(PostTokenError::Source(e)) => {
                eprintln!("ERROR: <stdin>:{}", source.format_error(&e));
                return std::process::ExitCode::FAILURE;
            }
            Err(e) => panic!("unexpected probe limit: {e:?}"),
        };
        print!("{:?} {:?} {} | ", t.begin, t.end, hex(t.source));
        match t.kind {
            PostTokenKind::Simple(s) => print!("0 {}", s.name()),
            PostTokenKind::Identifier => print!("1"),
            PostTokenKind::Literal { form, ty, data } =>
                print!("2 {}  {} {} 0 0", form as u8, ty.name(), hex(data.as_slice())),
            PostTokenKind::UserDefinedNumber { form, suffix_begin } =>
                print!("3 {} {} {}", form as u8, hex(&t.source[suffix_begin..]), hex(&t.source[..suffix_begin])),
            PostTokenKind::Character { ty, data, multicharacter, suffix_begin } =>
                print!("{} {} {} {} {} 0 {}", if suffix_begin.is_some() { 3 } else { 2 },
                    LiteralForm::Character as u8, suffix_begin.map_or(String::new(), |at| hex(&t.source[at..])),
                    ty.name(), hex(data.as_slice()), u8::from(multicharacter)),
            PostTokenKind::String { ty, elements, suffix } =>
                print!("{} {} {} {} {} {} 0", if suffix.is_some() { 3 } else { 2 },
                    LiteralForm::String as u8, hex(t.suffix), ty.name(), hex(t.data), elements),
            PostTokenKind::Invalid => print!("4"),
            PostTokenKind::EndOfFile => { print!("5"); eof += 1; }
        }
        println!();
    }
    std::process::ExitCode::SUCCESS
}
'''


def build(root, profile):
    owners = '''support/unicode/utf8 preprocess/source/source_buffer
        preprocess/source/translation_reader preprocess/lexer/char_classes
        preprocess/lexer/pp_lexer preprocess/lexer/pp_lexer_literals
        preprocess/lexer/pp_lexer_punctuators types/fundamental_type lex/gnu_builtins
        lex/token_kind lex/ud_suffix lex/number_literal lex/interchange_float
        lex/text_literal support/memory/arena lex/identifier_table
        lex/string_concatenation lex/post_tokenizer'''.split()
    cpp, rust = root / 'probe.cpp', root / 'probe.rs'
    cpp.write_text(CPP)
    rust.write_text(RUST)
    commands = [
        ['g++', '-std=gnu++11', '-O2', '-I' + str(REFERENCE / 'dev/src'), str(cpp),
         *[str(REFERENCE / 'dev/src' / (s + '.cpp')) for s in owners], '-o', str(root / 'cpp')],
        ['rustc', '--edition=2024', '--forbid', 'unsafe_code', str(rust),
         '--extern', 'cppgm_rust=' + str(ROOT / 'obj/rust' / profile / 'libcppgm_rust.rlib'),
         '-L', 'dependency=' + str(ROOT / 'obj/rust' / profile / 'deps'), '-o', str(root / 'rust')],
    ]
    (root / 'build-commands.json').write_text(json.dumps(commands, indent=2) + '\n')
    for i, command in enumerate(commands):
        with (root / f'build-{i}.log').open('wb') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)


def corpus():
    yield 'scalar-facts', b"'ABCDE' '\\377' u'\\xFFFF' U'\\x10000'_x 0x80000000 1.0L 1e9999_x"
    yield 'lookahead', b'"abc" "def" 7'
    yield 'eof-lookahead', b'"abc"'
    yield 'invalid-lookahead', b'u"a" U"b" 7'
    yield 'split-spans', b'operator ""\\\noperator ""and operator ""_x'
    prefixes = ['', 'u8', 'u', 'U', 'L']
    suffixes = ['', '_', '_x', '_\u03c0', 'reserved']
    for i, (p, q, s, t) in enumerate(itertools.product(prefixes, prefixes, suffixes, suffixes)):
        yield f'sequence-{i:03}', (f'{p}"\\x100"{s} /* gap */ {q}R"d(\u03c0\U00010000)d"{t} ;').encode()
    rng = random.Random(0x905bd7)
    atoms = ['operator', '""and', '""operator', '""_x', '"text"', 'u"\\xD800"',
             'U"\\U00010000"', 'R"d(\\xFF)d"_x', "'AB'", "L'\u03c0'_x", '1.0L',
             '1e+3_x', '0xffffffffffffffffL', '#', '%:%:', 'include', '<h>', '_\u03c0', ';']
    for i in range(200):
        yield f'mixed-{i:03}', ' /* gap */ '.join(rng.choices(atoms, k=32)).encode()
    for i, bad in enumerate([b'/*', b'"\\q"', b'\\uD800', b'+\xff']):
        for j, prefix in enumerate([b'1 ', b'"ok" ', b'operator ""sv ']):
            yield f'failure-{i}-{j}', prefix + bad


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    root = args.evidence.resolve()
    root.mkdir(parents=True, exist_ok=True)
    build(root, args.profile)
    rows = []
    for name, data in corpus():
        actual = invoke(root / 'rust', data, root / name / 'rust')
        expected = invoke(root / 'cpp', data, root / name / 'cpp')
        rows.append(dict(case=name, input_sha256=hashlib.sha256(data).hexdigest(),
                         candidate=actual, oracle=expected,
                         match=actual == expected and actual['exit'] in (0, 1)))
    result = dict(probes={s: digest(root / s) for s in ['cpp', 'rust']},
                  profile=args.profile,
                  library_sha256=digest(ROOT / 'obj/rust' / args.profile / 'libcppgm_rust.rlib'),
                  passed=sum(r['match'] for r in rows), total=len(rows), comparisons=rows)
    (root / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f"Typed post-token comparisons: {result['passed']}/{result['total']}")
    for row in rows:
        if not row['match']:
            print('FAIL', row['case'])
    return 0 if result['passed'] == result['total'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
