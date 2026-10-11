#!/usr/bin/env python3
"""Run tests-first audit controls without blessing the solution's known bugs.

These controls remain opt-in until the corresponding fixes are ready. Expected
outcomes live in the fixture source; no generated reference is needed.
"""

import argparse
import json
from pathlib import Path
import shlex
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parent.parent


def command_result(command, directory, timeout=60):
    try:
        result = subprocess.run(command, cwd=directory, capture_output=True,
                                text=True, timeout=timeout)
        return {"command": command, "status": result.returncode,
                "stdout": result.stdout, "stderr": result.stderr}
    except subprocess.TimeoutExpired as error:
        return {"command": command, "status": "timeout",
                "stdout": str(error.stdout or ""),
                "stderr": str(error.stderr or "")}
    except OSError as error:
        return {"command": command, "status": "launch failed",
                "stdout": "", "stderr": str(error)}


def display_path(source):
    try:
        return str(source.relative_to(ROOT))
    except ValueError:
        return str(source)


def print_failure(row):
    statuses = [observation["status"] for observation in row["observations"]]
    actual = f"; actual {row['actual']}" if "actual" in row else ""
    print(f"ERROR: {row['fixture']} {row['optimization']}: "
          f"expected {row['expectation']}; statuses {statuses}{actual}")
    for observation in reversed(row["observations"]):
        if observation["status"] != 0:
            detail = (observation["stderr"] or observation["stdout"]).strip()
            if detail:
                print("  " + detail.splitlines()[0])
            break


def run_case(source, compiler, host_cxx, host_oracle, optimization,
             expectation_override=None):
    text = source.read_text()
    expectation = expectation_override
    if expectation is None:
        header = next(line for line in text.splitlines()
                      if line.startswith("// AUDIT-EXPECT: "))
        expectation = header[len("// AUDIT-EXPECT: "):].strip()
    if expectation not in ("compile", "reject", "run", "symbols"):
        raise ValueError(f"{source}: invalid expectation {expectation!r}")
    extension = "// AUDIT-DIALECT: hosted-conditional-explicit" in text
    with tempfile.TemporaryDirectory(prefix="cppgm-audit-regression-") as temp:
        directory = Path(temp)
        output = directory / "fixture.o"
        flags = ["-std=c++11", optimization, "-c"]
        if host_oracle:
            flags += ["-x", "c++"]
        if host_oracle and not extension:
            flags += ["-pedantic-errors"]
        compile_result = command_result(
            compiler + flags + [str(source), "-o", str(output)], directory)
        observations = [compile_result]
        status = compile_result["status"]
        # A crash, timeout or unsupported-feature status is never a rejection.
        passed = status == 1 if expectation == "reject" else status == 0
        if passed and expectation == "symbols":
            symbols = command_result(
                ["perl", str(ROOT / "scripts/check_object_expectations.pl"),
                 str(source.with_suffix(".expect")), str(output)], directory)
            observations.append(symbols)
            passed = symbols["status"] == 0
        if passed and expectation == "run":
            program = directory / "fixture"
            link = command_result(host_cxx + [str(output), "-o", str(program)],
                                  directory)
            observations.append(link)
            passed = link["status"] == 0
            if passed:
                execution = command_result([str(program)], directory)
                observations.append(execution)
                passed = execution["status"] == 0
        return {"fixture": display_path(source),
                "optimization": optimization, "expectation": expectation,
                "passed": passed, "observations": observations}


def run_lowir_case(source, native, optimization):
    with tempfile.TemporaryDirectory(prefix="cppgm-audit-frame-") as temp:
        directory = Path(temp)
        program = directory / "fixture"
        mir = directory / "fixture.mir"
        compile_result = command_result(
            native + [optimization, "--dump-machine-ir", str(mir),
                      "-o", str(program), str(source)], directory)
        observations = [compile_result]
        passed = compile_result["status"] == 0
        if passed:
            execution = command_result([str(program)], directory)
            observations.append(execution)
            passed = execution["status"] == 0
            quality = command_result(
                ["perl", str(ROOT / "scripts/expect_ir.pl"), str(mir),
                 str(source.with_suffix(".expect"))], directory)
            observations.append(quality)
            passed = passed and quality["status"] == 0
        return {"fixture": display_path(source),
                "optimization": optimization, "expectation": "run and frame bound",
                "passed": passed, "observations": observations}


