#!/usr/bin/env python3
"""Record and check compiler performance baselines.

The check gates on instruction count and memory. Wall time is recorded for
context, but it is not used to fail a candidate.
"""

import argparse
import datetime as _datetime
import hashlib
import json
from pathlib import Path
import re
import statistics
import subprocess
import sys


DEFAULT_COMMAND = [
    "./dev/cppgm++",
    "-I",
    "benchmarks/self_compile/stable/include",
    "-c",
    "-o",
    "/tmp/cppgm-perf-check.o",
    "benchmarks/self_compile/stable/semantic_overload.cpp",
]

FROZEN_WORKLOAD_MANIFEST = Path(
    "benchmarks/self_compile/stable/PERF_EPOCH.json"
)
FROZEN_WORKLOAD_SOURCE = "benchmarks/self_compile/stable/semantic_overload.cpp"
FROZEN_WORKLOAD_INCLUDE_ROOT = "benchmarks/self_compile/stable/include"

TIME_VALUE_LABEL_RE = re.compile(r"^\s*([0-9]+(?:\.[0-9]+)?)\s+(.+?)\s*$")
TIME_LABEL_VALUE_RE = re.compile(
    r"^\s*([A-Za-z][A-Za-z0-9 _-]*?)\s+([0-9]+(?:\.[0-9]+)?)\s*$"
)

INTEGER_METRICS = {
    "maximum_resident_set_size",
    "average_shared_memory_size",
    "average_unshared_data_size",
    "average_unshared_stack_size",
    "page_reclaims",
    "page_faults",
    "swaps",
    "block_input_operations",
    "block_output_operations",
    "messages_sent",
    "messages_received",
    "signals_received",
    "voluntary_context_switches",
    "involuntary_context_switches",
    "instructions_retired",
    "cycles_elapsed",
    "peak_memory_footprint",
}

MEMORY_METRICS = {
    "maximum_resident_set_size",
    "peak_memory_footprint",
}

FLOAT_METRICS = {
    "real",
    "user",
    "sys",
}

KNOWN_TIME_METRICS = INTEGER_METRICS | FLOAT_METRICS

CHECKS = [
    ("instructions_retired", "instructions", "instruction_tolerance"),
    ("maximum_resident_set_size", "max rss", "rss_tolerance"),
    ("peak_memory_footprint", "peak footprint", "footprint_tolerance"),
]


class FrozenWorkloadError(RuntimeError):
    pass


def normalize_metric_name(label):
    return label.strip().lower().replace(" ", "_").replace("-", "_")


def parse_time_metrics(stderr):
    metrics = {}
    non_time_lines = []
    for line in stderr.splitlines():
        match = TIME_VALUE_LABEL_RE.match(line)
        if match:
            raw_value, label = match.groups()
        else:
            match = TIME_LABEL_VALUE_RE.match(line)
            if not match:
                non_time_lines.append(line)
                continue
            label, raw_value = match.groups()
            key = normalize_metric_name(label)
            if key not in KNOWN_TIME_METRICS:
                non_time_lines.append(line)
                continue

        key = normalize_metric_name(label)
        if key not in KNOWN_TIME_METRICS:
            non_time_lines.append(line)
            continue

        if key in INTEGER_METRICS:
            value = int(float(raw_value))
        else:
            value = float(raw_value)
        metrics[key] = value

    return metrics, "\n".join(non_time_lines).strip()


def repo_head(repo_root):
    try:
        proc = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=str(repo_root),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return proc.stdout.strip()


def clean_command(command):
    cleaned = list(command)
    if cleaned and cleaned[0] == "--":
        cleaned = cleaned[1:]
    return cleaned or list(DEFAULT_COMMAND)


def normalized_workload_command(command):
    normalized = []
    skip_output = False
    for arg in command:
        if skip_output:
            normalized.append("<output>")
            skip_output = False
            continue
        if arg == "-o":
            normalized.append(arg)
            skip_output = True
            continue
        if arg.startswith("-o") and len(arg) > 2:
            normalized.append("-o<output>")
            continue
        normalized.append(arg)
    return normalized


