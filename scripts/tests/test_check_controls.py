#!/usr/bin/env python3
"""Check the self-checking controls lane (scripts/check_controls.pl) and its
Makefile routes with a stand-in frontend and reference backend."""

from pathlib import Path
import shutil
import os
from unittest.mock import patch
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts/check_controls.pl"

# The stand-in frontend "lowers" a source to the status its main returns and
# the text a `// prints:` line names; `REJECTED` in the source is an error,
# `INTERNAL_ERROR` stops the frontend with an internal compiler error (status
# 70), `CRASHES` makes the frontend abort and `ABORTS` makes the program abort.
FRONTEND = textwrap.dedent("""\
    #!/usr/bin/env python3
    import os, pathlib, re, sys
    args = sys.argv[1:]
    assert args[:2] == ['--emit-lowir', '-O0'], args
    source = pathlib.Path(args[-1]).read_text()
    if 'REJECTED' in source:
        sys.exit(1)
    if 'INTERNAL_ERROR' in source:
        sys.exit(70)
    if 'CRASHES' in source:
        os.abort()
    status = 'abort' if 'ABORTS' in source else re.search(r'return (\\d+);', source).group(1)
    printed = re.search(r'// prints: (.*)', source)
    out = pathlib.Path(args[args.index('-o') + 1])
    out.write_text(status + '\\n' + (printed.group(1) if printed else ''))
    """)
# The stand-in backend turns that "LowIR" into a program with that behavior.
BACKEND = textwrap.dedent("""\
    #!/usr/bin/env python3
    import pathlib, sys
    args = sys.argv[1:]
    assert args[0] == '-O0', args
    status, printed = pathlib.Path(args[-1]).read_text().split('\\n', 1)
    program = pathlib.Path(args[args.index('-o') + 1])
    ending = 'kill -ABRT $$' if status == 'abort' else f'exit {status}'
    program.write_text('#!/bin/sh\\n' + (f'printf "%s\\\\n" {printed!r}\\n' if printed else '') +
                       ending + '\\n')
    program.chmod(0o755)
    """)


class CheckControlsTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="check-controls.")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / "scripts").symlink_to(REPO_ROOT / "scripts")
        (self.root / "dev").symlink_to(REPO_ROOT / "dev")
        self.frontend = self.root / "cppgm++"
        self.frontend.write_text(FRONTEND)
        self.frontend.chmod(0o755)
        self.backend = self.root / "lowir2native-ref"
        self.backend.write_text(BACKEND)
        self.backend.chmod(0o755)
        self.controls = self.root / "pa13/tests/controls"
        self.controls.mkdir(parents=True)

    def control(self, name, text, **sidecars):
        path = self.controls / name
        path.write_text(textwrap.dedent(text))
        for suffix, data in sidecars.items():
            path.with_suffix("." + suffix).write_text(data)
        return path

    def check(self, *args):
        return subprocess.run(["perl", str(CHECKER), *map(str, args)],
                              capture_output=True, text=True, cwd=self.root)

    def run_checker(self, target):
        return self.check(self.frontend, self.backend, target)

    def test_directory_grades_stated_outcomes_and_skips_focused_controls(self):
        self.control("100-pass.t", "// N3485 x: rule; expect exit 7\nint main() { return 7; }\n")
        self.control("100-reject.t", "// N3485 x: ill-formed; must be rejected\nREJECTED\n")
        self.control("530-focused-property.t", "int main() { return 3; }\n")
        result = self.run_checker(self.controls)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("PASS (2/2)", result.stdout)

    def test_wrong_status_and_accepted_rejection_fail(self):
        self.control("100-wrong.t", "// expect exit 0\nint main() { return 1; }\n")
        self.control("100-accepted.t", "// expect reject\nint main() { return 0; }\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-wrong.t: FAIL: program exited with status 1, expected exit status 0",
                      result.stderr)
        self.assertIn("100-accepted.t: FAIL: frontend accepted", result.stderr)

    def test_a_signal_is_not_an_exit_status(self):
        # SIGABRT gives the shell's 134, but a program it kills did not exit 134.
        self.control("100-exits.t", "// expect exit 134\nint main() { return 134; }\n")
        self.assertEqual(self.run_checker(self.controls).returncode, 0)
        self.control("100-aborts.t", "// expect exit 134\nint main() { ABORTS; }\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-aborts.t: FAIL: program was killed by signal 6 (SIGABRT), "
                      "expected exit status 134", result.stderr)
        self.assertNotIn("100-exits.t", result.stderr)

    def test_a_crashing_frontend_does_not_reject(self):
        self.control("100-crash.t", "// expect reject\nCRASHES\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-crash.t: FAIL: frontend was killed by signal 6 (SIGABRT) "
                      "instead of rejecting", result.stderr)

    def test_an_internal_compiler_error_does_not_reject(self):
        self.control("100-internal.t", "// expect reject\nINTERNAL_ERROR\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-internal.t: FAIL: frontend stopped with an internal compiler "
                      "error (status 70) instead of rejecting", result.stderr)

    def test_sidecar_states_status_and_exact_output(self):
        self.control("100-printed.t", "// prints: hello\nint main() { return 3; }\n",
                     expect="exit 3\n", stdout="hello\n")
        self.assertEqual(self.run_checker(self.controls).returncode, 0)
        (self.controls / "100-printed.stdout").write_text("goodbye\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("output differs", result.stderr)

    def test_a_named_control_must_state_an_outcome(self):
        focused = self.control("530-focused-property.t", "int main() { return 3; }\n")
        stated = self.control("100-pass.t", "// expect exit 7\nint main() { return 7; }\n")
        self.assertNotEqual(self.run_checker(focused).returncode, 0)
        self.assertNotEqual(self.check("--owns", focused).returncode, 0)
        self.assertEqual(self.check("--owns", stated).returncode, 0)
        self.assertEqual(self.run_checker(stated).returncode, 0)

    def make(self, pa, *args):
        return subprocess.run(
            ["make", "-s", "-C", str(self.root / pa), *args, "RUN_CHECK_DEPS=", "TEST_DEPS=",
             f"CPPGM_TEST_APP={self.frontend}", f"NATIVE_REFERENCE_APP={self.backend}"],
            capture_output=True, text=True)

    def test_lowir_assignment_routes_controls_to_the_checker(self):
        shutil.copy(REPO_ROOT / "pa13/Makefile", self.root / "pa13/Makefile")
        self.control("100-pass.t", "// expect exit 7\nint main() { return 7; }\n")
        check = self.make("pa13", "check", "TEST=tests/controls/100-pass.t")
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        ref = self.make("pa13", "ref-test", "TEST=tests/controls/100-pass.t")
        self.assertIn("no reference sidecar", ref.stdout)
        self.assertEqual(self.make("pa13", "test-controls").returncode, 0)
        self.control("100-wrong.t", "// expect exit 0\nint main() { return 1; }\n")
        self.assertNotEqual(self.make("pa13", "test-controls").returncode, 0)

    def test_control_shape_counts_failures_in_quiet_reports(self):
        shutil.copy(REPO_ROOT / "pa13/Makefile", self.root / "pa13/Makefile")
        self.control("100-pass.t", "// expect exit 0\nint main() { return 0; }\n")
        self.control("100-reject.t", "// expect reject\nREJECTED\n")
        self.control("100-fail.t", "// expect exit 0\nint main() { return 1; }\n")
        counts = self.root / "counts.txt"
        with patch.dict(os.environ, {"KEEP_GOING": "1", "CPPGM_REPORT_QUIET": "1",
                                     "CPPGM_TEST_COUNTS_FILE": str(counts)}):
            result = self.make("pa13", "test-controls")
        self.assertEqual(counts.read_text(), "2 3\n")
        self.assertTrue((self.root / "pa13/.test_failed").exists())
        self.assertIn("pa13 tests/controls: FAIL (2/3)", result.stdout)
        self.assertIn("100-fail.t: ERROR: control predicate failed", result.stdout)

    def test_pa12_routes_stated_outcomes_away_from_its_focused_checker(self):
        pa12 = self.root / "pa12/tests/controls"
        pa12.mkdir(parents=True)
        shutil.copy(REPO_ROOT / "pa12/Makefile", self.root / "pa12/Makefile")
        (pa12 / "100-pass.t").write_text("// expect exit 7\nint main() { return 7; }\n")
        check = self.make("pa12", "check", "TEST=tests/controls/100-pass.t")
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        (pa12 / "100-unstated.t").write_text("int main() { return 7; }\n")
        focused = self.make("pa12", "check", "TEST=tests/controls/100-unstated.t")
        self.assertNotEqual(focused.returncode, 0)
        self.assertIn("No control predicate owns", focused.stdout + focused.stderr)

    def test_object_extents_keep_direct_and_indirect_requirements_in_their_owners(self):
        # The stand-ins isolate checker predicates; the course controls also
        # execute through the real frontend/backend in test-controls.
        self.frontend.write_text(textwrap.dedent("""\
            #!/usr/bin/env python3
            import pathlib, sys
            args = sys.argv[1:]
            pathlib.Path(args[args.index('-o') + 1]).write_text(pathlib.Path(args[-1]).read_text())
            """))
        self.backend.write_text(textwrap.dedent("""\
            #!/usr/bin/env python3
            import pathlib, sys
            args = sys.argv[1:]
            program = pathlib.Path(args[args.index('-o') + 1])
            program.write_text('#!/bin/sh\\nexit 0\\n')
            program.chmod(0o755)
            """))
        lowir = (
            "function @result(%out : ptr [pass=indirect_result, object_bytes=24]) -> void {}\n"
            "function @reference(%value : ptr [pass=by_address, object_bytes=24]) -> i64 {}\n"
            "function @member(%this : ptr [object_bytes=24]) -> i64 {}\n"
            "function @unused(%this : ptr [object_bytes=24]) -> i64 {}\n"
            "function @pointer_value(%value : ptr) -> i64 {}\n"
            "function @main() -> i32 {\n  %answer = call i64 @member(%object)\n}\n"
        )
        direct = self.root / "pa12/tests/controls/200-parameter-object-extent-boundary.t"
        indirect = self.root / "pa22/tests/controls/300-member-pointer-object-extent-boundary.t"
        for pa, control in [("pa12", direct), ("pa22", indirect)]:
            control.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO_ROOT / pa / "Makefile", self.root / pa / "Makefile")
            control.write_text(lowir)
        self.assertEqual(self.make("pa12", "check", f"TEST=tests/controls/{direct.name}").returncode, 0)
        missing = self.make("pa22", "check", f"TEST=tests/controls/{indirect.name}")
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("indirect member call lost its object extent", missing.stdout + missing.stderr)
        indirect.write_text(lowir.replace("  %answer", "  %indirect = call i64 %callee(ptr [object_bytes=24] %object)\n  %answer"))
        self.assertEqual(self.make("pa22", "check", f"TEST=tests/controls/{indirect.name}").returncode, 0)
        direct.write_text(lowir.replace("@member(%this : ptr [object_bytes=24])", "@member(%this : ptr)"))
        missing = self.make("pa12", "check", f"TEST=tests/controls/{direct.name}")
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("direct member call lost its object extent", missing.stdout + missing.stderr)


if __name__ == "__main__":
    unittest.main()
