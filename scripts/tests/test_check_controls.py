#!/usr/bin/env python3
"""Check the self-checking controls lane (scripts/check_controls.pl) and its
Makefile routes with a stand-in frontend and reference backend."""

from pathlib import Path
import shutil
import subprocess
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts/check_controls.pl"

# The stand-in frontend "lowers" a source to the status its main returns and
# the text a `// prints:` line names; `REJECTED` in the source is an error,
# `CRASHES` makes the frontend abort and `ABORTS` makes the program abort.
FRONTEND = textwrap.dedent("""\
    #!/usr/bin/env python3
    import os, pathlib, re, sys
    args = sys.argv[1:]
    assert args[:2] == ['--emit-lowir', '-O0'], args
    source = pathlib.Path(args[-1]).read_text()
    if 'REJECTED' in source:
        sys.exit(1)
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
        self.control("100-pass.cpp", "// N3485 x: rule; expect exit 7\nint main() { return 7; }\n")
        self.control("100-reject.cpp", "// N3485 x: ill-formed; must be rejected\nREJECTED\n")
        self.control("530-focused-property.cpp", "int main() { return 3; }\n")
        result = self.run_checker(self.controls)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("PASS (2/2)", result.stdout)

    def test_wrong_status_and_accepted_rejection_fail(self):
        self.control("100-wrong.cpp", "// expect exit 0\nint main() { return 1; }\n")
        self.control("100-accepted.cpp", "// expect reject\nint main() { return 0; }\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-wrong.cpp: FAIL: program exited with status 1, expected exit status 0",
                      result.stderr)
        self.assertIn("100-accepted.cpp: FAIL: frontend accepted", result.stderr)

    def test_a_signal_is_not_an_exit_status(self):
        # SIGABRT gives the shell's 134, but a program it kills did not exit 134.
        self.control("100-exits.cpp", "// expect exit 134\nint main() { return 134; }\n")
        self.assertEqual(self.run_checker(self.controls).returncode, 0)
        self.control("100-aborts.cpp", "// expect exit 134\nint main() { ABORTS; }\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-aborts.cpp: FAIL: program was killed by signal 6 (SIGABRT), "
                      "expected exit status 134", result.stderr)
        self.assertNotIn("100-exits.cpp", result.stderr)

    def test_a_crashing_frontend_does_not_reject(self):
        self.control("100-crash.cpp", "// expect reject\nCRASHES\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("100-crash.cpp: FAIL: frontend was killed by signal 6 (SIGABRT) "
                      "instead of rejecting", result.stderr)

    def test_sidecar_states_status_and_exact_output(self):
        self.control("100-printed.cpp", "// prints: hello\nint main() { return 3; }\n",
                     expect="exit 3\n", stdout="hello\n")
        self.assertEqual(self.run_checker(self.controls).returncode, 0)
        (self.controls / "100-printed.stdout").write_text("goodbye\n")
        result = self.run_checker(self.controls)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("output differs", result.stderr)

    def test_a_named_control_must_state_an_outcome(self):
        focused = self.control("530-focused-property.cpp", "int main() { return 3; }\n")
        stated = self.control("100-pass.cpp", "// expect exit 7\nint main() { return 7; }\n")
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
        self.control("100-pass.cpp", "// expect exit 7\nint main() { return 7; }\n")
        check = self.make("pa13", "check", "TEST=tests/controls/100-pass.cpp")
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        ref = self.make("pa13", "ref-test", "TEST=tests/controls/100-pass.cpp")
        self.assertIn("no reference sidecar", ref.stdout)
        self.assertEqual(self.make("pa13", "test-controls").returncode, 0)
        self.control("100-wrong.cpp", "// expect exit 0\nint main() { return 1; }\n")
        self.assertNotEqual(self.make("pa13", "test-controls").returncode, 0)

    def test_pa12_routes_stated_outcomes_away_from_its_focused_checker(self):
        pa12 = self.root / "pa12/tests/controls"
        pa12.mkdir(parents=True)
        shutil.copy(REPO_ROOT / "pa12/Makefile", self.root / "pa12/Makefile")
        (pa12 / "100-pass.cpp").write_text("// expect exit 7\nint main() { return 7; }\n")
        check = self.make("pa12", "check", "TEST=tests/controls/100-pass.cpp")
        self.assertEqual(check.returncode, 0, check.stdout + check.stderr)
        (pa12 / "100-unstated.cpp").write_text("int main() { return 7; }\n")
        focused = self.make("pa12", "check", "TEST=tests/controls/100-unstated.cpp")
        self.assertNotEqual(focused.returncode, 0)
        self.assertIn("No PA12 survivor-property tests found", focused.stderr)


if __name__ == "__main__":
    unittest.main()