def sha256_file(path):
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def header_closure_digest(file_hashes):
    digest = hashlib.sha256()
    for relative_path in sorted(file_hashes):
        digest.update(relative_path.encode("utf-8"))
        digest.update(b"\0")
        digest.update(file_hashes[relative_path].encode("ascii"))
        digest.update(b"\n")
    return digest.hexdigest()


def validate_frozen_workload(repo_root, command):
    canonical_command = normalized_workload_command(DEFAULT_COMMAND)
    actual_command = normalized_workload_command(command)
    if actual_command != canonical_command:
        raise FrozenWorkloadError(
            "command is not the frozen semantic-overload workload: expected=%s actual=%s"
            % (" ".join(canonical_command), " ".join(actual_command))
        )

    repo_root = Path(repo_root).resolve()
    manifest_path = repo_root / FROZEN_WORKLOAD_MANIFEST
    try:
        manifest = load_json(manifest_path)
    except (OSError, ValueError) as exc:
        raise FrozenWorkloadError(
            "cannot read frozen workload manifest %s: %s" % (manifest_path, exc)
        )

    required_fields = {
        "schema_version",
        "epoch_commit",
        "source_snapshot_commit",
        "source",
        "source_sha256",
        "include_root",
        "header_closure_sha256",
        "headers",
    }
    missing_fields = sorted(required_fields - set(manifest))
    if missing_fields:
        raise FrozenWorkloadError(
            "manifest is missing fields: %s" % ", ".join(missing_fields)
        )
    if manifest["schema_version"] != 1:
        raise FrozenWorkloadError(
            "unsupported frozen workload manifest schema %r"
            % manifest["schema_version"]
        )
    if manifest["source"] != FROZEN_WORKLOAD_SOURCE:
        raise FrozenWorkloadError(
            "manifest source changed: %s" % manifest["source"]
        )
    if manifest["include_root"] != FROZEN_WORKLOAD_INCLUDE_ROOT:
        raise FrozenWorkloadError(
            "manifest include root changed: %s" % manifest["include_root"]
        )
    if not isinstance(manifest["headers"], dict) or not manifest["headers"]:
        raise FrozenWorkloadError("manifest header closure is empty or invalid")

    source_path = repo_root / manifest["source"]
    if not source_path.is_file() or source_path.is_symlink():
        raise FrozenWorkloadError(
            "frozen workload source is missing or is a symlink: %s" % source_path
        )
    actual_source_hash = sha256_file(source_path)
    if actual_source_hash != manifest["source_sha256"]:
        raise FrozenWorkloadError(
            "frozen workload source digest differs: expected=%s actual=%s"
            % (manifest["source_sha256"], actual_source_hash)
        )

    include_root = repo_root / manifest["include_root"]
    if not include_root.is_dir() or include_root.is_symlink():
        raise FrozenWorkloadError(
            "frozen include root is missing or is a symlink: %s" % include_root
        )

    expected_headers = manifest["headers"]
    invalid_header_names = sorted(
        name
        for name in expected_headers
        if not isinstance(name, str)
        or not name
        or Path(name).is_absolute()
        or ".." in Path(name).parts
    )
    if invalid_header_names:
        raise FrozenWorkloadError(
            "manifest has invalid header paths: %s" % ", ".join(invalid_header_names)
        )

    actual_header_paths = {}
    symlinks = []
    for path in sorted(include_root.rglob("*")):
        relative_path = path.relative_to(include_root).as_posix()
        if path.is_symlink():
            symlinks.append(relative_path)
        elif path.is_file():
            actual_header_paths[relative_path] = path
    if symlinks:
        raise FrozenWorkloadError(
            "frozen header closure contains symlinks: %s" % ", ".join(symlinks)
        )

    expected_names = set(expected_headers)
    actual_names = set(actual_header_paths)
    missing_headers = sorted(expected_names - actual_names)
    extra_headers = sorted(actual_names - expected_names)
    if missing_headers or extra_headers:
        details = []
        if missing_headers:
            details.append("missing=%s" % ",".join(missing_headers))
        if extra_headers:
            details.append("extra=%s" % ",".join(extra_headers))
        raise FrozenWorkloadError(
            "frozen header closure membership differs: %s" % " ".join(details)
        )

    actual_header_hashes = {
        name: sha256_file(actual_header_paths[name]) for name in sorted(actual_names)
    }
    changed_headers = sorted(
        name
        for name in expected_names
        if actual_header_hashes[name] != expected_headers[name]
    )
    if changed_headers:
        raise FrozenWorkloadError(
            "frozen header digests differ: %s" % ", ".join(changed_headers)
        )

    actual_closure_digest = header_closure_digest(actual_header_hashes)
    if actual_closure_digest != manifest["header_closure_sha256"]:
        raise FrozenWorkloadError(
            "frozen header closure digest differs: expected=%s actual=%s"
            % (manifest["header_closure_sha256"], actual_closure_digest)
        )

    return {
        "schema_version": manifest["schema_version"],
        "epoch_commit": manifest["epoch_commit"],
        "source_snapshot_commit": manifest["source_snapshot_commit"],
        "source": manifest["source"],
        "source_sha256": actual_source_hash,
        "include_root": manifest["include_root"],
        "header_count": len(actual_header_hashes),
        "header_closure_sha256": actual_closure_digest,
    }


