#!/usr/bin/env python3
"""Supplemental PA1 audit: raw CLI comparisons beyond the frozen course lane.

This is test tooling. It never supplies a candidate result or changes the
protected manifest. Every generated input and both invocation results are
retained so an audit can be reproduced from its receipt.
"""
import argparse
from collections import Counter
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import random
import re
import subprocess


ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('/home/vishvananda/work/references/v5rust-cpp')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def vocabulary():
    cpp = (REFERENCE / 'dev/src/preprocess/lexer/char_classes.cpp').read_text()
    rust = (ROOT / 'dev/src/preprocess/lexer/classes.rs').read_text()
    cpp_tables = re.findall(r'Ranges\[\] = \{(.*?)\};', cpp, re.S)
    cpp_ranges = [[tuple(int(v, 16) for v in pair) for pair in
                   re.findall(r'\{\s*(0x\w+),\s*(0x\w+)\s*\}', table)]
                  for table in cpp_tables]
    e1 = rust.split('const ANNEX_E1:')[1]
    e2 = rust.split('pub(super) fn identifier_start')[1].split('pub(super) fn identifier_continue')[0]
    rust_ranges = [[tuple(int(v, 16) for v in pair) for pair in
                    re.findall(r'\((0x\w+),\s*(0x\w+)\)', table)] for table in (e1, e2)]
    if cpp_ranges != rust_ranges:
        raise RuntimeError('Annex E source vocabulary mismatch')
    cpp_punc = (REFERENCE / 'dev/src/preprocess/lexer/pp_lexer_punctuators.cpp').read_text()
    rust_punc = (ROOT / 'dev/src/preprocess/lexer/punctuators.rs').read_text()
    cpp_spellings = re.findall(r'CPPGM_PUNCTUATOR\("([^"]+)"', cpp_punc)
    cpp_spellings += ['#', '##', '%:', '%:%:']
    rust_spellings = re.findall(r'\(b"([^"]+)", Punctuator::', rust_punc)
    cpp_words = re.findall(r'CPPGM_IDENTIFIER_LIKE\("([^"]+)"', cpp_punc)
    rust_words = re.findall(r'b"([^"]+)" => Punctuator::', rust_punc)
    if sorted(cpp_spellings) != sorted(rust_spellings) or sorted(cpp_words) != sorted(rust_words):
        raise RuntimeError('Punctuator source vocabulary mismatch')
    return cpp_ranges, [s.encode() for s in cpp_spellings + cpp_words]


def cases():
    ranges, punctuation = vocabulary()
    result = []

    def add(group, data, args=(), stdin='pipe', stdout='capture', offset=0):
        result.append(dict(group=group, input_hex=data.hex(), arguments=list(args),
                           stdin=stdin, stdout=stdout, offset=offset))

    for byte in range(256):
        for prefix, suffix in [(b'', b''), (b'ok\n', b'\n'), (b'R"(', b')"'),
                               (b'R"', b'(x)"'), (b'#include <', b'>')]:
            add('physical-bytes', prefix + bytes([byte]) + suffix)
    boundary = {0, 0x7f, 0x80, 0x7ff, 0x800, 0xd7ff, 0xd800, 0xdfff,
                0xe000, 0xffff, 0x10000, 0x10ffff, 0x110000, 0xffffffff}
    for table in ranges:
        for first, last in table:
            boundary.update([first - 1, first, first + 1, last - 1, last, last + 1])
    for code in sorted(boundary):
        text = ('\\U%08X' % code).encode()
        for prefix, suffix in [(b'', b''), (b'a', b'z'), (b'1', b'e+2'),
                               (b'"', b'"'), (b'"x"', b''), (b'#include <', b'>')]:
            add('unicode-boundaries', prefix + text + suffix)
        if code <= 0x10ffff and not 0xd800 <= code <= 0xdfff:
            raw = chr(code).encode()
            for prefix, suffix in [(b'', b''), (b'a', b'z'), (b'1', b'e+2')]:
                add('unicode-boundaries', prefix + raw + suffix)
    for left in punctuation:
        for right in punctuation:
            add('punctuator-order', left + right)
        for tail in [b'\xff', b'\xf0\x90', b'\xe0\x80\x80', b'\\\n\xff']:
            add('lookahead-failure', b'ok ' + left + tail)
    for third in b"=/'()!<>-?x":
        for prefix in [b'', b'?', b'??', b'\\\n', b'??/\n']:
            add('translation-order', prefix + b'??' + bytes([third]) + b'\n')
    for value in range(128):
        text = ('\\u%04X' % value).encode()
        for prefix, suffix in [(b'', b''), (b'"', b'"'), (b"'", b"'"), (b'/*', b'*/')]:
            add('basic-ucns', prefix + text + suffix)
    for prefix in [b'R', b'u8R', b'uR', b'UR', b'LR', b'u\\\n8R']:
        for delim in [b'', b'a', 'ª'.encode(), b'\xaa', b'\r', b'\0', b'??=']:
            for size in [0, 1, 15, 16, 17]:
                delimiter = delim * size
                add('raw-extents', prefix + b'"' + delimiter + b'(??/\n)wrong)'
                    + delimiter + b'"_suffix')
    for prefix in [b'#include', b'%:include', b'??=include', b'#include_next',
                   b' # /*\n*/ include', b'x #include', b'#include x', b'#other']:
        for header in [b'<a>', b'"a"', b'<>', b'""', b'<x', b'"x',
                       b'<\\u00AA>', b'<\\u0041>', b'<x\\\ny>', b'<\xff>']:
            add('header-context', prefix + b' ' + header)
    rng = random.Random(1731)
    fragments = [b'a', b'1.2e+3', b'.0e-', b'<:::', b'<::>', b'<::a', b'%:%:',
                 b'#include <x>', b'\n', b' ', b'\\\n', b'??/', b'??=', b'\\u00AA',
                 b'\\u0300', b'/*a\n*/', b'//x\n', b'"\\u0022"', b"u'\\123'",
                 b'R"tag(??/\n\\u00AA)tag"_x', b'"x"_suffix', b'\x80',
                 b'\xf0\x9f\x98\x80', b'"\\x"', b'\xff']
    for _ in range(512):
        add('mixed-sequences', b''.join(rng.choices(fragments, k=32)))
        add('random-bytes', rng.randbytes(rng.randrange(1, 128)))
    for data in [b'', b'\n', b'a\nb', b'\xff\n\xc0', b'\r\n', b'\n\n']:
        for args in [(), ('--unknown',), ('--batch-stdin',), ('--unknown', '--batch-stdin')]:
            add('cli-arguments', data, args)
    for data in [b'a', b'ok\n"\\q"', b'a\n' * 10000,
                 b'a\n' * 10000 + b'"\\q"']:
        add('failed-output', data, stdout='full')
    add('failed-output', b'a\nb', ('--batch-stdin',), stdout='full')
    for args in [(), ('--batch-stdin',)]:
        add('failed-input', b'', args, stdin='directory')
    for data in [b'ignored kept', b'ignored \xef\xbb\xbfok\n"\\q"']:
        add('seekable-input', data, stdin='file', offset=8)
    return result


