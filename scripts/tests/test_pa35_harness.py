#!/usr/bin/env python3
"""Check the PA35 harness without the course compiler: the gate table and
the README's statement of its limits and ladder steps, the progress ladder
(progress.pl), the release check's margins in both builds (calibrate.pl
--from), the frozen workload's manifest, and the behavior and generated
runners driven with the host compiler standing in for cppgm++."""

from pathlib import Path
import os
import re
import shutil
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
PA35 = REPO_ROOT / "pa35"
SCRIPTS = PA35 / "scripts"
CALIBRATE = PA35 / "maintainer/calibrate.pl"


def read_gates():
    gates = []
    for line in (PA35 / "gates.txt").read_text().splitlines():
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        fields = line.split()
        gate = dict(zip(("group", "name", "metric", "op", "limit"), fields[:5]))
        gate["limit"] = float(gate["limit"])
        gate["from"] = float(fields[5]) if len(fields) > 5 else None
        gate["step"] = float(fields[6]) if len(fields) > 6 else None
        gates.append(gate)
    return gates


def rungs(gate):
    """progress.pl's rung thresholds: limit x (1 + step)^k up to `from`."""
    if gate["op"] != "<=" or not gate["from"] or not gate["step"]:
        return [gate["limit"]]
    thresholds = [gate["limit"]]
    while thresholds[-1] < gate["from"] * (1 - 1e-12):
        thresholds.append(thresholds[-1] * (1 + gate["step"]))
    return thresholds


def write_values(directory, values, gates, groups=("correctness", "speed", "codegen")):
    for group in groups:
        rows = "".join(f"{g['metric']}\t{values[g['metric']]}\n" for g in gates if g["group"] == group)
        (directory / f"values-{group}.tsv").write_text(rows)


def passing_values(gates):
    """Every gate at its limit (==) or well inside it (<=)."""
    return {g["metric"]: (int(g["limit"]) if g["op"] == "==" else g["limit"] / 1.5) for g in gates}


class GateTableTests(unittest.TestCase):
    def test_seventeen_gates_in_three_ordered_groups(self):
        gates = read_gates()
        self.assertEqual(len(gates), 17)
        order = [g["group"] for g in gates]
        self.assertEqual(sorted(set(order), key=order.index), ["correctness", "speed", "codegen"])
        self.assertEqual(order, sorted(order, key=["correctness", "speed", "codegen"].index))
        self.assertEqual(len({g["name"] for g in gates}), 17)
        self.assertEqual(len({g["metric"] for g in gates}), 17)

    def test_every_measured_gate_has_a_ladder(self):
        for gate in read_gates():
            if gate["op"] == "==":
                self.assertIsNone(gate["from"], gate["name"])
                continue
            self.assertEqual(gate["op"], "<=", gate["name"])
            self.assertGreater(gate["from"], gate["limit"], gate["name"])
            self.assertGreater(gate["step"], 0, gate["name"])
            self.assertLessEqual(gate["step"], 0.05, gate["name"])


class SpeedModeTests(unittest.TestCase):
    def test_both_compilers_receive_matching_optimization_levels(self):
        # Exercise speed.pl's actual invocation path, timing helper and table
        # writer with cheap compiler stand-ins. No real workload compile.
        import json
        with tempfile.TemporaryDirectory(prefix="pa35-speed-modes.") as directory:
            root = Path(directory)
            calls = root / "calls.jsonl"
            for name in ("host", "candidate"):
                compiler = root / name
                compiler.write_text("#!/usr/bin/env python3\n" +
                    "import json, os, pathlib, sys\n" +
                    "with open(os.environ['PA35_TEST_CALLS'], 'a') as stream:\n" +
                    "    stream.write(json.dumps([pathlib.Path(sys.argv[0]).name, sys.argv[1:]]) + '\\n')\n" +
                    "pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(b'output')\n")
                compiler.chmod(0o755)
            result = subprocess.run(["perl", str(SCRIPTS / "speed.pl"),
                "--compiler", str(root / "candidate"), "--host", str(root / "host"),
                "--rounds", "2", "--out", str(root / "speed.tsv")],
                env=dict(os.environ, PA35_TEST_CALLS=str(calls)),
                capture_output=True, text=True, timeout=60)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            rows = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual(len(rows), 12)
            for who in ("host", "candidate"):
                levels = [next(arg for arg in args if arg in ("-E", "-O0", "-O2", "-O3"))
                          for name, args in rows if name == who]
                self.assertEqual(levels, ["-E", "-O0", "-O3"] * 2)
            self.assertIn("host -O3", result.stdout)
            self.assertIn("time-O3\t", (root / "speed.tsv").read_text())


class ProgressTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="pa35-progress.")
        self.addCleanup(temp.cleanup)
        self.dir = Path(temp.name)
        self.gates = read_gates()

    def progress(self):
        return subprocess.run(["perl", str(SCRIPTS / "progress.pl"), "--out", str(self.dir)],
                              capture_output=True, text=True)

    def test_nothing_measured_passes_no_rung(self):
        result = self.progress()
        self.assertEqual(result.returncode, 1)
        total = sum(len(rungs(g)) for g in self.gates)
        self.assertIn(f"===== TEST SUMMARY: 0 / {total} TESTS PASSED =====", result.stdout)

    def test_all_gates_pass_every_rung(self):
        write_values(self.dir, passing_values(self.gates), self.gates)
        result = self.progress()
        self.assertEqual(result.returncode, 0, result.stdout)
        total = sum(len(rungs(g)) for g in self.gates)
        self.assertIn(f"===== ALL TESTS PASSED SUCCESSFULLY! ({total}/{total}) =====", result.stdout)

    def test_a_value_between_rungs_passes_the_rungs_above_it(self):
        values = passing_values(self.gates)
        gate = next(g for g in self.gates if g["metric"] == "O0-own")
        values["O0-own"] = gate["limit"] * 1.2
        write_values(self.dir, values, self.gates)
        result = self.progress()
        self.assertEqual(result.returncode, 1)
        expected = sum(1 for t in rungs(gate) if values["O0-own"] <= t * (1 + 1e-12))
        self.assertIn(f"{gate['name']}: FAIL ({expected}/{len(rungs(gate))})", result.stdout)

    def test_a_group_older_than_an_earlier_group_is_not_counted(self):
        write_values(self.dir, passing_values(self.gates), self.gates)
        codegen = self.dir / "values-codegen.tsv"
        speed = self.dir / "values-speed.tsv"
        os.utime(codegen, (speed.stat().st_mtime - 100,) * 2)
        result = self.progress()
        self.assertEqual(result.returncode, 1)
        self.assertIn("measured before the earlier groups", result.stdout)
        for gate in self.gates:
            if gate["group"] == "codegen":
                self.assertIn(f"{gate['name']}: FAIL (0/{len(rungs(gate))})", result.stdout)


class CalibrateTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="pa35-calibrate.")
        self.addCleanup(temp.cleanup)
        self.dir = Path(temp.name)
        self.gates = read_gates()
        self.out = self.dir / "calibration.tsv"
        self.course_out = self.dir / "calibration-course.tsv"

    def calibrate(self, values, student=None, student_groups=("correctness", "speed", "codegen")):
        """Runs calibrate.pl on a course build with `values` and a student
        build with `student` (default: the same), both measured in every
        group unless `student_groups` leaves some out."""
        for build, given in (("course", values), ("student", student or values)):
            (self.dir / build).mkdir(exist_ok=True)
            groups = ("correctness", "speed", "codegen") if build == "course" else student_groups
            write_values(self.dir / build, given, self.gates, groups)
        return subprocess.run(["perl", str(CALIBRATE), "--from", str(self.dir), "--out", str(self.out),
                               "--course-out", str(self.course_out)],
                              capture_output=True, text=True)

    def gate_rows(self):
        """gate name -> (course status, student status), from the two files."""
        rows = {}
        for build, path in (("course", self.course_out), ("student", self.out)):
            for line in path.read_text().splitlines():
                if line.startswith("gate\t"):
                    fields = line.split("\t")
                    rows.setdefault(fields[1], {})[build] = fields[7]
        return {name: (status["course"], status["student"]) for name, status in rows.items()}

    def test_values_well_inside_the_limits_pass(self):
        result = self.calibrate(passing_values(self.gates))
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        rows = self.gate_rows()
        self.assertEqual(len(rows), 17)
        self.assertTrue(all(row == ("pass", "pass") for row in rows.values()), rows)
        self.assertIn("RELEASE CHECK PASSED (17/17)", result.stdout)
        text = self.out.read_text()
        for key in ("date", "source", "host compiler", "libstdc++", "build"):
            self.assertRegex(text, rf"(?m)^meta\t{re.escape(key)}\t\S")
        # The shipped file names the machine generically: no distribution
        # package strings, and the course build's flags only in its own file.
        self.assertNotRegex(text, r"(?m)^meta\tkernel\t.*-\w+\d+-")
        self.assertNotIn("jemalloc", text)

    def test_a_value_inside_the_limit_but_not_the_margin_is_short(self):
        values = passing_values(self.gates)
        gate = next(g for g in self.gates if g["metric"] == "O0-proxy")
        values["O0-proxy"] = gate["limit"] / 1.001
        result = self.calibrate(values)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.gate_rows()[gate["name"]], ("SHORT", "SHORT"))
        self.assertIn("RELEASE CHECK FAILED: 1 of 17 gates", result.stdout)

    def test_the_student_build_must_clear_the_margins_too(self):
        values = passing_values(self.gates)
        student = dict(values)
        gate = next(g for g in self.gates if g["metric"] == "O3-proxy")
        student["O3-proxy"] = gate["limit"] / 1.001
        result = self.calibrate(values, student)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.gate_rows()[gate["name"]], ("pass", "SHORT"))
        self.assertIn("RELEASE CHECK FAILED: 1 of 17 gates", result.stdout)
        student["O3-proxy"] = gate["limit"] * 1.001
        result = self.calibrate(values, student)
        self.assertEqual(result.returncode, 1)
        self.assertEqual(self.gate_rows()[gate["name"]], ("pass", "FAIL"))

    def test_each_kind_needs_its_margin_in_either_build(self):
        margins = {"O0-own": 0.02, "kernel-lookup": 0.02, "O3-proxy": 0.015, "text": 0.01,
                   "rss-O3": 0.25, "time-O0": 0.25, "scaling": 0.25}
        for metric, margin in margins.items():
            gate = next(g for g in self.gates if g["metric"] == metric)
            for build in ("course", "student"):
                for factor, status in ((1.001, "pass"), (0.999, "SHORT")):
                    values = passing_values(self.gates)
                    student = dict(values)
                    (values if build == "course" else student)[metric] = gate["limit"] / (1 + margin * factor)
                    result = self.calibrate(values, student)
                    row = self.gate_rows()[gate["name"]]
                    self.assertEqual(row[0 if build == "course" else 1], status, (metric, build, factor))
                    self.assertEqual(result.returncode, 0 if status == "pass" else 1, (metric, build, factor))

    def test_the_student_build_uses_the_exported_makefile_flags(self):
        export = (REPO_ROOT / "scripts/export_student_repo.sh").read_text()
        heredoc = export[export.index("cat > \"$dest/dev/Makefile\""):]
        exported = re.search(r"^CC_FLAGS \?= (.*?) \$\(CPPGM_STDLIB_FLAGS\)$", heredoc, re.M).group(1)
        self.assertNotIn("HOST_ALLOC_LIBS", heredoc[:heredoc.index("\nEOF\n")])
        calibrate = CALIBRATE.read_text()
        self.assertIn(f"'CC_FLAGS={exported}'", calibrate)

    def test_a_failing_or_missing_gate_fails(self):
        values = passing_values(self.gates)
        values["behavior-failures"] = 1
        values["text"] = "missing"
        result = self.calibrate(values)
        self.assertEqual(result.returncode, 1)
        rows = self.gate_rows()
        self.assertEqual(rows["behavior"][0], "FAIL")
        self.assertEqual(rows["text-size"][0], "FAIL")

    def test_the_student_build_measures_its_own_correctness(self):
        values = passing_values(self.gates)
        result = self.calibrate(values, student_groups=("speed", "codegen"))
        self.assertEqual(result.returncode, 1)
        rows = self.gate_rows()
        for gate in ("workload-compiles", "behavior", "generated"):
            self.assertEqual(rows[gate], ("pass", "FAIL"), gate)
        self.assertIn("RELEASE CHECK FAILED: 3 of 17 gates", result.stdout)