def run_once(time_binary, repo_root, command, timeout_sec):
    timed_command = [time_binary, "-lp"] + command
    try:
        proc = subprocess.run(
            timed_command,
            cwd=str(repo_root),
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            timeout=timeout_sec if timeout_sec > 0 else None,
        )
    except subprocess.TimeoutExpired as exc:
        print("Command timed out after %s seconds:" % timeout_sec, file=sys.stderr)
        print(" ".join(command), file=sys.stderr)
        if exc.stdout:
            print(exc.stdout, file=sys.stderr)
        if exc.stderr:
            print(exc.stderr, file=sys.stderr)
        raise SystemExit(2)

    metrics, command_stderr = parse_time_metrics(proc.stderr)
    if proc.returncode != 0:
        print("Command failed with exit code %d:" % proc.returncode, file=sys.stderr)
        print(" ".join(command), file=sys.stderr)
        if proc.stdout:
            print(proc.stdout, file=sys.stderr)
        if proc.stderr:
            print(proc.stderr, file=sys.stderr)
        raise SystemExit(proc.returncode)

    return {
        "metrics": metrics,
        "stdout": proc.stdout.strip(),
        "stderr": command_stderr,
    }


def collect_runs(args, command):
    runs = []
    for index in range(args.runs):
        print("run %d/%d..." % (index + 1, args.runs), flush=True)
        runs.append(
            run_once(
                args.time_binary,
                Path(args.repo_root).resolve(),
                command,
                args.timeout_sec,
            )
        )
    return runs


def summarize_runs(runs):
    keys = sorted({key for run in runs for key in run["metrics"].keys()})
    summary = {}
    for key in keys:
        values = [run["metrics"][key] for run in runs if key in run["metrics"]]
        if not values:
            continue
        summary[key] = {
            "median": statistics.median(values),
            "min": min(values),
            "max": max(values),
        }
    return summary


def make_report(args, command, runs, workload):
    repo_root = Path(args.repo_root).resolve()
    return {
        "created_at": _datetime.datetime.now(
            _datetime.timezone.utc
        ).isoformat(),
        "repo_root": str(repo_root),
        "head": repo_head(repo_root),
        "command": command,
        "workload": workload,
        "runs": runs,
        "summary": summarize_runs(runs),
    }


def write_json(path, data):
    output_path = Path(path)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(json.dumps(data, indent=2, sort_keys=True) + "\n")


def load_json(path):
    with Path(path).open() as handle:
        return json.load(handle)


