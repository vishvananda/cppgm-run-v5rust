#!/usr/bin/env python3
"""PA33 accepts alternative code and rejects lost effects, bounds and locations."""
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
COMPARE = ROOT / 'pa33/scripts/compare_results.pl'
DRIVER = ROOT / 'scripts/check_cppgm_native_programs.pl'


class Pa33CourseTests(unittest.TestCase):
    def compare(self, directory, mir, expectation=None, status='0', stdout='ok\n'):
        base = directory / 'case'
        base.with_suffix('.t').write_text('input\n')
        for suffix in ('ref', 'my'):
            base.with_suffix(f'.{suffix}.impl.exit_status').write_text('0\n')
            base.with_suffix(f'.{suffix}.program.exit_status').write_text(
                status if suffix == 'my' else '0')
            base.with_suffix(f'.{suffix}.program.stdout').write_text(
                stdout if suffix == 'my' else 'ok\n')
        base.with_suffix('.ref.mir').write_text('different reference instructions\n')
        base.with_suffix('.ref.cmir').write_text('different reference layout\n')
        if mir is not None:
            base.with_suffix('.my.mir').write_text(mir)
        if expectation:
            base.with_suffix('.ref.expect').write_text(expectation)
        return subprocess.run(['perl', str(COMPARE), 'ref', 'my', str(directory)],
                              capture_output=True, text=True)

    def test_course_uses_bounds_and_execution_without_matching_reference(self):
        mir = 'machine_ir x86_64 linux\nfunction @main\n  block ^new_label\n    ret 0\n'
        cases = [
            (mir, None, '0', 'ok\n', True),
            (mir, 'instructions <= 1\n', '0', 'ok\n', True),
            (mir.replace('ret 0', 'mov r11, 0\n    ret r11'),
             'instructions <= 1\n', '0', 'ok\n', False),
            (mir, None, '1', 'ok\n', False),
            (mir, None, '0', 'wrong\n', False),
            (None, None, '0', 'ok\n', False),
        ]
        for args in cases:
            with self.subTest(args=args), tempfile.TemporaryDirectory() as temporary:
                result = self.compare(Path(temporary), *args[:-1])
                self.assertEqual(result.returncode == 0, args[-1], result.stdout + result.stderr)

    def test_every_quality_and_debug_fixture_has_an_explicit_contract(self):
        for bucket in ('o1', 'o2', 'o3', 'debuginfo'):
            for fixture in (ROOT / 'pa33/tests' / bucket).rglob('*.t'):
                self.assertTrue(fixture.with_suffix('.ref.expect').is_file(), str(fixture))

    def test_debug_contract_allows_folded_code_but_rejects_lost_or_wrong_locations(self):
        fixture = ROOT / 'pa33/tests/debuginfo/o1/100-return-copy-coalesce-debug'
        expectation = fixture.with_suffix('.ref.expect').read_text()
        # Fold the entire computation and change the block and instruction layout.
        prefix = 'machine_ir x86_64 linux\nfunction @main\n  block ^folded\n    '
        for body, valid in [
            ('ret 3 !dbg(test.cpp, 4, 1)\n', True),
            ('ret 3\n', False),
            ('ret 3 !dbg(test.cpp, 3, 1)\n', False),
            ('ret 3 !dbg(other.cpp, 4, 1)\n', False),
        ]:
            with self.subTest(body=body), tempfile.TemporaryDirectory() as temporary:
                result = self.compare(Path(temporary), prefix + body, expectation)
                self.assertEqual(result.returncode == 0, valid, result.stdout + result.stderr)

    def test_debug_contract_checks_surviving_call_locations(self):
        base = ROOT / 'pa33/tests/debuginfo/o2/200-edge-placement-debug'
        mir = base.with_suffix('.ref.mir').read_text()
        expectation = base.with_suffix('.ref.expect').read_text()
        # Changing all chosen registers is accepted; moving a surviving call's
        # location to an unrelated statement in the same function is rejected.
        mir = mir.replace('r15', 'r12')
        for replacement, valid in [('11, 5', True), ('7, 5', False)]:
            changed = re.sub(r'(call @identity[^\n]*!dbg\(test.cpp, )11, 5',
                             lambda m: m[1] + replacement, mir)
            with self.subTest(location=replacement), tempfile.TemporaryDirectory() as temporary:
                result = self.compare(Path(temporary), changed, expectation)
                self.assertEqual(result.returncode == 0, valid, result.stdout + result.stderr)

    def test_driver_checks_all_levels_and_both_object_formats(self):
        with tempfile.TemporaryDirectory(prefix='pa33-driver-c.obj-') as temporary:
            directory = Path(temporary)
            fixture = directory / 'case.t'
            fixture.write_text('int main() { return 0; }\n')
            compiler = directory / 'compiler'
            compiler.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
