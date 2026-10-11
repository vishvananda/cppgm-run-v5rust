#!/usr/bin/env python3
"""Supplementary raw comparisons for PA2 text literals and sequence boundaries.

Inputs and raw results remain in the requested evidence directory. This does
not modify the frozen fixtures, gate manifest or acceptance policy.
"""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import random
import sys

from audit_pa2_numbers import ROOT, REFERENCE, digest, invoke


def corpus():
    prefixes = ['', 'u8', 'u', 'U', 'L']
    suffixes = ['', '_', '_x', '_π', '_𐀀', 'reserved', 'and', '__x']
    bodies = ['', 'A', 'π', '𐀀', '\\0', '\\377', '\\400', '\\777', '\\x7f', '\\x80',
              '\\xff', '\\x100', '\\xffff', '\\x10000', '\\x10ffff', '\\xd800',
              '\\xffffffff', '\\x100000000', '\\x' + 'f' * 8000, '\\x' + '0' * 8000 + 'ff',
              '\\u0000', '\\u0022', '\\u005c', '\\u03c0', '\\U00010000',
              '\\a\\b\\f\\n\\r\\t\\v\\?\\\\\\\"', '\\1234', '\\08', 'ABCDEF']
    groups = {}
    groups['strings'] = [f'{p}"{body}"{s}' for p, body, s in itertools.product(prefixes, bodies, suffixes)]
    character_bodies = [b for b in bodies if '\\"' not in b] + ["\\'", 'AB', 'ABCDE', 'éñ', '\\xff\\xff']
    groups['characters'] = [f"{p}'{body}'{s}" for p, body, s in
                            itertools.product(['', 'u', 'U', 'L'], character_bodies, suffixes)]
    raw_bodies = ['', 'π𐀀', '\\x100 \\u0000', 'quote" apostrophe\' backslash\\',
                  'one\ntwo\r\nthree', '??/\n\\\n', 'a)d)b)c', '\x00']
    groups['raw'] = [f'{p}R"{d}({b}){d}"{s}' for p, d, b, s in
                     itertools.product(prefixes, ['', 'd', 'abcdefghijklmnop'], raw_bodies, suffixes)]
    groups['unification'] = [f'{p}"\\x100"{s} /* gap */ {q}"π𐀀"{t}' for p, q, s, t in
                             itertools.product(prefixes, prefixes, suffixes, suffixes)]
    groups['operators'] = [f'operator ""{s} "after"' for s in suffixes + ['or', 'not', 'bitand', 'bitor', 'xor', 'compl', 'and_eq', 'or_eq', 'xor_eq', 'not_eq', 'new', 'delete']]
    groups['operators'] += ['operator ""\\\n_x', 'operator ""\\u03c0', 'operator ""operator ""and',
                            'operator u""suffix', 'operator R"()"suffix', 'operator "" ""sv',
                            'operator /* gap */ ""sv', 'operator ""sv ""sv']
    rng = random.Random(0x905bd7)
    groups['mixed-runs'] = []
    for _ in range(1200):
        pieces = []
        for _ in range(rng.randint(1, 8)):
            p, s = rng.choice(prefixes), rng.choice(suffixes)
            if rng.randrange(2):
                pieces.append(f'{p}"{rng.choice(bodies)}"{s}')
            else:
                pieces.append(f'{p}R"d({rng.choice(raw_bodies)})d"{s}')
        groups['mixed-runs'].append(rng.choice([' ', '\n', ' /* gap */ ', '\\\n']).join(pieces))
    return groups


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--evidence', type=Path, required=True)
    parser.add_argument('--profile', choices=['release', 'debug'], default='release')
    args = parser.parse_args()
    candidate = ROOT / 'obj/rust' / args.profile / 'posttoken'
    oracle = REFERENCE / 'dev/posttoken'
    pinned = json.loads((ROOT / 'scripts/clone/manifest.json').read_text())['oracle_tools']['posttoken']
    if digest(oracle) != pinned:
        raise SystemExit('oracle hash does not match the frozen manifest')
    root = args.evidence.resolve() / args.profile
    root.mkdir(parents=True, exist_ok=True)
    groups = corpus()
    # A splice can join a suffix to the next encoding prefix and make a later
    # quoted body lexically ill-formed. Run mixed sequences independently so a
    # source error in one cannot silently hide all remaining generated cases.
    inputs = [(name, (' ;\n'.join(values) + ' ;\n').encode(), ())
              for name, values in groups.items() if name != 'mixed-runs']
    inputs += [(f'mixed-run-{i:04}', value.encode(), ())
               for i, value in enumerate(groups['mixed-runs'])]
    probes = [
        b'1 "ok" /*', b'1 "ok" "\\q"', b'1 "ok" \\uD800', b'1 "ok" +\xff',
        b'operator ""sv /*', b'operator ""sv "\\q"', b'operator ""sv +\xff',
        b'"x"', b'"x"_x', b'"x" "y"', b'"x" "y"_x', b'"x" u"y"',
        b'"x" u"y" U"z"', b'"x"_x "y"_y', b'"x"#include <h>',
        b'"\\uD800"', b'"\\U00110000"', b'"\\x"', b"''", b'R"bad(unfinished',
        b'uR"d(\xff)d"', b'"x"\r\n"y"', b'"x"\x00',
        b'"x" "y"\\\n_suffix', b'operator ""\\\nsv',
    ]
    # Lexer construction is outside posttoken's located-error handler. Probe
    # initial decoding after BOM/splices as well as errors after ordinary input.
    probes += [prefix + bad for prefix in
               [b'', b'\xef\xbb\xbf', b'\\\n', b'??/\n', b' \n', b'1\n', b'"ok" ']
               for bad in [b'\xc0', b'\xff', b'\xc2', b'\xe0\x80', b'\xed\xa0\x80',
                           b'\xf4\x90\x80\x80', b'\xf5\x80\x80\x80']]
    inputs += [(f'probe-{i:02}', data, ()) for i, data in enumerate(probes)]
    inputs += [('batch-sentinel', b'one\n\nlast', ('--batch-stdin',))]
    inputs += [('frozen-' + path.stem, path.read_bytes(), ())
               for path in sorted((ROOT / 'pa2/tests').glob('*.t'))]
    rows = []
    for name, data, arguments in inputs:
        directory = root / name
        actual = invoke(candidate, data, directory / 'candidate', arguments)
        expected = invoke(oracle, data, directory / 'oracle', arguments)
        cases = 1 if name.startswith('mixed-run-') else len(groups.get(name, []))
        # Every grouped stream must reach EOF; a matching lexical failure only
        # proves the prefix through that error. Isolated inputs may reject.
        allowed = (0,) if name in groups else (0, 1)
        rows.append(dict(group=name, input_sha256=hashlib.sha256(data).hexdigest(), input_bytes=len(data),
                         arguments=arguments, candidate=actual, oracle=expected,
                         match=actual == expected and actual['exit'] in allowed, cases=cases))
    result = dict(candidate=str(candidate), candidate_sha256=digest(candidate),
                  oracle=str(oracle), oracle_sha256=digest(oracle), comparisons=rows,
                  case_counts={k: len(v) for k, v in groups.items()},
                  passed=sum(row['match'] for row in rows), total=len(rows))
    (root / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    for row in rows:
        print(row['group'], 'PASS' if row['match'] else 'FAIL', row['cases'])
    return 0 if result['passed'] == result['total'] else 1


if __name__ == '__main__':
    sys.exit(main())
