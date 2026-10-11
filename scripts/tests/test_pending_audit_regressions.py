#!/usr/bin/env python3
"""Check that tests-first controls cannot bless crashes or wrong runtime results."""

import json
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/check_pending_audit_regressions.py"


class PendingAuditRegressionTests(unittest.TestCase):
    def test_only_diagnostic_rejection_satisfies_negative_control(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            source = directory / "negative.cpp"
            source.write_text("// AUDIT-EXPECT: reject\nint invalid;\n")
            for code, passed in ((0, False), (1, True), (2, False),
                                 (86, False), (124, False)):
                with self.subTest(exit_status=code):
                    compiler = directory / "compiler"
                    compiler.write_text(f"#!/bin/sh\nexit {code}\n")
                    compiler.chmod(0o755)
                    report = directory / "report.json"
                    result = subprocess.run(
                        ["python3", str(SCRIPT), "--compiler",
                         shlex.quote(str(compiler)), "--optimization", "O0",
                         "--report", str(report), str(source)],
                        capture_output=True, text=True, timeout=30)
                    self.assertEqual(result.returncode, 0 if passed else 1,
                                     result.stderr)
                    self.assertEqual(json.loads(report.read_text())[0]["passed"],
                                     passed)
                    if passed:
                        self.assertEqual(result.stdout,
                                         "Audit regression controls: 1/1 passed\n")

    def test_compiler_signal_is_a_failure_even_for_a_negative(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            source = directory / "negative.cpp"
            source.write_text("// AUDIT-EXPECT: reject\nint invalid;\n")
            compiler = directory / "compiler"
            compiler.write_text("#!/bin/sh\nkill -SEGV $$\n")
            compiler.chmod(0o755)
            result = subprocess.run(
                ["python3", str(SCRIPT), "--compiler", shlex.quote(str(compiler)),
                 "--optimization", "O0", str(source)],
                capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn("ERROR:", result.stdout)

    def test_run_control_checks_runtime_after_successful_compilation(self):
        compiler = shutil.which("g++") or shutil.which("clang++")
        if compiler is None:
            self.skipTest("host C++ compiler unavailable")
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            for code in (0, 5):
                with self.subTest(program_exit=code):
                    source = directory / "runtime.cpp"
                    source.write_text("// AUDIT-EXPECT: run\n"
                                      f"int main(){{return {code};}}\n")
                    report = directory / "report.json"
                    result = subprocess.run(
                        ["python3", str(SCRIPT), "--compiler", compiler,
                         "--host-cxx", compiler, "--host-oracle",
                         "--optimization", "O0", "--report", str(report),
                         str(source)], capture_output=True, text=True, timeout=30)
                    self.assertEqual(result.returncode, 0 if code == 0 else 1,
                                     result.stderr)
                    row = json.loads(report.read_text())[0]
                    self.assertEqual([obs["status"] for obs in row["observations"]],
                                     [0, 0, code])

    def test_successful_encoder_with_wrong_name_still_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            control = directory / "name.audit"
            control.write_text("input pa9/tests/abi/300-function-owner-member-pointer-nttp-data.t\n"
                               "expected _ZN2ns6HolderIXadL_ZN1C1mEEEE1fERS1_\n")
            encoder = directory / "encoder"
            encoder.write_text("#!/bin/sh\nprintf 'wrong-name\\n' > \"$2\"\n")
            encoder.chmod(0o755)
            result = subprocess.run(
                ["python3", str(SCRIPT), "--abimangle", shlex.quote(str(encoder)),
                 str(control)], capture_output=True, text=True, timeout=30)
            self.assertEqual(result.returncode, 1, result.stderr)
            self.assertIn("wrong-name", result.stdout)


if __name__ == "__main__":
    unittest.main()