def format_bytes(value):
    value = float(value)
    units = ["B", "KiB", "MiB", "GiB"]
    unit = units[0]
    for unit in units:
        if abs(value) < 1024.0 or unit == units[-1]:
            break
        value /= 1024.0
    return "%.2f %s" % (value, unit)


def format_metric(key, value):
    if value is None:
        return "missing"
    if key in MEMORY_METRICS:
        return "%s (%s)" % (format_bytes(value), f"{int(value):,}")
    if key in INTEGER_METRICS:
        return f"{int(value):,}"
    return "%.3f" % float(value)


def print_record_summary(report):
    print("")
    print("Recorded baseline")
    print("  head: %s" % (report.get("head") or "unknown"))
    print("  command: %s" % " ".join(report["command"]))
    if report.get("workload"):
        print("  workload epoch: %s" % report["workload"]["epoch_commit"])
        print(
            "  frozen headers: %d (%s)"
            % (
                report["workload"]["header_count"],
                report["workload"]["header_closure_sha256"],
            )
        )
    for key in [
        "instructions_retired",
        "maximum_resident_set_size",
        "peak_memory_footprint",
        "real",
        "user",
        "sys",
    ]:
        if key in report["summary"]:
            value = report["summary"][key]["median"]
            print("  %-28s %s" % (key + ":", format_metric(key, value)))


def compare_reports(
    baseline,
    candidate,
    args,
    rss_exceedance="fail",
    warnings=None,
    heading="Performance check",
):
    if rss_exceedance not in {"fail", "warn"}:
        raise ValueError("rss_exceedance must be 'fail' or 'warn'")
    if warnings is None:
        warnings = []
    baseline_summary = baseline.get("summary", {})
    candidate_summary = candidate.get("summary", {})
    failures = []
    baseline_command = normalized_workload_command(baseline.get("command", []))
    candidate_command = normalized_workload_command(candidate.get("command", []))
    if baseline_command != candidate_command:
        failures.append(
            "benchmark workload command differs: baseline=%s candidate=%s"
            % (" ".join(baseline_command), " ".join(candidate_command))
        )
    baseline_workload = baseline.get("workload")
    candidate_workload = candidate.get("workload")
    if baseline_workload and candidate_workload:
        if baseline_workload != candidate_workload:
            failures.append(
                "frozen benchmark workload identity differs: baseline=%s candidate=%s"
                % (
                    json.dumps(baseline_workload, sort_keys=True),
                    json.dumps(candidate_workload, sort_keys=True),
                )
            )
    elif candidate_workload and not baseline_workload:
        legacy_epoch = candidate_workload.get("epoch_commit")
        if baseline.get("head") != legacy_epoch:
            failures.append(
                "baseline lacks frozen workload identity and was not recorded at epoch %s"
                % legacy_epoch
            )
    elif baseline_workload and not candidate_workload:
        failures.append("candidate lacks frozen benchmark workload identity")
    rows = []

    for key, label, tolerance_attr in CHECKS:
        tolerance = getattr(args, tolerance_attr)
        baseline_value = baseline_summary.get(key, {}).get("median")
        candidate_value = candidate_summary.get(key, {}).get("median")
        if baseline_value is None or candidate_value is None:
            failures.append("%s is missing from baseline or candidate" % key)
            rows.append(
                (label, key, baseline_value, candidate_value, tolerance, None, "FAIL")
            )
            continue

        delta = (float(candidate_value) / float(baseline_value)) - 1.0
        is_rss = key == "maximum_resident_set_size"
        exceeded = (
            delta >= tolerance
            if is_rss
            else float(candidate_value) > float(baseline_value) * (1.0 + tolerance)
        )
        status = "PASS"
        if exceeded:
            message = (
                "%s increased by %.2f%%; %s is %.2f%%"
                % (
                    key,
                    delta * 100.0,
                    "warning threshold" if is_rss else "tolerance",
                    tolerance * 100.0,
                )
            )
            if is_rss and rss_exceedance == "warn":
                warnings.append(message)
                status = "WARN"
            else:
                failures.append(message)
                status = "FAIL"
        rows.append(
            (label, key, baseline_value, candidate_value, tolerance, delta, status)
        )

    print("")
    print(heading)
    print("  baseline head: %s" % (baseline.get("head") or "unknown"))
    print("  candidate head: %s" % (candidate.get("head") or "unknown"))
    if candidate_workload:
        print("  workload epoch: %s" % candidate_workload.get("epoch_commit", "unknown"))
        print(
            "  frozen headers: %s (%s)"
            % (
                candidate_workload.get("header_count", "unknown"),
                candidate_workload.get("header_closure_sha256", "unknown"),
            )
        )
    print("")
    print("%-16s %-28s %-28s %-10s %-10s" % ("metric", "baseline", "candidate", "delta", "status"))
    for label, key, baseline_value, candidate_value, tolerance, delta, status in rows:
        delta_text = "missing" if delta is None else "%+.2f%%" % (delta * 100.0)
        threshold_label = "warn" if key == "maximum_resident_set_size" else "tol"
        status += " (%s %.2f%%)" % (threshold_label, tolerance * 100.0)
        print(
            "%-16s %-28s %-28s %-10s %-10s"
            % (
                label,
                format_metric(key, baseline_value),
                format_metric(key, candidate_value),
                delta_text,
                status,
            )
        )

    for key in ["real", "user", "sys", "cycles_elapsed"]:
        baseline_value = baseline_summary.get(key, {}).get("median")
        candidate_value = candidate_summary.get(key, {}).get("median")
        if baseline_value is None or candidate_value is None:
            continue
        delta = (float(candidate_value) / float(baseline_value)) - 1.0
        print(
            "  info %-20s %s -> %s (%+.2f%%)"
            % (
                key + ":",
                format_metric(key, baseline_value),
                format_metric(key, candidate_value),
                delta * 100.0,
            )
        )

    return failures


