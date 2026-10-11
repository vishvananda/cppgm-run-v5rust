#!/usr/bin/env python3
"""Check scripts/check_fixture_lanes.py: every tracked fixture input of the
repository sits in a lane its assignment's Makefile runs, and the check finds
the inputs, strays and helpers that do not on small stand-in assignments."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CHECKER = REPO_ROOT / "scripts/check_fixture_lanes.py"


class RepositoryTests(unittest.TestCase):
    def test_every_tracked_fixture_input_is_run(self):
        result = subprocess.run([sys.executable, str(CHECKER)],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


class StandInTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="fixture-lanes.")
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        (self.root / "scripts").symlink_to(REPO_ROOT / "scripts")
        (self.root / "dev").symlink_to(REPO_ROOT / "dev")

    def write(self, relative, text=""):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def assignment(self, pa, makefile, runner="run_all_tests.pl"):
        self.write(f"{pa}/Makefile", textwrap.dedent(makefile))
        self.write(f"{pa}/scripts/{runner}", "#!/usr/bin/perl\n")

    def check(self, *pas):
        subprocess.run(["git", "init", "-q", str(self.root)], check=True)
        subprocess.run(["git", "-C", str(self.root), "add", "-A"], check=True)
        result = subprocess.run([sys.executable, str(CHECKER), "--root",
                                 str(self.root), *pas],
                                capture_output=True, text=True)
        problems = [line.split(":", 1)[0] for line in result.stdout.splitlines()
                    if not line.startswith("fixture lanes:")]
        return result.returncode, problems, result.stdout

    def test_pa9_fixture_beside_its_only_root(self):
        # PA9's own Makefile runs and regenerates tests/abi alone.
        (self.root / "pa9").mkdir()
        shutil.copy(REPO_ROOT / "pa9/Makefile", self.root / "pa9/Makefile")
        (self.root / "pa9/scripts").symlink_to(REPO_ROOT / "pa9/scripts")
        for case in ("tests/abi/200-typeinfo-name", "tests/700-structured-thunk"):
            self.write(f"pa9/{case}.t", "function f\n")
            self.write(f"pa9/{case}.ref", "_Z1fv\n")
            self.write(f"pa9/{case}.ref.exit_status", "EXIT_SUCCESS\n")
        status, problems, output = self.check()
        self.assertEqual(status, 1, output)
        self.assertEqual(problems, ["pa9/tests/700-structured-thunk.t"], output)
        self.assertIn("test lane roots: tests/abi", output)

    def test_regression_and_controls_are_lanes_of_their_own(self):
        self.assignment("pa20", """\
            test:
            \t@scripts/run_all_tests.pl app my tests
            \t@if [ -d "tests/controls" ]; then perl ../scripts/check_controls.pl app ref "tests/controls"; fi
            ref-test:
            \t@scripts/run_all_tests.pl app-ref ref tests
            """)
        self.write("pa20/tests/general/100-run.t")
        self.write("pa20/tests/regression/100-pinned.t")
        self.write("pa20/tests/controls/100-stated.cpp", "// expect exit 0\nint main() {}\n")
        self.write("pa20/tests/controls/100-silent.cpp", "int main() {}\n")
        self.write("pa20/tests/controls/100-sidecar.cpp", "int main() {}\n")
        self.write("pa20/tests/controls/100-sidecar.expect", "exit 0\n")
        status, problems, output = self.check()
        self.assertEqual(status, 1, output)
        self.assertEqual(sorted(problems), ["pa20/tests/controls/100-silent.cpp",
                                            "pa20/tests/regression/100-pinned.t"], output)

    def test_regeneration_alone_does_not_run_a_fixture(self):
        self.assignment("pa21", """\
            test:
            \t@scripts/run_all_tests.pl app my tests/general
            \t@scripts/compare_results.pl ref my tests/general tests/spec
            ref-test:
            \t@set -e; for root in tests/general tests/spec; do scripts/run_all_tests.pl ref-app ref "$$root"; done
            """)
        self.write("pa21/tests/general/100-run.t")
        self.write("pa21/tests/spec/100-regenerated.t")
        status, problems, output = self.check()
        self.assertEqual(problems, ["pa21/tests/spec/100-regenerated.t"], output)
        self.assertIn("ref-test regenerates its references, but no test lane runs it",
                      output)

    def test_focused_checker_selects_by_its_collect_pattern(self):
        self.assignment("pa22", """\
            test:
            \t@perl scripts/check_focus.pl app tests/controls
            """, runner="check_focus.pl")
        self.write("pa22/scripts/check_focus.pl",
                   "my @tests = collect_tests($root, qr/(?:inline-limit).*\\.t$/);\n")
        self.write("pa22/tests/controls/100-inline-limit-cap.t")
        self.write("pa22/tests/controls/100-unclaimed.t")
        status, problems, output = self.check()
        self.assertEqual(problems, ["pa22/tests/controls/100-unclaimed.t"], output)

    def test_unknown_runner_is_reported(self):
        self.assignment("pa23", """\
            test:
            \t@perl scripts/mystery.pl app tests
            """, runner="mystery.pl")
        self.write("pa23/tests/100-run.t")
        status, problems, output = self.check()
        self.assertEqual(status, 1, output)
        self.assertIn("cannot tell what pa23/scripts/mystery.pl selects", output)

    def test_companions_helpers_and_strays(self):
        self.assignment("pa25", """\
            test:
            \t@scripts/run_all_tests.pl app my tests
            """)
        for name in ("100-multi.t", "100-multi.t.1", "100-multi.t2", "100-multi.ref",
                     "100-multi.ref.program.stdout", "100-multi.lib.helper.c",
                     "100-multi.inc/tool.h", "100-multi.env"):
            self.write(f"pa25/tests/general/{name}", '#include "support.h"\n')
        self.write("pa25/tests/support.h", '#include "detail.h"\n')
        self.write("pa25/tests/detail.h")
        self.write("pa25/tests/unused.h", '#include "orphan.h"\n')
        self.write("pa25/tests/orphan.h")
        self.write("pa25/tests/general/README.md", "notes\n")
        self.write("pa25/tests/general/110-renumbered")
        self.write("pa25/tests/general/120-removed.expect", "exit 0\n")
        status, problems, output = self.check()
        self.assertEqual(sorted(problems), [
            "pa25/tests/general/110-renumbered",
            "pa25/tests/general/120-removed.expect",
            "pa25/tests/orphan.h",
            "pa25/tests/unused.h",
        ], output)

    def test_external_lane_must_name_its_inputs(self):
        self.write("pa35/Makefile", "test:\n\tperl scripts/check.pl\n")
        self.write("pa35/scripts/behavior.pl", 'glob("$PA35/tests/behavior/*.cpp")\n')
        self.write("pa35/scripts/generated.pl", "narrow_main.cpp\n")
        self.write("pa35/tests/behavior/loops.cpp")
        self.write("pa35/tests/byte_values/narrow_main.cpp")
        self.write("pa35/tests/generators/unnamed.pl")
        status, problems, output = self.check()
        self.assertEqual(problems, ["pa35/tests/generators/unnamed.pl"], output)


if __name__ == "__main__":
    unittest.main()