def run_abi_case(source, abimangle):
    fields = dict(line.split(" ", 1) for line in source.read_text().splitlines()
                  if line and not line.startswith("#"))
    with tempfile.TemporaryDirectory(prefix="cppgm-audit-abi-") as temp:
        directory = Path(temp)
        output = directory / "name.txt"
        observation = command_result(
            abimangle + ["-o", str(output), str(ROOT / fields["input"])], directory)
        actual = output.read_text().strip() if output.exists() else None
        return {"fixture": display_path(source), "optimization": "none",
                "expectation": fields["expected"], "actual": actual,
                "passed": observation["status"] == 0 and actual == fields["expected"],
                "observations": [observation]}


def run_standalone_case(source, compiler, native, optimization):
    with tempfile.TemporaryDirectory(prefix="cppgm-audit-standalone-") as temp:
        directory = Path(temp)
        lowir = directory / "fixture.lowir"
        program = directory / "fixture"
        frontend = command_result(
            compiler + ["--emit-lowir", optimization, "-o", str(lowir),
                        str(source)], directory)
        observations = [frontend]
        passed = frontend["status"] == 0
        if passed:
            backend = command_result(native + [optimization, "-o", str(program),
                                               str(lowir)], directory)
            observations.append(backend)
            passed = backend["status"] == 0
            if passed:
                execution = command_result([str(program)], directory)
                observations.append(execution)
                passed = execution["status"] == 0
        return {"fixture": display_path(source), "optimization": optimization,
                "expectation": "standalone run", "passed": passed,
                "observations": observations}


def run_loop_cases(control, fields, lowiropt, native):
    rows = []
    original = (ROOT / fields["input"]).read_text()
    for optimization in fields["optimization"].split():
        for offset, expected in ((16, "finite"), (1, "nonterminating")):
            with tempfile.TemporaryDirectory(prefix="cppgm-audit-loop-") as temp:
                directory = Path(temp)
                source = directory / "input.lowir"
                source.write_text(original + f'''
function @main() -> i32 [role=entry] {{
  slot $storage : obj<32x8>
  block ^entry:
    %begin = addr $storage
    %end = index i8 %begin, {offset}
    %result = call ptr @{fields["function"]}(%begin, %end)
    %bad = cmp ne ptr %result, %begin
    %exit = convert trunc i32 i64 %bad
    return i32 %exit
}}
''')
                optimized = directory / "optimized.lowir"
                program = directory / "program"
                observations = [command_result(
                    lowiropt + ["-" + optimization, "-o", str(optimized),
                                str(source)], directory)]
                passed = observations[-1]["status"] == 0
                if passed:
                    observations.append(command_result(
                        native + ["-O0", "-o", str(program), str(optimized)], directory))
                    passed = observations[-1]["status"] == 0
                    if passed:
                        observations.append(command_result([str(program)], directory, timeout=2))
                        required = 0 if expected == "finite" else "timeout"
                        passed = observations[-1]["status"] == required
                rows.append({"fixture": display_path(control),
                             "optimization": optimization, "expectation": expected,
                             "passed": passed, "observations": observations})
    return rows