def command_record(args):
    if args.runs < 1:
        raise SystemExit("--runs must be at least 1")
    command = clean_command(args.command)
    try:
        workload = validate_frozen_workload(args.repo_root, command)
    except FrozenWorkloadError as exc:
        print("FAIL: %s" % exc, file=sys.stderr)
        return 2
    runs = collect_runs(args, command)
    report = make_report(args, command, runs, workload)
    write_json(args.baseline, report)
    print_record_summary(report)
    print("  wrote: %s" % args.baseline)
    return 0


def command_check(args):
    if args.runs < 1:
        raise SystemExit("--runs must be at least 1")
    command = clean_command(args.command)
    try:
        workload = validate_frozen_workload(args.repo_root, command)
    except FrozenWorkloadError as exc:
        print("FAIL: %s" % exc, file=sys.stderr)
        return 2
    baseline = load_json(args.baseline)
    runs = collect_runs(args, command)
    candidate = make_report(args, command, runs, workload)
    warnings = []
    failures = compare_reports(
        baseline,
        candidate,
        args,
        rss_exceedance="warn",
        warnings=warnings,
    )
    confirmation_candidate = None
    confirmation_failures = []
    if warnings and not failures:
        print("")
        for warning in warnings:
            print("WARN: %s" % warning)
        print(
            "WARN: maximum RSS reached the warning threshold; "
            "running one confirmation batch"
        )
        confirmation_runs = collect_runs(args, command)
        confirmation_candidate = make_report(
            args, command, confirmation_runs, workload
        )
        confirmation_failures = compare_reports(
            baseline,
            confirmation_candidate,
            args,
            rss_exceedance="fail",
            heading="Performance confirmation check",
        )
        failures.extend(confirmation_failures)
    if args.report:
        payload = {
            "baseline": baseline,
            "candidate": candidate,
            "failures": failures,
            "warnings": warnings,
        }
        if confirmation_candidate is not None:
            payload["confirmation_candidate"] = confirmation_candidate
            payload["confirmation_failures"] = confirmation_failures
        write_json(args.report, payload)
        print("  wrote report: %s" % args.report)

    if failures:
        print("")
        for failure in failures:
            print("FAIL: %s" % failure)
        return 1

    print("")
    if confirmation_candidate is not None:
        print("PASS: candidate cleared the maximum RSS confirmation check")
    else:
        print("PASS: candidate is within instruction and memory tolerances")
    return 0


