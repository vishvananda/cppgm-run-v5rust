#!/usr/bin/env python3
"""Check PA8's actual routes with CLI-only producers and the native backend."""

import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
NATIVE = REPO_ROOT / "dev/lowir2native"


@unittest.skipUnless(NATIVE.is_file(), "build lowir2native to run execution harness tests")
class LowirProgramHarnessTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="lowir-program-harness.")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.pa = self.root / "pa8"
        self.pa.mkdir()
        (self.root / "scripts").symlink_to(REPO_ROOT / "scripts")
        (self.root / "dev").symlink_to(REPO_ROOT / "dev")
        shutil.copy(REPO_ROOT / "pa8/Makefile", self.pa / "Makefile")
        (self.pa / "scripts").mkdir()
        for name in ("run_all_tests.pl", "compare_results.pl"):
            shutil.copy(REPO_ROOT / "pa8/scripts" / name, self.pa / "scripts" / name)
        self.producer = self.root / "producer"
        self.producer.write_text(textwrap.dedent("""\
            #!/usr/bin/env python3
            import os, pathlib, sys
            args = sys.argv[1:]
            output = pathlib.Path(args[args.index('-o') + 1])
            if '--exercise' not in args:
                data = pathlib.Path(args[-1]).read_text()
                variant = os.environ.get('SPEC_VARIANT', 'good')
                if variant == 'whitespace':
                    data = data.replace('return i64 0', 'return i64 0x0').replace('\\n', '\\n  ')
                if variant == 'drop_declaration':
                    data = data.split('\\n', 1)[1]
                if variant == 'change_value':
                    data = data.replace('return i64 0', 'return i64 1')
                output.write_text(data)
                sys.exit(0)
            assert args[args.index('--exercise') + 1] == 'sum', args
            variant = 'good' if sys.argv[0].endswith('-ref') else os.environ.get('VARIANT', 'good')
            if variant == 'failed':
                sys.exit(2)
            bodies = {
                'good': 'return i64 3',
                'alternative': '%other = binary add i64 1, 2\\n    return i64 %other',
                'wrong': 'return i64 4',
            }
            if variant == 'malformed':
                output.write_text('not a LowIR program')
            else:
                output.write_text('function @answer() -> i64 {\\n  block ^start:\\n    ' + bodies[variant] + '\\n}\\n')
            """))
        self.producer.chmod(0o755)
        self.reference = self.root / "producer-ref"
        shutil.copy(self.producer, self.reference)
        self.backend_called = self.root / "backend-called"
        self.backend = self.root / "native-ref"
        self.backend.write_text(
            "#!/usr/bin/env python3\nimport os, pathlib, sys\n"
            f"pathlib.Path({str(self.backend_called)!r}).write_text('called')\n"
            f"os.execv({str(NATIVE)!r}, [{str(NATIVE)!r}, *sys.argv[1:]])\n"
        )
        self.backend.chmod(0o755)
        spec = self.pa / "tests/spec/100-case"
        spec.parent.mkdir(parents=True)
        source = ("declare function @unused(%x : i64) -> i64\n\n"
                  "function @helper() -> i64 {\n  block ^entry:\n    return i64 0\n}\n")
        spec.with_suffix(".t").write_text(source)
        spec.with_suffix(".ref").write_text(source)
        spec.with_suffix(".ref.exit_status").write_text("EXIT_SUCCESS\n")
        self.base = self.pa / "tests/behavior/100-case"
        self.base.parent.mkdir(parents=True)
        self.base.with_suffix(".t").write_text(textwrap.dedent("""\
            function @main() -> i64 [role=entry] {
              block ^entry:
                %value = call i64 @answer()
                %bad = cmp ne i64 %value, 3
                %status = convert zext i64 i1 %bad
                return i64 %status
            }
            """))
        self.base.with_suffix(".exercise").write_text("sum\n")
        self.base.with_suffix(".ref.impl.exit_status").write_text("0\n")
        self.base.with_suffix(".ref.program.exit_status").write_text("0\n")
        self.base.with_suffix(".ref.program.stdout").write_text("")

    def make(self, *args, variant="good", spec_variant="good", direct=True):
        env = os.environ.copy()
        for name in ("KEEP_GOING", "CPPGM_APP_ARGS", "CPPGM_CHECK_MODE", "MAKEFLAGS"):
            env.pop(name, None)
        env.update(CPPGM_BATCH_TESTS="0", CPPGM_TEST_RUNNER="0", CPPGM_TEST_JOBS="1",
                   CPPGM_LOWIR_DIRECT_TEXT_COMPARE="1" if direct else "0",
                   VARIANT=variant, SPEC_VARIANT=spec_variant)
        return subprocess.run(
            ["make", "--no-print-directory", "TEST_DEPS=",
             f"CPPGM_TEST_APP={self.producer}", f"REF_TEST_APP={self.reference}",
             f"NATIVE_REFERENCE_APP={self.backend}", *args],
            cwd=self.pa, env=env, text=True, capture_output=True, timeout=30,
        )

    def assertPassed(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_default_suite_executes_and_rejects_wrong_program(self):
        good = self.make("test")
        self.assertPassed(good)
        self.assertIn("tests/spec/100-case.t: PASS (1/1)", good.stdout)
        self.assertIn("tests/behavior: PASS (1/1)", good.stdout)
        bad = self.make("test", variant="wrong")
        self.assertNotEqual(bad.returncode, 0, bad.stdout + bad.stderr)
        self.assertIn("program", bad.stdout.lower())

    def test_different_valid_lowir_passes_without_reference_ir(self):
        self.assertPassed(self.make("test", variant="alternative"))
        self.assertFalse(self.base.with_suffix(".ref").exists())
        self.assertFalse(self.base.with_suffix(".ref.mir").exists())
        self.assertFalse(self.base.with_suffix(".my.mir").exists())
        generated = self.base.with_suffix(".my.lowir").read_text()
        self.assertIn("binary add", generated)

    def test_roundtrip_allows_formatting_but_preserves_values_and_declarations(self):
        self.assertPassed(self.make("test", spec_variant="whitespace", direct=False))
        for variant in ("drop_declaration", "change_value"):
            with self.subTest(variant=variant):
                result = self.make("test", spec_variant=variant, direct=False)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertIn("roundtrip changed", result.stdout)

    def test_malformed_lowir_and_failed_producer_cannot_reuse_stale_program(self):
        for variant in ("malformed", "failed"):
            with self.subTest(variant=variant):
                self.assertPassed(self.make("test"))
                self.backend_called.unlink()
                result = self.make("test", variant=variant)
                self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertFalse(self.base.with_suffix(".my.program").exists())
                self.assertFalse(self.base.with_suffix(".my.program.exit_status").exists())
                self.assertEqual(self.backend_called.exists(), variant == "malformed")

    def test_mixed_check_and_reference_generation_select_the_right_producer(self):
        spec = "tests/spec/*.t tests/behavior/*.t"
        self.assertPassed(self.make("check", f"TEST={spec}"))
        for reference in (self.pa / "tests").rglob("*.ref*"):
            reference.unlink()
        self.assertPassed(self.make("ref-test", variant="wrong"))
        self.assertTrue((self.pa / "tests/spec/100-case.ref.exit_status").exists())
        self.assertEqual(self.base.with_suffix(".ref.program.exit_status").read_text().strip(), "0")
        self.assertPassed(self.make("check", f"TEST={spec}"))

    def test_missing_reference_backend_fails_without_fallback(self):
        result = self.make("test", "NATIVE_REFERENCE_APP=/missing/lowir2native-ref")
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("/missing/lowir2native-ref", result.stdout + result.stderr)
        self.assertFalse(self.backend_called.exists())

    def test_pa12_controls_need_only_a_source_to_lowir_compiler(self):
        self.pa = self.root / "pa12"
        self.pa.mkdir()
        shutil.copy(REPO_ROOT / "pa12/Makefile", self.pa / "Makefile")
        control = "tests/controls/200-static-reference-lifetime.t"
        (self.pa / control).parent.mkdir(parents=True)
        shutil.copy(REPO_ROOT / "pa12" / control, self.pa / control)
        compiler = REPO_ROOT / "dev/cppgm++"
        self.producer.write_text(
            "#!/usr/bin/env python3\nimport os, sys\n"
            "assert '--emit-lowir' in sys.argv, 'native driver is not implemented'\n"
            f"os.execv({str(compiler)!r}, [{str(compiler)!r}, *sys.argv[1:]])\n"
        )
        self.assertPassed(self.make("check", "RUN_CHECK_DEPS=", f"TEST={control}"))
        self.assertTrue(self.backend_called.exists())
        self.backend_called.unlink()
        missing = self.make("check", "RUN_CHECK_DEPS=", f"TEST={control}",
                            "NATIVE_REFERENCE_APP=/missing/lowir2native-ref")
        self.assertNotEqual(missing.returncode, 0, missing.stdout + missing.stderr)
        self.assertFalse(self.backend_called.exists())


if __name__ == "__main__":
    unittest.main()