class ReadmeTests(unittest.TestCase):
    """The handout states the limits and ladder steps gates.txt holds."""

    def setUp(self):
        self.readme = (PA35 / "README.md").read_text()
        self.gates = read_gates()

    def test_the_gate_table_states_each_limit(self):
        stated = {}
        for name, limit in re.findall(r"^\| \w+ \| `([\w*-]+)`(?: \(\d+\))? \|.*\| <= ([\d.]+)(?: each)? \|$",
                                      self.readme, re.M):
            stated[name] = float(limit)
        for gate in self.gates:
            if gate["op"] != "<=":
                continue
            name = "kernel-*" if gate["name"].startswith("kernel-") else gate["name"]
            self.assertIn(name, stated, gate["name"])
            self.assertEqual(stated[name], gate["limit"], gate["name"])

    def test_the_ladder_steps_are_the_ones_in_gates_txt(self):
        paragraph = re.search(r"steps follow measurement noise \(([^)]*)\)", " ".join(self.readme.split()))
        self.assertIsNotNone(paragraph)
        stated = {float(p) / 100 for p in re.findall(r"([\d.]+)%", paragraph.group(1))}
        self.assertEqual(stated, {g["step"] for g in self.gates if g["step"]})


class MeasurementHelperTests(unittest.TestCase):
    """PA35.pm's helpers, driven from perl one-liners."""

    def perl(self, code):
        result = subprocess.run(["perl", f"-I{SCRIPTS}", "-MPA35", "-e", code], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def test_a_child_ended_by_a_signal_is_not_a_success(self):
        out = self.perl('my ($st) = PA35::run_capture("sh", "-c", "kill -9 \\$\\$"); print $st')
        self.assertEqual(out, "137")
        out = self.perl('my ($st) = PA35::run_capture("sh", "-c", "exit 3"); print $st')
        self.assertEqual(out, "3")
        out = self.perl('my ($st) = PA35::run_capture("sh", "-c", "true"); print $st')
        self.assertEqual(out, "0")

    def test_a_size_that_timed_out_is_unbounded_growth(self):
        growth = lambda times: self.perl(
            'print PA35::scaling_growth([' + ",".join(f'"{t}"' for t in times) + '], 0.1)')
        self.assertEqual(growth([0.01, 0.20, 0.40, "timeout"]), "inf")
        self.assertEqual(growth([0.01, 0.20, "failed"]), "failed")
        self.assertEqual(growth([0.01, 0.20, 0.40]), "2.05")
        self.assertEqual(growth([0.01, 0.02, 0.03]), "flat")
        self.assertEqual(growth(["timeout"]), "base timeout")

    def test_the_self_build_keeps_the_objects_layout_below_their_common_directory(self):
        out = self.perl('print PA35::common_directory("../obj/dev/a.o", "../obj/dev/sub/b.o", "../obj/dev/c.o")')
        self.assertEqual(out, "../obj/dev/")
        out = self.perl('print PA35::common_directory("../obj/alt/dev/a.o", "../obj/alt/dev/sub/b.o")')
        self.assertEqual(out, "../obj/alt/dev/")


class WorkloadAndRunnerTests(unittest.TestCase):
    def test_scripts_compile(self):
        for script in sorted(SCRIPTS.glob("*.pl")) + [CALIBRATE]:
            result = subprocess.run(["perl", "-c", str(script)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, f"{script}: {result.stderr}")

    def test_workload_matches_its_manifest(self):
        result = subprocess.run(["perl", f"-I{SCRIPTS}", "-MPA35", "-e", "print PA35::verify_workload()"],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stderr)
        listed = len((PA35 / "workload/MANIFEST.sha256").read_text().splitlines())
        self.assertEqual(result.stdout, str(listed))

    @unittest.skipUnless(shutil.which("g++"), "needs g++")
    def test_behavior_and_generated_runners_with_the_host_compiler(self):
        host = shutil.which("g++")
        programs = sorted((PA35 / "tests/behavior").glob("*.cpp"))[:2]
        result = subprocess.run(["perl", str(SCRIPTS / "behavior.pl"), "--compiler", host, "--levels", "O0,O2",
                                 "--jobs", "2"] + [str(p) for p in programs],
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("behavior: 2/2 tests pass at O0 O2", result.stdout)
        result = subprocess.run(["perl", str(SCRIPTS / "generated.pl"), "--compiler", host, "--seeds", "1",
                                 "--levels", "O1", "--jobs", "2"], capture_output=True, text=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("generated: 5/5 programs pass at O1", result.stdout)


if __name__ == "__main__":
    unittest.main()


class ParityReportTests(unittest.TestCase):
    """parity.pl --from renders the report from a run's measurements: the
    direct ratio of each compiler's own figure, the host-relative figures
    beside it, and an explicit, nonzero-status row for a missing figure."""

    def report(self, reference_speed, our_speed, summary_tsv, summary_txt):
        temp = tempfile.TemporaryDirectory(prefix="pa35-parity.")
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        (root / "ab" / "measure").mkdir(parents=True)
        (root / "speed-reference.tsv").write_text("metric\tcppgm\thost\tratio\n" + reference_speed)
        (root / "speed-yours.tsv").write_text("metric\tcppgm\thost\tratio\n" + our_speed)
        (root / "ab" / "measure" / "summary.tsv").write_text("level\tmetric\tseed\tself\tratio\n" + summary_tsv)
        (root / "ab" / "measure" / "summary.txt").write_text(summary_txt)
        result = subprocess.run(["perl", str(PA35 / "scripts/parity.pl"), "--from", str(root)],
                                capture_output=True, text=True)
        rows = {}
        for line in (root / "parity.tsv").read_text().splitlines()[1:]:
            name, a, b, ratio = line.split("\t")
            rows[name] = (a, b, ratio)
        return result, rows

    SUMMARY = "O0\ttotal\t1000\t1100\t1.100000\nO0\tidentical\t1\t1\t1\n"
    TEXT = "text bytes: old 5000, new 5500 (1.100x)\n"

    def test_equal_compile_times_are_a_ratio_of_one_whatever_the_host_ran(self):
        result, rows = self.report(
            "time-O0\t1.000000\t10.000000\t0.100000\nrss-O0\t100\t200\t0.5\n"
            "time-O3\t2.000000\t20.000000\t0.100000\nrss-O3\t100\t400\t0.25\n",
            "time-O0\t1.000000\t5.000000\t0.200000\nrss-O0\t100\t200\t0.5\n"
            "time-O3\t2.000000\t10.000000\t0.200000\nrss-O3\t100\t400\t0.25\n",
            self.SUMMARY, self.TEXT)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(rows["time-O0 seconds"][2], "1.000")
        self.assertEqual(rows["time-O3 seconds"][2], "1.000")
        self.assertEqual(rows["time-O0 (to host)"][:2], ("0.100000", "0.200000"))
        self.assertEqual(rows["rss-O3 KB"][2], "1.000")
        self.assertEqual(rows["O0 total instructions"][2], "1.100")
        self.assertEqual(rows["text bytes"][2], "1.100")

    def test_a_failed_or_timed_out_compile_is_reported_and_fails(self):
        result, rows = self.report(
            "time-O0\t1.000000\t10.000000\t0.100000\nrss-O0\t100\t200\t0.5\n"
            "time-O3\t2.000000\t20.000000\t0.100000\nrss-O3\t100\t400\t0.25\n",
            "time-O0\ttimeout\t5.000000\ttimeout\nrss-O0\ttimeout\t200\ttimeout\n"
            "time-O3\tfailed\t10.000000\tfailed\nrss-O3\tfailed\t400\tfailed\n",
            self.SUMMARY, self.TEXT)
        self.assertEqual(result.returncode, 1)
        self.assertIn("incomplete:", result.stdout)
        self.assertEqual(rows["time-O0 seconds"], ("1.000000", "timeout", "-"))
        self.assertEqual(rows["time-O3 seconds"], ("2.000000", "failed", "-"))
        self.assertIn("timeout", result.stdout)
