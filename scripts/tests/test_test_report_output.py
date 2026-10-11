#!/usr/bin/env python3
"""The report keeps successful child logs quiet and exposes failed checks."""

from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]


class TestReportOutputTests(unittest.TestCase):
    def report(self, *, failure=None, ordered=True, target="test-report-nobuild",
               exported=False, low_jobs=False, real_control=False, later_lane=False):
        with tempfile.TemporaryDirectory(prefix="test-report-output.") as directory:
            root = Path(directory)
            shutil.copy(ROOT / "Makefile", root / "Makefile")
            if exported:
                script = (ROOT / "scripts/export_student_repo.sh").read_text()
                start = script.index("sanitize_student_root_makefile() {")
                end = script.index("\n}\n", start) + len("\n}\n")
                subprocess.run(
                    ["bash", "-euc", script[start:end] +
                     '\nsanitize_student_root_makefile "$1"', "bash", str(root / "Makefile")],
                    check=True,
                )
                makefile = (root / "Makefile").read_text()
                self.assertNotIn("scripts/audit_", makefile)
                self.assertNotIn("test-seams", makefile)
                self.assertNotIn("test-harness", makefile)
                self.assertNotIn("test-cells", makefile)
            (root / "scripts").mkdir()
            audit = root / "scripts/audit_compiler_exceptions.pl"
            audit.write_text(
                'if ("' + str(failure) + '" eq "audit") { '
                'print STDERR "audit failure diagnostic\\n"; exit 1; }\n'
                'print "audit success inventory\\n" unless grep { $_ eq "--quiet" } @ARGV;\n'
            )
            (root / "dev").mkdir()
            (root / "dev/Makefile").write_text(
                "all:\n\t@echo build success detail\n" +
                ("\t@echo build failure diagnostic >&2; exit 1\n"
                 if failure == "build" else "")
            )
            (root / "pa11/tests/general").mkdir(parents=True)
            control_recipe = ""
            if real_control:
                source = root / "pa11/tests/general/100-unreachable-terminator.cpp"
                source.write_text("// The producer supplies the control's LowIR.\n")
                producer = root / "producer"
                producer.write_text(
                    "#!/usr/bin/env python3\nimport pathlib, sys\n"
                    "output = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\n"
                    "output.write_text(" + repr(
                        "function @guarded_value(%x : i1) -> i32 [object=_Z13guarded_valueb] {\n"
                        "  block ^entry:\n    " +
                        ("return i32 0" if failure == "control" else "unreachable") +
                        "\n}\n") + ")\n"
                )
                producer.chmod(0o755)
                control_recipe = ("\t@perl " + str(ROOT / "scripts/check_pa11_unreachable_terminator.pl") +
                                  " " + str(producer) + " tests/general\n")
            (root / "pa11/Makefile").write_text(
                "test:\n"
                "\t@if [ \"$$CPPGM_REPORT_QUIET\" != 1 ]; then echo suite success detail; "
                "echo maintainer success detail >&2; fi\n" +
                control_recipe + "\t@echo '2 2' >> ../.test_counts\n" +
                ("\t@echo suite failure diagnostic; exit 1\n" if failure == "suite" else "") +
                ("\t@echo later lane diagnostic; echo '3 3' >> ../.test_counts; "
                 "touch .test_failed\n" if later_lane else "") +
                ("\t@echo comparison failure diagnostic; touch .test_failed\n"
                 if failure == "comparison" else "") +
                "test-seams:\n"
                "\t@echo 'expected rewrite ERROR: intentionally rejected'\n" +
                ("\t@echo seams failure diagnostic >&2; exit 1\n" if failure == "seams" else "")
            )
            environment = os.environ.copy()
            # Each scratch report starts a fresh top-level make, even when
            # the harness itself was launched by make.
            for variable in ("MAKEFLAGS", "MFLAGS", "MAKELEVEL", "MAKEOVERRIDES"):
                environment.pop(variable, None)
            result = subprocess.run(
                ["make", *(["-j1"] if low_jobs else []), target,
                 "DEFAULT_BUILD_JOBS=" + ("2" if low_jobs else "1"),
                 "ACTIVE_TEST_REPORT_PAS=pa11",
                 "TEST_REPORT_ASSIGNMENT_JOBS=1", "TEST_REPORT_STALL_SEC=0",
                 "ORDERED=" + ("true" if ordered else "false")],
                cwd=root, env=environment, capture_output=True, text=True, timeout=30,
            )
            return result

    def test_success_prints_only_total_in_both_orders(self):
        for ordered in (True, False):
            with self.subTest(ordered=ordered):
                result = self.report(ordered=ordered)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(result.stdout,
                                 "===== ALL TESTS PASSED SUCCESSFULLY! (2 / 2) =====\n")
                self.assertEqual(result.stderr, "")

    def test_report_build_is_quiet_on_success(self):
        for target in ("test-report", "test-report-through-pa11"):
            with self.subTest(target=target):
                result = self.report(target=target)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assertEqual(result.stdout,
                                 "===== ALL TESTS PASSED SUCCESSFULLY! (2 / 2) =====\n")
                self.assertEqual(result.stderr, "")

    def test_exported_report_has_no_maintainer_dependencies(self):
        result = self.report(exported=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout,
                         "===== ALL TESTS PASSED SUCCESSFULLY! (2 / 2) =====\n")
        self.assertEqual(result.stderr, "")

    def test_report_with_low_job_limit_has_no_success_warning(self):
        result = self.report(low_jobs=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(result.stdout,
                         "===== ALL TESTS PASSED SUCCESSFULLY! (2 / 2) =====\n")
        self.assertEqual(result.stderr, "")

    def test_real_successful_control_stays_quiet_beside_a_failure(self):
        for exported in (False, True):
            for ordered in (True, False):
                with self.subTest(exported=exported, ordered=ordered):
                    result = self.report(failure="comparison", real_control=True,
                                         exported=exported, ordered=ordered)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("comparison failure diagnostic", result.stdout)
                    self.assertNotIn("properties: PASS", result.stdout + result.stderr)
                    self.assertNotIn("success detail", result.stdout + result.stderr)

    def test_real_control_failure_remains_visible_in_quiet_mode(self):
        result = self.report(failure="control", real_control=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("__builtin_unreachable did not lower to the `unreachable` terminator",
                      result.stdout + result.stderr)
        self.assertNotIn("properties: PASS", result.stdout + result.stderr)

    def test_failing_lane_does_not_hide_the_lanes_after_it(self):
        for exported in (False, True):
            for ordered in (True, False):
                with self.subTest(exported=exported, ordered=ordered):
                    result = self.report(failure="suite", later_lane=True,
                                         exported=exported, ordered=ordered)
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn("suite failure diagnostic", result.stdout)
                    self.assertIn("later lane diagnostic", result.stdout)
                    self.assertIn("TEST SUMMARY: 5 / 5 TESTS PASSED", result.stdout)
                    self.assertNotIn("ALL TESTS PASSED", result.stdout)

    def test_failed_children_expose_diagnostics(self):
        for failure in ("audit", "build", "suite", "comparison", "seams"):
            for ordered in (True, False):
                with self.subTest(failure=failure, ordered=ordered):
                    result = self.report(failure=failure, ordered=ordered,
                                         target="test-report" if failure == "build"
                                         else "test-report-nobuild")
                    self.assertNotEqual(result.returncode, 0)
                    self.assertIn(failure + " failure diagnostic", result.stdout + result.stderr)
                    self.assertNotIn("ALL TESTS PASSED", result.stdout)
                    if failure != "seams":
                        self.assertNotIn("expected rewrite ERROR", result.stdout)
                    if failure in ("suite", "comparison"):
                        self.assertNotIn("success detail", result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
