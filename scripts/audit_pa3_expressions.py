#!/usr/bin/env python3
"""Supplemental raw PA3 comparisons; generated inputs never enter compiler code."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import random
import subprocess

ROOT = Path(__file__).resolve().parents[1]
REFERENCE = Path('/home/vishvananda/work/references/v5rust-cpp')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def corpus():
    values = ['0', '1', '2', '-1', '-2', '2147483647', '2147483648',
              '9223372036854775807', '(1 << 63)', '0u', '1u', '2u',
              '0x7fffffffffffffff', '0x8000000000000000', '0xffffffffffffffff',
              '18446744073709551615u', '63', '64', '65', "'a'", "u'\\xffff'",
              "U'\\U0001ffff'", "L'\\x7fffffff'", 'true', 'false']
    operators = ['*', '/', '%', '+', '-', '<<', '>>', '<', '>', '<=', '>=',
                 '==', '!=', '&', '^', '|', '&&', '||', 'bitand', 'xor',
                 'bitor', 'and', 'or', 'not_eq']
    groups = {
        'binary-boundaries': [f'({a}) {op} ({b})' for a, op, b in
                              itertools.product(values, operators, values)],
        'unary-boundaries': [f'{op} ({a})' for op, a in
                             itertools.product(['+', '-', '!', '~', 'not', 'compl'], values)],
        'precedence': [f'1 {a} 2 {b} 3' for a, b in itertools.product(operators, repeat=2)],
    }
    invalid = ['1.0', '1_x', '09', '0x', '18446744073709551616', '9223372036854775808',
               "'ab'", "u'\\U00010000'", '"abc"', 'nullptr + 1', 'new', 'delete',
               'defined and', 'defined ()', 'defined (1)', 'defined defined', 'defined(true)',
               'defined (class)', 'defined π', 'defined Ā', 'defined (a b)', '#', '##', '@']
    groups['conversion-and-defined'] = invalid + [f'{prefix} ({value})' for prefix, value in
        itertools.product(['0 &&', '1 ||', '0 ? 5 :', '1 ? 5 :'], invalid)]
    bad_ops = ['1/0', '(1 << 63)/-1', '(1 << 63)%-1', '1 << -1', '1u >> 64',
               '9223372036854775807 + 1', '-(1 << 63)', '(1 << 63) - 1',
               '9223372036854775807 * 2', '5/0u']
    groups['nested-short-circuit'] = [f'{condition} ? ({a}) : ({b})' for condition, a, b in
        itertools.product(['0', '1', '1u', '(1 << 63)'], bad_ops + values[:5], bad_ops + values[:5])]
    rng = random.Random(0x905BD7)

    def expression(depth):
        if depth == 0 or rng.randrange(5) == 0:
            return rng.choice(values + ['defined(a)', 'defined(b)', 'identifier'])
        kind = rng.randrange(4)
        if kind == 0:
            return rng.choice(['+', '-', '!', '~']) + ' (' + expression(depth - 1) + ')'
        if kind == 1:
            return '(' + expression(depth - 1) + ') ? (' + expression(depth - 1) + ') : (' + expression(depth - 1) + ')'
        return '(' + expression(depth - 1) + ') ' + rng.choice(operators) + ' (' + expression(depth - 1) + ')'

    groups['generated-expressions'] = [expression(5) for _ in range(4000)]
    tokens = values[:5] + ['(', ')', '?', ':', 'defined', 'a', '1.0', '+', '*', '&&', '||']
    groups['malformed-token-streams'] = [' '.join(rng.choices(tokens, k=rng.randrange(1, 30))) for _ in range(3000)]
    groups['deep-stacks'] = ['(' * 65536 + '1' + ')' * 65536,
                            '+ ' * 65536 + '1', '0 ? 1u : ' * 65536 + '-5']
    return groups


def invoke(binary, data, directory, arguments=(), mode='pipe', offset=0):
    directory.mkdir(parents=True, exist_ok=True)
    source = directory / 'input'
    source.write_bytes(data)
    with source.open('rb') as input_file:
        input_file.seek(offset)
        process = subprocess.run([str(binary), *arguments],
                                 input=data[offset:] if mode == 'pipe' else None,
                                 stdin=input_file if mode == 'file' else None,
                                 capture_output=True, timeout=60)
    (directory / 'stdout').write_bytes(process.stdout)
    (directory / 'stderr').write_bytes(process.stderr)
    return dict(exit=process.returncode, stdout_sha256=digest(directory / 'stdout'),
                stderr_sha256=digest(directory / 'stderr'), lines=len(process.stdout.splitlines()))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    candidate = ROOT / 'obj/rust' / args.profile / 'ppexpr'
    oracle = REFERENCE / 'dev/ppexpr'
    pinned = json.loads((ROOT / 'scripts/clone/manifest.json').read_text())['oracle_tools']['ppexpr']
    if digest(oracle) != pinned:
        raise SystemExit('oracle hash does not match the frozen manifest')
    root = args.evidence.resolve() / args.profile
    root.mkdir(parents=True, exist_ok=True)
    groups = corpus()
    inputs = [(name, ('\n'.join(lines) + '\n').encode(), (), 'pipe', 0, len(lines) + 1)
              for name, lines in groups.items()]
    inputs += [
        ('translation', b'\xef\xbb\xbf1??/\n23 + 4\n\n/* a\nb */ true\n', (), 'pipe', 0, None),
        ('initial-error', b'\xc0', (), 'pipe', 0, None),
        ('late-error', b'1\n7 + "\\q"', (), 'pipe', 0, None),
        ('invalid-then-source-error', b'1\n) "\\q"', (), 'pipe', 0, None),
        ('comment-error', b'1\n2 + /*', (), 'pipe', 0, None),
        ('decode-error', b'1\n2 + \xc0', (), 'pipe', 0, None),
        ('flush-then-error', b'1\n' * 40000 + b'"\\q"', (), 'pipe', 0, None),
        ('seekable', b'ignored 1 + 2u', (), 'file', 8, None),
        ('empty', b'', (), 'pipe', 0, None),
        ('batch', b'one\n\nlast', ('--batch-stdin',), 'pipe', 0, None),
        ('ignored-arguments', b'1+2', ('--help', 'ignored'), 'pipe', 0, None),
    ]
    for source in sorted((ROOT / 'pa3/tests').glob('*.t')):
        inputs.append(('course-' + source.stem, source.read_bytes(), (), 'pipe', 0, None))
    rows = []
    for name, data, arguments, mode, offset, lines in inputs:
        directory = root / name
        actual = invoke(candidate, data, directory / 'candidate', arguments, mode, offset)
        expected = invoke(oracle, data, directory / 'oracle', arguments, mode, offset)
        rows.append(dict(group=name, input_sha256=hashlib.sha256(data).hexdigest(),
                         input_bytes=len(data), arguments=arguments, mode=mode, offset=offset,
                         candidate=actual, oracle=expected,
                         match=actual == expected and actual['exit'] in (0, 1)
                         and (lines is None or actual['lines'] == lines)))
        print(name, 'PASS' if rows[-1]['match'] else 'FAIL')
    result = dict(candidate=str(candidate), candidate_sha256=digest(candidate),
                  oracle=str(oracle), oracle_sha256=digest(oracle), comparisons=rows,
                  expression_counts={k: len(v) for k, v in groups.items()},
                  passed=sum(row['match'] for row in rows), total=len(rows))
    (root / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    return 0 if result['passed'] == result['total'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
