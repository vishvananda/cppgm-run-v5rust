#!/usr/bin/env python3
"""Compare PA3's typed outcomes, cursor demand and macro queries directly.

Test-only consumers are built outside the checkout. The C++ probe reads frozen
owners; the Rust probe links the current library. Neither supplies tool output.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from audit_pa3_expressions import ROOT, REFERENCE, corpus, digest, invoke


CPP = r'''
#include <iostream>
#include "preprocess/source/source_buffer.h"
#include "preprocess/lexer/pp_lexer.h"
#include "preprocess/expr/controlling_expression.h"

void hex(const char* data, uint32_t size) {
    const char* digits = "0123456789ABCDEF";
    for (uint32_t i = 0; i < size; ++i) {
        const auto c = static_cast<unsigned char>(data[i]);
        std::cout << digits[c >> 4] << digits[c & 15];
    }
}

class Cursor : public PPTokenCursor {
public:
    explicit Cursor(PPLexer& lexer) : lexer_(lexer) {}
    const PPToken& next() override {
        std::cout << "demand\n";
        const auto& t = lexer_.next();
        std::cout << "token " << unsigned(t.kind) << " SourceOffset(" << t.begin
                  << ") SourceOffset(" << t.end << ") ";
        hex(t.spelling, t.length);
        std::cout << '\n';
        return t;
    }
private:
    PPLexer& lexer_;
};

class Macros : public MacroDefinitionQuery {
public:
    bool is_defined(const char* spelling, uint32_t length) const override {
        std::cout << "query ";
        hex(spelling, length);
        std::cout << '\n';
        return length != 0 && (static_cast<unsigned char>(spelling[0]) & 1) != 0;
    }
};

int main() {
    const auto source = SourceBuffer::ReadFrom(std::cin);
    try {
        PPLexer lexer(source);
        Cursor cursor(lexer);
        Macros macros;
        ControllingExpressionEvaluator evaluator(macros);
        unsigned eof = 0;
        while (eof != 3) {
            const auto result = evaluator.evaluate(cursor);
            std::cout << "result ";
            switch (result.status) {
            case ControllingExpressionStatus::Empty: std::cout << "empty"; break;
            case ControllingExpressionStatus::Invalid: std::cout << "invalid"; break;
            case ControllingExpressionStatus::Valid:
                std::cout << result.value.bits << ' ' << result.value.is_unsigned; break;
            }
            std::cout << " eof=" << result.end_of_file << '\n';
            eof += result.end_of_file;
        }
    } catch (const SourceError& error) {
        std::cout.flush();
        std::cerr << "ERROR: <stdin>:" << FormatSourceError(source, error) << '\n';
        return 1;
    }
}
'''

RUST = r'''
use cppgm_rust::lex::{number::NumberDialect, pp_lexer::{PpLexer, PpToken, PpTokenCursor}};
use cppgm_rust::preprocess::expression::{ExpressionError, ExpressionEvaluator,
    ExpressionStatus, MacroDefinitionQuery};
use cppgm_rust::support::source::{SourceBuffer, SourceError};
use std::io::{self, Read, Write};

fn hex(data: &[u8]) -> String {
    data.iter().map(|c| format!("{c:02X}")).collect()
}

struct Cursor<'a>(PpLexer<'a>);
impl PpTokenCursor for Cursor<'_> {
    fn next_token(&mut self) -> Result<PpToken<'_>, SourceError> {
        println!("demand");
        let token = self.0.next_token()?;
        println!("token {} {:?} {:?} {}", token.kind as u8,
                 token.begin, token.end, hex(token.spelling));
        Ok(token)
    }
}

struct Macros;
impl MacroDefinitionQuery for Macros {
    fn is_defined(&self, spelling: &[u8]) -> bool {
        println!("query {}", hex(spelling));
        spelling.first().is_some_and(|byte| byte & 1 != 0)
    }
}

fn evaluate(source: &SourceBuffer) -> Result<(), SourceError> {
    let mut cursor = Cursor(PpLexer::new(source)?);
    let macros = Macros;
    let mut evaluator = ExpressionEvaluator::new(&macros, NumberDialect::Cxx11);
    let mut eof = 0;
    while eof != 3 {
        let result = match evaluator.evaluate(&mut cursor) {
            Ok(result) => result,
            Err(ExpressionError::Source(error)) => return Err(error),
            Err(error) => panic!("unexpected invariant error: {error:?}"),
        };
        print!("result ");
        match result.status {
            ExpressionStatus::Empty => print!("empty"),
            ExpressionStatus::Invalid => print!("invalid"),
            ExpressionStatus::Valid(value) => print!("{} {}", value.bits, u8::from(value.is_unsigned)),
        }
        println!(" eof={}", u8::from(result.end_of_file));
        eof += u8::from(result.end_of_file);
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    let mut input = Vec::new();
    io::stdin().read_to_end(&mut input).unwrap();
    let source = SourceBuffer::new(input).unwrap();
    match evaluate(&source) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            io::stdout().flush().unwrap();
            eprintln!("ERROR: <stdin>:{}", source.format_error(&error));
            std::process::ExitCode::FAILURE
        }
    }
}
'''


def build(root, profile):
    owners = '''support/unicode/utf8 preprocess/source/source_buffer
        preprocess/source/translation_reader preprocess/lexer/char_classes
        preprocess/lexer/pp_lexer preprocess/lexer/pp_lexer_literals
        preprocess/lexer/pp_lexer_punctuators types/fundamental_type lex/gnu_builtins
        lex/token_kind lex/ud_suffix lex/number_literal lex/interchange_float
        lex/text_literal preprocess/expr/pp_arithmetic
        preprocess/expr/controlling_expression'''.split()
    (root / 'probe.cpp').write_text(CPP)
    (root / 'probe.rs').write_text(RUST)
    commands = [
        ['g++', '-std=gnu++11', '-O2', '-I' + str(REFERENCE / 'dev/src'),
         str(root / 'probe.cpp'),
         *[str(REFERENCE / 'dev/src' / (owner + '.cpp')) for owner in owners],
         '-o', str(root / 'cpp')],
        ['rustc', '--edition=2024', '--forbid', 'unsafe_code', str(root / 'probe.rs'),
         '--extern', 'cppgm_rust=' + str(ROOT / 'obj/rust' / profile / 'libcppgm_rust.rlib'),
         '-L', 'dependency=' + str(ROOT / 'obj/rust' / profile / 'deps'),
         '-o', str(root / 'rust')],
    ]
    (root / 'build-commands.json').write_text(json.dumps(commands, indent=2) + '\n')
    for index, command in enumerate(commands):
        with (root / f'build-{index}.log').open('wb') as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True)


def inputs():
    expressions = [
        'defined(a)', 'defined b', 'defined (class)', 'defined(defined)',
        'defined π', 'defined Ā', 'defined(\\u03C0)', 'defined /* gap */ ( a )',
        'defined(a b)', 'defined()', 'defined(1)', 'defined and', 'defined(',
        'defined', '0 && defined(a)', '1 || defined(b)', '1 ? 5 : defined(a)',
        '0 ? defined(a) : -5', '1 + ) defined(a)', '1/0 + defined(a)',
        '1/0 * defined(a)', '0 && 1/0 + defined(a)',
        'false ? 5/0u : -5', '0 ? (5/0 ? 1u : -2) : -3',
        '0 ? (1 << -1u) : -5', '0 ? (1u << -1) : -5',
        '0 && (1 || (1/0))', '1 || (0 && (1/0))',
        '0 ? 1 : 1 ? 2u : -3', '1 ? 0 ? 2u : -3 : 4',
        '0 ? 1 : 2 : 3', '1 ? (2 : 3)', '((1 ? 2 : 3))',
    ]
    # A good line after every error proves recovery and capacity/state reuse.
    for index, expression in enumerate(expressions):
        yield f'boundary-{index:02}', (expression + '\n7\n').encode(), 0
    groups = corpus()
    for name, limit in [('unary-boundaries', 150), ('precedence', 576),
                        ('nested-short-circuit', 900), ('generated-expressions', 400),
                        ('malformed-token-streams', 800), ('deep-stacks', 3)]:
        yield name, ('\n'.join(groups[name][:limit]) + '\n7\n').encode(), 0
    yield 'empty', b'', 0
    yield 'translation', b'\xef\xbb\xbf1??/\n23 + 4\n/* a\nb */ defined(a)\n', 0
    for index, source in enumerate(sorted((ROOT / 'pa3/tests').glob('*.t'))):
        yield f'course-{index:02}', source.read_bytes(), 0
    for index, bad in enumerate([b'/*', b'"\\q"', b'\xc0', b'\\uD800']):
        for position, prefix in enumerate([b'', b'1\n', b'1\n) ', b'1\ndefined(a ',
                                           b'1\n0 && defined(a) + ']):
            yield f'source-error-{index}-{position}', prefix + bad, 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    root = args.evidence.resolve()
    root.mkdir(parents=True, exist_ok=True)
    build(root, args.profile)
    rows = []
    for name, data, expected_exit in inputs():
        directory = root / name
        actual = invoke(root / 'rust', data, directory / 'rust')
        expected = invoke(root / 'cpp', data, directory / 'cpp')
        reached_eof = (directory / 'rust/stdout').read_bytes().count(b' eof=1\n') == 3
        rows.append(dict(case=name, input_sha256=hashlib.sha256(data).hexdigest(),
                         candidate=actual, oracle=expected, expected_exit=expected_exit,
                         match=actual == expected and actual['exit'] == expected_exit
                         and (expected_exit != 0 or reached_eof)))
    result = dict(profile=args.profile, probes={s: digest(root / s) for s in ['cpp', 'rust']},
                  library_sha256=digest(ROOT / 'obj/rust' / args.profile / 'libcppgm_rust.rlib'),
                  passed=sum(row['match'] for row in rows), total=len(rows), comparisons=rows)
    (root / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(f"PA3 typed/demand comparisons: {result['passed']}/{result['total']}")
    for row in rows:
        if not row['match']:
            print('FAIL', row['case'])
    return 0 if result['passed'] == result['total'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