def run_reader_case(control, fields, lowiropt):
    with tempfile.TemporaryDirectory(prefix="cppgm-audit-reader-") as temp:
        directory = Path(temp)
        observation = command_result(
            lowiropt + ["-" + fields["optimization"], "-o",
                        str(directory / "output.lowir"),
                        str(ROOT / fields["input"])], directory)
        return {"fixture": display_path(control),
                "optimization": fields["optimization"], "expectation": "reject",
                "passed": observation["status"] == 1,
                "observations": [observation]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--compiler", default=str(ROOT / "dev/cppgm++"))
    parser.add_argument("--host-cxx", default="g++")
    parser.add_argument("--native", default=str(ROOT / "dev/lowir2native"))
    parser.add_argument("--abimangle", default=str(ROOT / "dev/abimangle"))
    parser.add_argument("--lowiropt", default=str(ROOT / "dev/lowiropt"))
    parser.add_argument("--host-oracle", action="store_true")
    parser.add_argument("--optimization", choices=("O0", "O1", "O2", "O3"),
                        action="append")
    parser.add_argument("--report", type=Path)
    parser.add_argument("fixtures", nargs="*", type=Path)
    args = parser.parse_args()
    sources = args.fixtures or sorted([
        *ROOT.glob("pa*/tests/controls/*-audit-*.cpp"),
        *ROOT.glob("pa*/tests/controls/*-audit-*.lowir"),
        *ROOT.glob("pa*/tests/controls/*-audit-*.audit")])
    if not sources:
        parser.error("no audit controls selected")
    compiler = shlex.split(args.compiler)
    host_cxx = shlex.split(args.host_cxx)
    native = shlex.split(args.native)
    abimangle = shlex.split(args.abimangle)
    lowiropt = shlex.split(args.lowiropt)
    for command in (compiler, host_cxx, native, abimangle, lowiropt):
        if not command:
            parser.error("compiler commands must not be empty")
        if Path(command[0]).is_file():
            command[0] = str(Path(command[0]).resolve())
    rows = []
    for source in sources:
        source = source.resolve()
        if source.suffix == ".audit":
            fields = dict(line.split(" ", 1) for line in source.read_text().splitlines()
                          if line and not line.startswith("#"))
            kind = fields.get("kind", "abi")
            if kind == "abi":
                if args.host_oracle:
                    continue
                case_rows = [run_abi_case(source, abimangle)]
            elif kind == "loop":
                if args.host_oracle:
                    continue
                if args.optimization:
                    fields["optimization"] = " ".join(
                        opt for opt in fields["optimization"].split()
                        if opt in args.optimization)
                case_rows = run_loop_cases(source, fields, lowiropt, native)
            elif kind == "reader":
                if args.host_oracle:
                    continue
                case_rows = [run_reader_case(source, fields, lowiropt)]
            elif kind in ("source", "standalone"):
                input_source = ROOT / fields["input"]
                case_rows = []
                for optimization in fields["optimization"].split():
                    if args.optimization and optimization not in args.optimization:
                        continue
                    if kind == "standalone" and not args.host_oracle:
                        row = run_standalone_case(input_source, compiler, native,
                                                  "-" + optimization)
                    else:
                        row = run_case(input_source, compiler, host_cxx,
                                       args.host_oracle, "-" + optimization,
                                       fields["expected"])
                    row["input"] = row["fixture"]
                    row["fixture"] = display_path(source)
                    case_rows.append(row)
            else:
                parser.error(f"{source}: unknown control kind {kind!r}")
            rows.extend(case_rows)
            for row in case_rows:
                if not row["passed"]:
                    print_failure(row)
            continue
        for optimization in args.optimization or ["O0", "O2"]:
            if source.suffix == ".lowir":
                if args.host_oracle or f"# AUDIT-OPT: {optimization}" not in source.read_text():
                    continue
                row = run_lowir_case(source, native, "-" + optimization)
            else:
                required_opts = [line.split(":", 1)[1].strip()
                                 for line in source.read_text().splitlines()
                                 if line.startswith("// AUDIT-OPT:")]
                if required_opts and optimization not in required_opts:
                    continue
                row = run_case(source, compiler, host_cxx, args.host_oracle,
                               "-" + optimization)
            rows.append(row)
            if not row["passed"]:
                print_failure(row)
    if not rows:
        parser.error("no controls apply to the selected compiler/optimization")
    if args.report:
        args.report.write_text(json.dumps(rows, indent=2) + "\n")
    passed = sum(row["passed"] for row in rows)
    print(f"Audit regression controls: {passed}/{len(rows)} passed")
    return 0 if passed == len(rows) else 1


if __name__ == "__main__":
    raise SystemExit(main())
