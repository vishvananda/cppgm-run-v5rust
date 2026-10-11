#!/usr/bin/env python3
"""Raw differential evidence for PA2's numeric/keyword behavior group.

Supplementary test tooling, independent of the protected manifest. Generated
inputs and raw outputs are retained; they are never compiler answers.
"""
import argparse
from fractions import Fraction
import hashlib
import json
from pathlib import Path
import random
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('/home/vishvananda/work/references/v5rust-cpp')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def decimal(value):
    """Exact finite decimal for a nonnegative dyadic rational."""
    denominator = value.denominator
    power = denominator.bit_length() - 1
    assert denominator == 1 << power
    digits = str(value.numerator * 5 ** power).zfill(power + 1)
    return digits[:-power] + '.' + digits[-power:] if power else digits + '.0'


def corpus():
    groups = {}
    cpp = (REFERENCE / 'dev/src/lex/token_kind.cpp').read_text()
    groups['simple-vocabulary'] = [s for s in re.findall(r'\{ "([^"]+)", TokenKind::', cpp)]
    groups['simple-vocabulary'] += ['#', '##', '%:', '%:%:', 'alignas_', 'co_await', '__int128', '$', '@', '`']
    suffixes = ['', 'u', 'U', 'l', 'L', 'll', 'LL', 'ul', 'lu', 'uL', 'Lu', 'ull', 'llu', 'ULL', 'LLU',
                'lL', 'Ll', 'uuu', 'lll', '_', '_e3', '_π', '_\U00010000', 'u_x', 'l_x', 'abc']
    integers = []
    for width in (0, 1, 7, 15, 31, 32, 63, 64, 65, 128, 512):
        for offset in (-1, 0, 1):
            value = max(0, (1 << width) + offset)
            for prefix in (str(value), '0' + format(value, 'o'), '0x' + format(value, 'x'), '0X' + format(value, 'X')):
                integers.extend(prefix + suffix for suffix in suffixes)
    groups['integer-candidates'] = integers
    groups['numeric-grammar'] = [body + suffix for body in
        ('0x', '0X', '09', '08', '0b1', '0x1p3', '1e+', '1e-', '1e', '1E', '1.', '.0', '1..', '1.0.1',
         '1.0e', '1.0e+', '1.0e-2', '1.0e+2', '123_e3', '0x12_p3') for suffix in
        ('', '_x', '_π', 'f', 'F', 'l', 'L', 'q', 'Q', 'f16', 'f128', 'abc', '_x.y')]
    rng = random.Random(0x905BD7)
    for precision, emax, suffix in [(24, 127, 'f'), (53, 1023, ''), (64, 16383, 'L')]:
        emin = 1 - emax
        values = []
        # Exact halfway values, both parities, across subnormal/normal ranges,
        # exponent transitions and the largest finite binade; and neighbors.
        for exponent in (emin - precision, emin - 1, emin, -10, 0, 10, emax - 1, emax):
            quantum = Fraction(2) ** (max(emin, exponent) - (precision - 1))
            for significand in (0, 1, 2, (1 << (precision - 1)) - 1,
                                1 << (precision - 1), (1 << precision) - 1):
                halfway = (Fraction(significand) + Fraction(1, 2)) * quantum
                delta = quantum / (1 << (precision + 5))
                for value in (halfway - delta, halfway, halfway + delta):
                    values.append(decimal(value) + suffix)
        for _ in range(160):
            exponent = rng.randint(-80 if suffix == 'f' else -500 if not suffix else -5000,
                                   80 if suffix == 'f' else 500 if not suffix else 5000)
            values.append(str(rng.randrange(10 ** rng.randint(1, 80))) + '.'
                          + str(rng.randrange(10 ** rng.randint(1, 80))) + 'e' + str(exponent) + suffix)
        values += [x + suffix for x in ('0e9999999999999999', '1e9999999999999999', '1e-9999999999999999',
                                       '0.' + '0' * 12000 + '1', '1.' + '0' * 12000 + '1')]
        # A digit below the frozen kept-digit boundary must affect ties.
        values += ['1.000000059604644775390625' + '0' * 2000 + '1f'] if suffix == 'f' else []
        groups['binary' + str(precision) + '-rounding'] = values
    return groups


def invoke(binary, data, directory, args=(), stdin='pipe', stdout='capture', offset=0):
    directory.mkdir(parents=True, exist_ok=True)
    source = directory / 'input'
    source.write_bytes(data)
    with source.open('rb') as input_file, (directory / 'stdout').open('wb') as output_file:
        input_file.seek(offset)
        process = subprocess.run([str(binary), *args],
            input=data[offset:] if stdin == 'pipe' else None,
            stdin=input_file if stdin == 'file' else None,
            stdout=subprocess.PIPE if stdout == 'capture' else output_file,
            stderr=subprocess.PIPE, timeout=120)
    if stdout == 'capture':
        (directory / 'stdout').write_bytes(process.stdout)
    (directory / 'stderr').write_bytes(process.stderr)
    return dict(exit=process.returncode, stdout_sha256=digest(directory / 'stdout'),
                stderr_sha256=digest(directory / 'stderr'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    sys.set_int_max_str_digits(0)
    candidate = ROOT / 'obj/rust' / args.profile / 'posttoken'
    oracle = REFERENCE / 'dev/posttoken'
    root = args.evidence.resolve() / args.profile
    root.mkdir(parents=True, exist_ok=True)
    groups = corpus()
    inputs = [(group, ('\n'.join(values) + '\n').encode(), (), 'pipe', 'capture', 0)
              for group, values in groups.items()]
    inputs += [
        ('translation', b'\xef\xbb\xbf1\\\n23 1e+\\\n2 0xFFu _\\u03c0\n', (), 'pipe', 'capture', 0),
        ('error-comment', b'1.0L ok\n /*', (), 'pipe', 'capture', 0),
        ('error-literal', b'1.0L ok\n "\\q"', (), 'pipe', 'capture', 0),
        ('error-byte', b'1.0L ok +\xff', (), 'pipe', 'capture', 0),
        ('seekable', b'ignored 123u 1.0L', (), 'file', 'capture', 8),
        ('empty', b'', (), 'pipe', 'capture', 0),
        ('batch', b'one\n\nlast', ('--batch-stdin',), 'pipe', 'capture', 0),
        ('ignored-args', b'123 .25L', ('--help', 'ignored'), 'pipe', 'capture', 0),
    ]
    rows = []
    for group, data, arguments, stdin, stdout, offset in inputs:
        directory = root / group
        actual = invoke(candidate, data, directory / 'candidate', arguments, stdin, stdout, offset)
        expected = invoke(oracle, data, directory / 'oracle', arguments, stdin, stdout, offset)
        rows.append(dict(group=group, input_sha256=hashlib.sha256(data).hexdigest(), input_bytes=len(data),
                         arguments=arguments, stdin=stdin, offset=offset, candidate=actual, oracle=expected,
                         match=actual == expected and actual['exit'] in (0, 1), tokens=len(groups.get(group, []))))
    result = dict(candidate=str(candidate), candidate_sha256=digest(candidate),
                  oracle=str(oracle), oracle_sha256=digest(oracle), comparisons=rows,
                  token_counts={k: len(v) for k, v in groups.items()},
                  passed=sum(row['match'] for row in rows), total=len(rows))
    (root / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    for row in rows:
        print(row['group'], 'PASS' if row['match'] else 'FAIL', row['tokens'])
    return 0 if result['passed'] == result['total'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