out = pathlib.Path(args[args.index('-o') + 1])
with open(os.environ['PA33_CALL_LOG'], 'a') as log:
    log.write(json.dumps(args) + '\\n')
if '--emit-lowir' in args:
    out.write_text(os.environ.get('PA33_LOWIR_BODY', 'serialized-lowir'))
elif '-c' in args:
    out.write_text('object')
else:
    out.write_text('#!/bin/sh\\n' + os.environ.get('PA33_PROGRAM_BODY', 'exit 0') + '\\n')
    out.chmod(0o755)
''')
            compiler.chmod(0o755)
            log = directory / 'calls'
            env = dict(os.environ, CPPGM_HOST_CXX=str(compiler), PA33_CALL_LOG=str(log))
            command = ['perl', str(DRIVER), str(compiler), str(fixture)]
            result = subprocess.run(command, env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('PASS (9/9)', result.stdout)
            calls = [json.loads(line) for line in log.read_text().splitlines()]
            for level in ('-O1', '-O2', '-O3'):
                self.assertEqual(sum(args[0] == level for args in calls), 4)
            outputs = [Path(args[args.index('-o') + 1]).suffix
                       for args in calls if '-c' in args]
            self.assertCountEqual(outputs, ['.obj', '.o'] * 3)
            # Use a separate backend and verify it consumes the compiler's
            # LowIR, including rejection of malformed producer output.
            backend = directory / 'backend'
            backend_log = directory / 'backend-calls'
            backend.write_text('''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
with open(os.environ['PA33_NATIVE_LOG'], 'a') as log:
    log.write(json.dumps(args) + '\\n')
if pathlib.Path(args[-1]).read_text() != 'serialized-lowir':
    sys.exit('invalid producer LowIR')
out = pathlib.Path(args[args.index('-o') + 1])
out.write_text('#!/bin/sh\\nexit 0\\n')
out.chmod(0o755)
''')
            backend.chmod(0o755)
            serialized_env = dict(env, PA33_NATIVE_LOG=str(backend_log))
            serialized = command + [str(backend)]
            result = subprocess.run(serialized, env=serialized_env,
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            self.assertIn('PASS (12/12)', result.stdout)
            backend_calls = [json.loads(line) for line in backend_log.read_text().splitlines()]
            self.assertEqual([args[0] for args in backend_calls], ['-O1', '-O2', '-O3'])
            self.assertTrue(all(Path(args[-1]).suffix == '.lowir' for args in backend_calls))
            result = subprocess.run(serialized,
                                    env=dict(serialized_env, CPPGM_REPORT_QUIET='1'),
                                    capture_output=True, text=True)
            self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
            result = subprocess.run(serialized,
                                    env=dict(serialized_env, PA33_LOWIR_BODY='invalid'),
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('lowir: compilation/link failed', result.stderr)
            self.assertIn('invalid producer LowIR', result.stderr)
            for body in ('exit 1', 'echo wrong'):
                result = subprocess.run(command, env=dict(env, PA33_PROGRAM_BODY=body),
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == '__main__':
    unittest.main()