def invoke(tool, case, directory):
    data = bytes.fromhex(case['input_hex'])
    with (directory / 'input').open('wb') as stream:
        stream.write(data)
    descriptor = None
    try:
        if case['stdin'] == 'directory':
            descriptor = os.open(ROOT / 'pa1', os.O_RDONLY)
        elif case['stdin'] == 'file':
            descriptor = os.open(directory / 'input', os.O_RDONLY)
            os.lseek(descriptor, case['offset'], os.SEEK_SET)
        with open('/dev/full', 'wb') as full:
            process = subprocess.run([str(tool), *case['arguments']],
                                     input=data if descriptor is None else None,
                                     stdin=descriptor,
                                     stdout=full if case['stdout'] == 'full' else subprocess.PIPE,
                                     stderr=subprocess.PIPE, timeout=10)
    finally:
        if descriptor is not None:
            os.close(descriptor)
    return dict(exit=process.returncode, stdout_hex=(process.stdout or b'').hex(),
                stderr_hex=process.stderr.hex())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', required=True, type=Path)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)
    candidate = ROOT / 'obj/rust' / args.profile / 'pptoken'
    oracle = REFERENCE / 'dev/pptoken'
    corpus = cases()

    def compare(item):
        number, case = item
        directory = args.evidence / f'case-{number}'
        directory.mkdir(exist_ok=True)
        actual = invoke(candidate, case, directory)
        expected = invoke(oracle, case, directory)
        match = actual == expected and actual['exit'] in (0, 1)
        (directory / 'input').unlink()
        directory.rmdir()
        return dict(id=number, **case, candidate=actual, oracle=expected, match=match)

    with ThreadPoolExecutor(max_workers=4) as pool:
        records = list(pool.map(compare, enumerate(corpus)))
    receipt = dict(profile=args.profile, candidate_sha256=digest(candidate),
                   oracle_sha256=digest(oracle), cases=records,
                   vocabulary='exact Annex E1/E2 ranges and all punctuator spellings')
    (args.evidence / f'{args.profile}.json').write_text(json.dumps(receipt, indent=2) + '\n')
    counts = Counter(case['group'] for case in corpus)
    print('Corpus:', dict(sorted(counts.items())))
    failures = [record['id'] for record in records if not record['match']]
    print(f'PA1 {args.profile} supplemental exact comparisons: '
          f'{len(records) - len(failures)}/{len(records)}; failing IDs: {failures}')
    return bool(failures)


if __name__ == '__main__':
    raise SystemExit(main())
