#!/usr/bin/env python3
"""Validate local N3485 clause references against doc/n3485.txt.

The validator intentionally checks references against the checked-in draft text,
not against memory or a newer standard. It understands the common local forms:

  // N3485 focus: 14.3.2 [temp.arg.nontype], 14.7.3 [temp.expl.spec]
  // N3485 focus: [temp.deduct], [temp.dep.expr], and alias-template SFINAE.
  // N3485 focus: 14.5.7 alias templates and 14.8.2 substitution.

For a number/name pair, both sides must resolve to the same N3485 heading. For
number-only or bracket-only references, the referenced heading must exist.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


DEFAULT_STANDARD = Path("doc/n3485.txt")
DEFAULT_EXCLUDE_DIRS = {
    ".git",
    "obj",
}
GENERATED_SUFFIX_MARKERS = (
    ".my",
    ".check",
    ".diff",
    ".testout",
)
PLACEHOLDER_LABELS = {
    "clause.name",
}


HEADING_WITH_LABEL_RE = re.compile(
    r"^\s*(?P<number>\d+(?:\.\d+)*)\s+"
    r"(?P<title>.*?)\s+\[(?P<label>[A-Za-z][A-Za-z0-9_.-]*)\]\s*$"
)
HEADING_WITHOUT_LABEL_RE = re.compile(
    r"^\s*(?P<number>\d+(?:\.\d+)*)\s+(?P<title>.*\S)\s*$"
)
LABEL_ONLY_RE = re.compile(r"^\s*\[(?P<label>[A-Za-z][A-Za-z0-9_.-]*)\]\s*$")
PAIR_RE = re.compile(
    r"(?P<number>\d+(?:\.\d+)*)\s*\[(?P<label>[A-Za-z][A-Za-z0-9_.-]*)\]"
)
LABEL_REF_RE = re.compile(r"\[(?P<label>[A-Za-z][A-Za-z0-9_.-]*)\]")
NUMBER_REF_RE = re.compile(r"(?<![A-Za-z0-9_.])(?P<number>\d+(?:\.\d+)+)(?![A-Za-z0-9_.])")


@dataclass(frozen=True)
class Heading:
    number: str
    label: str
    title: str
    line: int


@dataclass(frozen=True)
class ReferenceLine:
    path: Path
    line: int
    text: str


@dataclass(frozen=True)
class Finding:
    severity: str
    path: Path
    line: int
    message: str
    text: str


def normalize_title(title: str) -> str:
    return re.sub(r"\s+", " ", title).strip()


def parse_standard(path: Path) -> tuple[dict[str, Heading], dict[str, Heading]]:
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    by_number: dict[str, Heading] = {}
    by_label: dict[str, Heading] = {}
    pending: tuple[int, str, str] | None = None

    def add_heading(number: str, title: str, label: str, line: int) -> None:
        heading = Heading(number=number, label=label, title=normalize_title(title), line=line)
        by_number.setdefault(number, heading)
        by_label.setdefault(label, heading)

    for index, line in enumerate(lines, 1):
        match = HEADING_WITH_LABEL_RE.match(line)
        if match:
            add_heading(
                match.group("number"),
                match.group("title"),
                match.group("label"),
                index,
            )
            pending = None
            continue

        label_match = LABEL_ONLY_RE.match(line)
        if label_match and pending is not None:
            pending_line, number, title = pending
            add_heading(number, title, label_match.group("label"), pending_line)
            pending = None
            continue

        pending = None
        no_label = HEADING_WITHOUT_LABEL_RE.match(line)
        if not no_label:
            continue

        number = no_label.group("number")
        title = normalize_title(no_label.group("title"))
        if not title:
            continue
        # Top-level paragraphs frequently start with "1 A ..."; only keep a
        # pending split heading if this looks like a section heading.
        if "." in number or len(number) <= 2:
            pending = (index, number, title)

    return by_number, by_label


def git_tracked_files(root: Path) -> list[Path]:
    try:
        output = subprocess.check_output(
            ["git", "ls-files"],
            cwd=root,
            text=True,
            stderr=subprocess.DEVNULL,
        )
    except (OSError, subprocess.CalledProcessError):
        return []
    return [root / line for line in output.splitlines() if line]


def is_generated_path(path: Path) -> bool:
    name = path.name
    if any(marker in name for marker in GENERATED_SUFFIX_MARKERS):
        return True
    return path.suffix in {".o", ".a", ".so", ".dylib", ".exe"}


def iter_default_files(root: Path, standard: Path) -> Iterable[Path]:
    tracked = git_tracked_files(root)
    if tracked:
        candidates = tracked
    else:
        candidates = [path for path in root.rglob("*") if path.is_file()]

    standard_abs = standard.resolve()
    for path in sorted(candidates):
        try:
            rel_parts = path.relative_to(root).parts
        except ValueError:
            rel_parts = path.parts
        if any(part in DEFAULT_EXCLUDE_DIRS for part in rel_parts):
            continue
        if path.resolve() == standard_abs:
            continue
        if is_generated_path(path):
            continue
        yield path


def read_text(path: Path) -> str | None:
    try:
        data = path.read_bytes()
    except OSError:
        return None
    if b"\0" in data:
        return None
    try:
        return data.decode("utf-8")
    except UnicodeDecodeError:
        return data.decode("utf-8", errors="replace")


def iter_reference_lines(paths: Iterable[Path]) -> Iterable[ReferenceLine]:
    for path in paths:
        text = read_text(path)
        if text is None:
            continue
        lines = text.splitlines()
        for index, line in enumerate(lines, 1):
            if "N3485" not in line:
                continue
            block = [line.strip()]
            next_index = index
            while next_index < len(lines):
                raw_next = lines[next_index]
                stripped = raw_next.strip()
                if not stripped.startswith("//"):
                    break
                # Continuation comments are common for N3485 focus lines.
                block.append(stripped)
                next_index += 1
            yield ReferenceLine(path=path, line=index, text=" ".join(block))


def ranges_overlap(left: tuple[int, int], right: tuple[int, int]) -> bool:
    return left[0] < right[1] and right[0] < left[1]


def canonical(heading: Heading) -> str:
    return f"{heading.number} [{heading.label}] {heading.title}"


def validate_reference_line(
    ref: ReferenceLine,
    by_number: dict[str, Heading],
    by_label: dict[str, Heading],
) -> tuple[list[Finding], int]:
    findings: list[Finding] = []
    paired_ranges: list[tuple[int, int]] = []
    count = 0

    for match in PAIR_RE.finditer(ref.text):
        count += 1
        paired_ranges.append(match.span())
        number = match.group("number")
        label = match.group("label")
        num_heading = by_number.get(number)
        label_heading = by_label.get(label)
        if num_heading is None:
            findings.append(
                Finding("error", ref.path, ref.line, f"unknown N3485 clause {number}", ref.text)
            )
            continue
        if label_heading is None:
            findings.append(
                Finding("error", ref.path, ref.line, f"unknown N3485 stable label [{label}]", ref.text)
            )
            continue
        if num_heading.label != label:
            findings.append(
                Finding(
                    "error",
                    ref.path,
                    ref.line,
                    (
                        f"mismatched N3485 reference {number} [{label}]: "
                        f"{number} is {canonical(num_heading)}, but [{label}] is {canonical(label_heading)}"
                    ),
                    ref.text,
                )
            )

    for match in LABEL_REF_RE.finditer(ref.text):
        if any(ranges_overlap(match.span(), span) for span in paired_ranges):
            continue
        label = match.group("label")
        if label in PLACEHOLDER_LABELS:
            continue
        count += 1
        if label not in by_label:
            findings.append(
                Finding("error", ref.path, ref.line, f"unknown N3485 stable label [{label}]", ref.text)
            )

    for match in NUMBER_REF_RE.finditer(ref.text):
        if any(ranges_overlap(match.span(), span) for span in paired_ranges):
            continue
        count += 1
        number = match.group("number")
        if number not in by_number:
            findings.append(
                Finding("error", ref.path, ref.line, f"unknown N3485 clause {number}", ref.text)
            )

    return findings, count


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "paths",
        nargs="*",
        type=Path,
        help="files or directories to scan; defaults to tracked repository files",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path("."),
        help="repository root used for default tracked-file scanning",
    )
    parser.add_argument(
        "--standard",
        type=Path,
        default=DEFAULT_STANDARD,
        help=f"N3485 text file to validate against (default: {DEFAULT_STANDARD})",
    )
    parser.add_argument(
        "--show-ok",
        action="store_true",
        help="print resolved reference lines that have no findings",
    )
    return parser.parse_args()


def expand_paths(paths: list[Path]) -> list[Path]:
    expanded: list[Path] = []
    for path in paths:
        if path.is_dir():
            expanded.extend(sorted(p for p in path.rglob("*") if p.is_file()))
        elif path.is_file():
            expanded.append(path)
    return expanded


def main() -> int:
    args = parse_args()
    root = args.root.resolve()
    standard = args.standard if args.standard.is_absolute() else root / args.standard
    if not standard.exists():
        raise SystemExit(f"standard text not found: {standard}")

    by_number, by_label = parse_standard(standard)
    if not by_number or not by_label:
        raise SystemExit(f"no N3485 headings parsed from {standard}")

    if args.paths:
        scan_paths = expand_paths([p if p.is_absolute() else root / p for p in args.paths])
    else:
        scan_paths = list(iter_default_files(root, standard))

    findings: list[Finding] = []
    reference_lines = 0
    reference_count = 0
    for ref in iter_reference_lines(scan_paths):
        line_findings, line_count = validate_reference_line(ref, by_number, by_label)
        reference_lines += 1
        reference_count += line_count
        findings.extend(line_findings)
        if args.show_ok and not line_findings and line_count:
            print(f"ok: {ref.path.relative_to(root)}:{ref.line}: {ref.text}")

    for finding in findings:
        try:
            display_path = finding.path.relative_to(root)
        except ValueError:
            display_path = finding.path
        print(f"{finding.severity}: {display_path}:{finding.line}: {finding.message}")
        print(f"  {finding.text}")

    print(
        "N3485 reference validation: "
        f"headings={len(by_number)} labels={len(by_label)} "
        f"files={len(scan_paths)} reference_lines={reference_lines} "
        f"references={reference_count} findings={len(findings)}"
    )
    return 1 if findings else 0


if __name__ == "__main__":
    sys.exit(main())