def threshold_failure(message):
    return any(message.startswith(key + " increased by ") for key, _, _ in CHECKS)


def command_compare(args):
    baseline = load_json(args.baseline)
    candidate = load_json(args.candidate)
    warnings = []
    failures = compare_reports(
        baseline,
        candidate,
        args,
        rss_exceedance="warn",
        warnings=warnings,
        heading="Recorded performance comparison",
    )
    identity_failures = [failure for failure in failures if not threshold_failure(failure)]
    threshold_failures = [failure for failure in failures if threshold_failure(failure)]

    if args.report:
        write_json(
            args.report,
            {
                "advisory": args.advisory,
                "baseline": baseline,
                "candidate": candidate,
                "failures": failures,
                "warnings": warnings,
            },
        )
        print("  wrote report: %s" % args.report)

    if identity_failures:
        print("")
        for failure in identity_failures:
            print("FAIL: %s" % failure)
        return 1

    if args.advisory:
        print("")
        for failure in threshold_failures:
            print("WARN: %s" % failure)
        for warning in warnings:
            print("WARN: %s" % warning)
        if threshold_failures or warnings:
            print("PASS: recorded candidate has advisory performance deviations")
        else:
            print("PASS: recorded candidate is within advisory tolerances")
        return 0

    if threshold_failures or warnings:
        print("")
        for failure in threshold_failures:
            print("FAIL: %s" % failure)
        for warning in warnings:
            print("FAIL: %s; a recorded confirmation batch is required" % warning)
        return 1

    print("")
    print("PASS: recorded candidate is within instruction and memory tolerances")
    return 0


def add_common_args(parser, runs_default):
    parser.add_argument("--repo-root", default=Path(__file__).resolve().parents[1])
    parser.add_argument("--runs", type=int, default=runs_default)
    parser.add_argument("--timeout-sec", type=int, default=0)
    parser.add_argument("--time-binary", default="/usr/bin/time")
    parser.add_argument("command", nargs=argparse.REMAINDER)


def build_parser():
    parser = argparse.ArgumentParser(
        description="Record, check, or compare cppgm performance baselines."
    )
    subparsers = parser.add_subparsers(dest="mode", required=True)

    record = subparsers.add_parser("record", help="record a baseline JSON file")
    record.add_argument("--baseline", required=True)
    add_common_args(record, runs_default=3)
    record.set_defaults(func=command_record)

    check = subparsers.add_parser("check", help="check against a baseline JSON file")
    check.add_argument("--baseline", required=True)
    check.add_argument("--report")
    check.add_argument("--instruction-tolerance", type=float, default=0.005)
    check.add_argument(
        "--rss-warning-tolerance",
        "--rss-tolerance",
        dest="rss_tolerance",
        type=float,
        default=0.03,
        help=(
            "maximum RSS warning threshold; one confirmation batch runs when "
            "the threshold is reached, and a second exceedance fails"
        ),
    )
    check.add_argument("--footprint-tolerance", type=float, default=0.01)
    add_common_args(check, runs_default=3)
    check.set_defaults(func=command_check)

    compare = subparsers.add_parser(
        "compare", help="compare two recorded performance JSON files"
    )
    compare.add_argument("--baseline", required=True)
    compare.add_argument("--candidate", required=True)
    compare.add_argument("--report")
    compare.add_argument("--advisory", action="store_true")
    compare.add_argument("--instruction-tolerance", type=float, default=0.005)
    compare.add_argument(
        "--rss-warning-tolerance",
        "--rss-tolerance",
        dest="rss_tolerance",
        type=float,
        default=0.03,
    )
    compare.add_argument("--footprint-tolerance", type=float, default=0.01)
    compare.set_defaults(func=command_compare)

    return parser


def main(argv):
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
