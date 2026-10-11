import contextlib
import importlib.util
import io
import shutil
from pathlib import Path
import re
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock


REPO_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = REPO_ROOT / "scripts" / "validate_perf_regression.py"
SPEC = importlib.util.spec_from_file_location("validate_perf_regression", SCRIPT_PATH)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class ValidatePerfRegressionTest(unittest.TestCase):
    @staticmethod
    def perf_report(rss, instructions=100, footprint=100, head="candidate"):
        return {
            "head": head,
            "command": MODULE.DEFAULT_COMMAND,
            "summary": {
                "instructions_retired": {"median": instructions},
                "maximum_resident_set_size": {"median": rss},
                "peak_memory_footprint": {"median": footprint},
            },
        }

    def test_default_workload_uses_frozen_project_headers(self):
        include_dir = "benchmarks/self_compile/stable/include"
        self.assertIn("-I", MODULE.DEFAULT_COMMAND)
        include_index = MODULE.DEFAULT_COMMAND.index("-I")
        self.assertEqual(MODULE.DEFAULT_COMMAND[include_index + 1], include_dir)
        self.assertNotIn("dev/src", MODULE.DEFAULT_COMMAND)

    def test_frozen_project_include_closure_is_complete(self):
        include_dir = REPO_ROOT / "benchmarks/self_compile/stable/include"
        source = REPO_ROOT / "benchmarks/self_compile/stable/semantic_overload.cpp"
        project_files = [source, *sorted(include_dir.iterdir())]
        include_pattern = re.compile(r'^\s*#\s*include\s+"([^"]+)"')

        missing = []
        for path in project_files:
            if not path.is_file():
                continue
            for line in path.read_text().splitlines():
                match = include_pattern.match(line)
                if match and not (include_dir / match.group(1)).is_file():
                    missing.append(f"{path.name}: {match.group(1)}")

        self.assertEqual(missing, [])

    def test_frozen_manifest_matches_source_and_all_headers(self):
        identity = MODULE.validate_frozen_workload(
            REPO_ROOT, MODULE.DEFAULT_COMMAND
        )
        self.assertEqual(
            identity["epoch_commit"],
            "9764b3835e3c6996b6b80803054f80e1cf50f98e",
        )
        self.assertEqual(identity["header_count"], 51)
        self.assertEqual(
            identity["header_closure_sha256"],
            "7c8a5445f33f04b314de98e6a099de4d75124b4bb032fc97ee5055e56d4827c8",
        )

    def test_frozen_manifest_rejects_header_content_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            source_root = REPO_ROOT / "benchmarks/self_compile/stable"
            target_root = temporary_root / "benchmarks/self_compile/stable"
            shutil.copytree(source_root, target_root)
            changed = target_root / "include/semantic_model.h"
            changed.write_text(changed.read_text() + "\n// drift\n")

            with self.assertRaisesRegex(
                MODULE.FrozenWorkloadError,
                "frozen header digests differ: semantic_model.h",
            ):
                MODULE.validate_frozen_workload(
                    temporary_root, MODULE.DEFAULT_COMMAND
                )

    def test_frozen_manifest_rejects_header_membership_drift(self):
        with tempfile.TemporaryDirectory() as temporary:
            temporary_root = Path(temporary)
            source_root = REPO_ROOT / "benchmarks/self_compile/stable"
            target_root = temporary_root / "benchmarks/self_compile/stable"
            shutil.copytree(source_root, target_root)
            (target_root / "include/extra.h").write_text("// extra\n")

            with self.assertRaisesRegex(
                MODULE.FrozenWorkloadError,
                "extra=extra.h",
            ):
                MODULE.validate_frozen_workload(
                    temporary_root, MODULE.DEFAULT_COMMAND
                )

    def test_perf_gate_rejects_live_or_modified_workload_commands(self):
        live_headers = [
            "./dev/cppgm++",
            "-I",
            "dev/src",
            "-c",
            "-o",
            "/tmp/candidate.o",
            "benchmarks/self_compile/stable/semantic_overload.cpp",
        ]
        with self.assertRaisesRegex(
            MODULE.FrozenWorkloadError,
            "command is not the frozen semantic-overload workload",
        ):
            MODULE.validate_frozen_workload(REPO_ROOT, live_headers)

    def test_workload_comparison_ignores_only_output_path(self):
        baseline = ["./dev/cppgm++", "-I", "frozen", "-c", "-o", "/tmp/a.o", "input.cpp"]
        candidate = ["./dev/cppgm++", "-I", "frozen", "-c", "-o", "/tmp/b.o", "input.cpp"]
        changed_headers = [
            "./dev/cppgm++", "-I", "dev/src", "-c", "-o", "/tmp/b.o", "input.cpp"
        ]

        self.assertEqual(
            MODULE.normalized_workload_command(baseline),
            MODULE.normalized_workload_command(candidate),
        )
        self.assertNotEqual(
            MODULE.normalized_workload_command(baseline),
            MODULE.normalized_workload_command(changed_headers),
        )

        summary = {
            key: {"median": 1}
            for key in (
                "instructions_retired",
                "maximum_resident_set_size",
                "peak_memory_footprint",
            )
        }
        args = SimpleNamespace(
            instruction_tolerance=0.01,
            rss_tolerance=0.03,
            footprint_tolerance=0.03,
        )
        with contextlib.redirect_stdout(io.StringIO()):
            failures = MODULE.compare_reports(
                {"command": baseline, "summary": summary},
                {"command": changed_headers, "summary": summary},
                args,
            )
        self.assertTrue(
            any("benchmark workload command differs" in failure for failure in failures)
        )

    def test_epoch_head_legacy_baseline_remains_compatible(self):
        identity = MODULE.validate_frozen_workload(
            REPO_ROOT, MODULE.DEFAULT_COMMAND
        )
        summary = {
            key: {"median": 1}
            for key in (
                "instructions_retired",
                "maximum_resident_set_size",
                "peak_memory_footprint",
            )
        }
        args = SimpleNamespace(
            instruction_tolerance=0.01,
            rss_tolerance=0.03,
            footprint_tolerance=0.03,
        )
        baseline = {
            "head": identity["epoch_commit"],
            "command": MODULE.DEFAULT_COMMAND,
            "summary": summary,
        }
        candidate = {
            "head": "candidate",
            "command": MODULE.DEFAULT_COMMAND,
            "workload": identity,
            "summary": summary,
        }
        with contextlib.redirect_stdout(io.StringIO()):
            failures = MODULE.compare_reports(baseline, candidate, args)
        self.assertEqual(failures, [])

        baseline["head"] = "not-the-frozen-epoch"
        with contextlib.redirect_stdout(io.StringIO()):
            failures = MODULE.compare_reports(baseline, candidate, args)
        self.assertTrue(
            any("lacks frozen workload identity" in failure for failure in failures)
        )

    def test_rss_threshold_warns_without_failing_initial_comparison(self):
        args = SimpleNamespace(
            instruction_tolerance=0.005,
            rss_tolerance=0.03,
            footprint_tolerance=0.01,
        )
        warnings = []
        with contextlib.redirect_stdout(io.StringIO()):
            failures = MODULE.compare_reports(
                self.perf_report(100, head="baseline"),
                self.perf_report(103),
                args,
                rss_exceedance="warn",
                warnings=warnings,
            )
        self.assertEqual(failures, [])
        self.assertEqual(len(warnings), 1)
        self.assertIn("warning threshold", warnings[0])

    def test_rss_warning_runs_one_confirmation_batch(self):
        args = SimpleNamespace(
            repo_root=REPO_ROOT,
            runs=3,
            timeout_sec=0,
            time_binary="/usr/bin/time",
            command=[],
            baseline="baseline.json",
            report=None,
            instruction_tolerance=0.005,
            rss_tolerance=0.03,
            footprint_tolerance=0.01,
        )
        baseline = self.perf_report(100, head="baseline")
        first = self.perf_report(104, head="candidate")
        confirmation = self.perf_report(102, head="candidate")
        with (
            mock.patch.object(MODULE, "validate_frozen_workload", return_value={}),
            mock.patch.object(MODULE, "load_json", return_value=baseline),
            mock.patch.object(
                MODULE, "collect_runs", side_effect=[["first"], ["confirmation"]]
            ) as collect_runs,
            mock.patch.object(
                MODULE, "make_report", side_effect=[first, confirmation]
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            status = MODULE.command_check(args)
        self.assertEqual(status, 0)
        self.assertEqual(collect_runs.call_count, 2)

    def test_second_rss_warning_fails_confirmation(self):
        args = SimpleNamespace(
            repo_root=REPO_ROOT,
            runs=3,
            timeout_sec=0,
            time_binary="/usr/bin/time",
            command=[],
            baseline="baseline.json",
            report=None,
            instruction_tolerance=0.005,
            rss_tolerance=0.03,
            footprint_tolerance=0.01,
        )
        baseline = self.perf_report(100, head="baseline")
        first = self.perf_report(104, head="candidate")
        confirmation = self.perf_report(105, head="candidate")
        with (
            mock.patch.object(MODULE, "validate_frozen_workload", return_value={}),
            mock.patch.object(MODULE, "load_json", return_value=baseline),
            mock.patch.object(
                MODULE, "collect_runs", side_effect=[["first"], ["confirmation"]]
            ),
            mock.patch.object(
                MODULE, "make_report", side_effect=[first, confirmation]
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            status = MODULE.command_check(args)
        self.assertEqual(status, 1)

    def test_compare_advisory_reports_metric_deviation_without_failing(self):
        args = SimpleNamespace(
            baseline="baseline.json",
            candidate="candidate.json",
            report=None,
            advisory=True,
            instruction_tolerance=0.005,
            rss_tolerance=0.03,
            footprint_tolerance=0.01,
        )
        baseline = self.perf_report(
            100, instructions=100, footprint=100, head="baseline"
        )
        candidate = self.perf_report(104, instructions=102, footprint=102)
        with (
            mock.patch.object(
                MODULE, "load_json", side_effect=[baseline, candidate]
            ),
            contextlib.redirect_stdout(io.StringIO()) as output,
        ):
            status = MODULE.command_compare(args)
        self.assertEqual(status, 0)
        self.assertIn("advisory performance deviations", output.getvalue())

    def test_compare_advisory_still_rejects_workload_identity_mismatch(self):
        args = SimpleNamespace(
            baseline="baseline.json",
            candidate="candidate.json",
            report=None,
            advisory=True,
            instruction_tolerance=0.005,
            rss_tolerance=0.03,
            footprint_tolerance=0.01,
        )
        baseline = self.perf_report(100, head="baseline")
        candidate = self.perf_report(100)
        candidate["command"] = ["different"]
        with (
            mock.patch.object(
                MODULE, "load_json", side_effect=[baseline, candidate]
            ),
            contextlib.redirect_stdout(io.StringIO()),
        ):
            status = MODULE.command_compare(args)
        self.assertEqual(status, 1)

    def test_compare_parser_has_no_execution_arguments(self):
        args = MODULE.build_parser().parse_args(
            [
                "compare",
                "--baseline",
                "baseline.json",
                "--candidate",
                "candidate.json",
                "--advisory",
            ]
        )
        self.assertIs(args.func, MODULE.command_compare)
        self.assertTrue(args.advisory)
        self.assertFalse(hasattr(args, "runs"))

    def test_parser_defaults_match_project_gate_policy(self):
        parser = MODULE.build_parser()
        check = parser.parse_args(["check", "--baseline", "baseline.json"])
        self.assertEqual(check.runs, 3)
        self.assertEqual(check.instruction_tolerance, 0.005)
        self.assertEqual(check.rss_tolerance, 0.03)
        self.assertEqual(check.footprint_tolerance, 0.01)

        compare = parser.parse_args(
            [
                "compare",
                "--baseline",
                "baseline.json",
                "--candidate",
                "candidate.json",
            ]
        )
        self.assertEqual(compare.instruction_tolerance, 0.005)
        self.assertEqual(compare.rss_tolerance, 0.03)
        self.assertEqual(compare.footprint_tolerance, 0.01)


if __name__ == "__main__":
    unittest.main()
