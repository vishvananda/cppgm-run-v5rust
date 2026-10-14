#!/usr/bin/env python3
"""Fail when a tracked fixture sits outside every lane its assignment runs.

A fixture that no lane runs still ships to students, but nothing grades it.
For each paN/tests this check reports

  - an input (`x.t`, a `.cpp` control or program, a `.lowir` input, an
    `.audit` record) that no `test*` lane of paN's Makefile selects, saying
    so when a `ref-test*` lane regenerates its references all the same;
  - a file that is neither an input, nor a companion of one (a file or
    directory named `x.<anything>` beside input `x.t` or `x.cpp`:
    references, `x.t.N` and `x.t2` sources, `x.lib.*`, `x.expect`, `x.inc/`),
    nor a helper (HELPERS) that a fixture names.

The lanes come from the Makefile.  `make -n` prints the recipes of every
`test*` and `ref-test*` target with their variables expanded; each command
there that runs a script on paths under tests/ is a lane, and the script
decides what it selects: the `collect_tests(..., qr/.../)` pattern in its
source, or SELECTIONS below.  The shared collectors skip `controls/` and
`regression/` directories below a root, which are lanes of their own, and
so does this check unless a script walks its root itself.  What a Makefile
cannot show is listed in EXTERNAL_LANES with the reason.

Usage: check_fixture_lanes.py [--root DIR] [--verbose] [paN ...]
"""

import argparse
from concurrent.futures import ThreadPoolExecutor
import fnmatch
import glob
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import subprocess
import sys
import tempfile


REPO_ROOT = Path(__file__).resolve().parent.parent

GRADING_TARGET = re.compile(r"^test(?:-[a-z0-9-]+)?$")
REGENERATING_TARGET = re.compile(r"^ref-test(?:-[a-z0-9-]+)?$")
INPUT_SUFFIXES = (".t", ".cpp", ".lowir", ".audit")

# Scripts a lane recipe runs on fixture paths without running the fixtures.
NON_RUNNERS = {
    "compare_results.pl": "compares the outputs a runner wrote",
    "compare_results_common.pl": "compares the outputs a runner wrote",
    "compare_results_link.pl": "compares the outputs a runner wrote",
    "canonicalize_lowir_native_refs.pl": "rewrites the references a runner wrote",
    "check_preproc_refs_portable.sh": "checks the references for host paths",
    "check_ir_envelope.pl": "measures the programs the suite lanes build",
    "check_lowir_seams.py": "rewrites the outputs the suite lanes wrote",
}

# Runners whose selection is not a collect_tests(..., qr/.../) call in their
# source: (pattern on the path, whether controls/ and regression/ below a
# root are skipped).
SELECTIONS = {
    # Every paN/scripts/run_all_tests.pl runs run_all_tests_common.pl in
    # text_t mode or execs a worker that collects `.t` the same way.
    "run_all_tests.pl": (r"\.t$", True),
    "run_all_tests_common.pl": (r"\.t$", True),
    # PA32's object round trip walks its roots itself and does not prune.
    "run_object_lowir_roundtrip_tests.pl": (r"\.(?:cpp|t)$", False),
}

# Runners that select by content as well as by name and print what they
# select: the arguments that make them list it.
LISTING_RUNNERS = {
    # Only the controls that state an outcome; focused checkers own the rest.
    "check_controls.pl": ["--list"],
}

# Lanes a Makefile does not show: (assignment or "*", pattern under the
# assignment directory, the script that runs them from the repository root,
# how that script names them: "pattern" when it contains the pattern,
# "name" when it names each file).  Matching files are inputs.
EXTERNAL_LANES = (
    # PA35's targets run scripts/check.pl, whose suites name their inputs.
    ("pa35", "tests/behavior/*.cpp", "pa35/scripts/behavior.pl", "pattern"),
    ("pa35", "tests/byte_values/*", "pa35/scripts/generated.pl", "name"),
    ("pa35", "tests/generators/*", "pa35/scripts/generated.pl", "name"),
    # Tests-first records of known defects, kept out of the course lanes and
    # the student export until their fixes land; see
    # docs/student-reference-audit-review.md.
    ("*", "tests/controls/*-audit-*.audit",
     "scripts/check_pending_audit_regressions.py", "pattern"),
)

# Files that support fixtures without being one: (pattern on the file name,
# whether a fixture, or a helper a fixture names, must name it).
HELPERS = (
    ("*.h", True),         # headers fixtures include
    ("*.inc", True),       # sources fixtures include
    ("README.md", False),  # notes on a lane's directory
    (".gitkeep", False),   # keeps an empty lane root in Git
)

SHELL_KEYWORDS = {"if", "then", "else", "elif", "fi", "do", "done", "while",
                  "until", "!", "{", "}", "esac"}
INTERPRETERS = {"perl", "bash", "sh", "python3", "python"}
ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
ROOT_WORD = re.compile(r"^tests(?:/[^\s:]*)?$")
COLLECT_PATTERN = re.compile(
    r"collect_tests\(\s*\$\w+\s*,\s*qr/((?:[^/\\]|\\.)*)/[a-z]*\s*\)")


class Lane:
    def __init__(self, script, roots):
        self.script = script
        self.roots = roots
        self.grading = False

    def describe(self, root):
        kind = "test" if self.grading else "ref-test"
        return f"{kind}: {self.script.relative_to(root)} {' '.join(self.roots)}"


def make_environment():
    env = dict(os.environ)
    for name in ("MAKEFLAGS", "MFLAGS", "GNUMAKEFLAGS", "MAKELEVEL",
                 "MAKEOVERRIDES", "TEST", "CPPGM_BATCH_TESTS"):
        env.pop(name, None)
    env["LC_ALL"] = "C"
    return env


def make(pa_dir, scratch, *arguments):
    # MAKE=true keeps recursive makes (the dev rebuild) from running, and
    # OBJ keeps the Makefiles' directory creation out of the build tree.
    return subprocess.run(
        ["make", "--no-print-directory", "-C", str(pa_dir), "MAKE=true",
         "CPPGM_SKIP_DEV_REBUILD=1", f"OBJ={scratch}",
         f"GENERATED={scratch}/generated", *arguments],
        capture_output=True, text=True, env=make_environment()).stdout


def lane_targets(pa_dir, scratch):
    targets = set()
    for line in make(pa_dir, scratch, "-qp").splitlines():
        match = re.match(r"^([A-Za-z0-9_.-]+):(?!=)", line)
        if match and (GRADING_TARGET.match(match.group(1)) or
                      REGENERATING_TARGET.match(match.group(1))):
            targets.add(match.group(1))
    return sorted(targets)


def tokenize(line):
    lexer = shlex.shlex(line, posix=True, punctuation_chars=";&|()<>")
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        return list(lexer)
    except ValueError:
        return line.split()


def substitute(word, bindings):
    for name, values in bindings.items():
        for spelling in (f"${{{name}}}", f"${name}"):
            if word == spelling or word.startswith(spelling + "/"):
                return [value + word[len(spelling):] for value in values]
    return [word]


def simple_commands(line):
    """Split one shell line into simple commands, expanding the words of
    the for-loops it contains."""
    commands = []
    command = []
    for token in tokenize(line) + [";"]:
        if token and set(token) <= set(";&|()<>"):
            if command:
                commands.append(command)
            command = []
        elif command or token not in SHELL_KEYWORDS:
            command.append(token)
    bindings = {}
    expanded = []
    for command in commands:
        if command[0] == "case":
            continue
        if command[0] == "for" and len(command) >= 3 and command[2] == "in":
            bindings[command[1]] = [word for value in command[3:]
                                    for word in value.split()]
            continue
        expanded.append([piece for word in command
                         for piece in substitute(word, bindings)])
    return expanded


def lane_from_command(words, pa_dir, root):
    while words and ASSIGNMENT.match(words[0]):
        words = words[1:]
    if words and words[0] == "env":
        words = words[1:]
        while words and (words[0].startswith("-") or ASSIGNMENT.match(words[0])):
            words = words[1:]
    if words and words[0] in INTERPRETERS:
        words = words[1:]
        while words and words[0].startswith("-"):
            words = words[1:]
    if not words or "/" not in words[0]:
        return None
    script = Path(os.path.normpath(pa_dir / words[0]))
    if not str(script).startswith(str(root) + os.sep) or not script.is_file():
        return None
    roots = [piece.rstrip("/") for word in words[1:] for piece in word.split()
             if ROOT_WORD.match(piece)]
    return Lane(script, roots) if roots else None


def derive_lanes(pa_dir, root):
    """The Makefile's lane targets and the lanes their recipes run."""
    lanes = {}
    with tempfile.TemporaryDirectory(prefix="fixture-lanes.") as scratch:
        targets = lane_targets(pa_dir, scratch)
        for grading in (True, False):
            pattern = GRADING_TARGET if grading else REGENERATING_TARGET
            group = [target for target in targets if pattern.match(target)]
            if not group:
                continue
            for batch in ("", "1"):
                text = make(pa_dir, scratch, "-n", "-k",
                            f"CPPGM_BATCH_TESTS={batch}", *group)
                for line in text.replace("\\\n", " ").splitlines():
                    for words in simple_commands(line):
                        lane = lane_from_command(words, pa_dir, root)
                        if lane is None:
                            continue
                        key = (lane.script, tuple(lane.roots))
                        lane = lanes.setdefault(key, lane)
                        lane.grading = lane.grading or grading
    return targets, list(lanes.values())


def selection_for(script):
    """(pattern, prunes) for a runner, None for a non-runner, "list" for a
    runner that lists its selection; LookupError when the source does not
    tell."""
    name = script.name
    if name in NON_RUNNERS:
        return None
    if name in LISTING_RUNNERS:
        return "list"
    if name in SELECTIONS:
        return SELECTIONS[name]
    source = script.read_text(errors="replace")
    patterns = COLLECT_PATTERN.findall(source)
    if not patterns:
        raise LookupError(name)
    walks_itself = re.search(r"^sub collect_tests\b", source, re.M)
    prunes = not walks_itself or "File::Find::prune" in source
    return ("|".join(f"(?:{pattern})" for pattern in patterns), prunes)


def lane_selects(lane, selection, pa_dir, inputs):
    if selection == "list":
        listed = subprocess.run(
            ["perl", str(lane.script), *LISTING_RUNNERS[lane.script.name],
             *lane.roots], cwd=pa_dir, capture_output=True, text=True).stdout
        names = {os.path.normpath(line) for line in listed.splitlines() if line}
        return inputs & names
    pattern, prunes = selection
    regex = re.compile(pattern)
    selected = set()
    for word in lane.roots:
        for start in (os.path.relpath(match, pa_dir)
                      for match in glob.glob(str(pa_dir / word))):
            for path in inputs:
                if path != start and not path.startswith(start + "/"):
                    continue
                below = PurePosixPath(path[len(start):]).parts[:-1]
                if prunes and {"controls", "regression"} & set(below):
                    continue
                if regex.search(path):
                    selected.add(path)
    return selected


def tracked_test_files(root):
    listed = subprocess.run(["git", "-C", str(root), "ls-files", "-z"],
                            capture_output=True, text=True, check=True).stdout
    files = {}
    for path in listed.split("\0"):
        match = re.match(r"^(pa\d+)/(tests/.+)$", path)
        if match:
            files.setdefault(match.group(1), []).append(match.group(2))
    return files


def stem(name):
    for suffix in INPUT_SUFFIXES:
        if name.endswith(suffix):
            return name[:-len(suffix)]
    return None


def external_entries(pa):
    return [entry for entry in EXTERNAL_LANES if entry[0] in ("*", pa)]


def classify(pa, files):
    """(inputs, companions, strays, helpers a fixture must name)."""
    external = [entry[1] for entry in external_entries(pa)]
    inputs = {path for path in files if path.endswith(".t") or
              any(fnmatch.fnmatchcase(path, pattern) for pattern in external)}
    stems = {}

    def add_stem(path):
        directory, _, name = path.rpartition("/")
        if stem(name) is not None:
            stems.setdefault(directory, set()).add(stem(name))

    def companion(path):
        parts = path.split("/")
        return any(parts[depth].startswith(base + ".")
                   for depth in range(1, len(parts))
                   for base in stems.get("/".join(parts[:depth]), ()))

    for path in inputs:
        add_stem(path)
    for path in sorted(files):
        if path not in inputs and path.endswith(INPUT_SUFFIXES) and not companion(path):
            inputs.add(path)
            add_stem(path)
    companions, strays, helpers = set(), [], []
    for path in files:
        if path in inputs:
            continue
        if companion(path):
            companions.add(path)
            continue
        name = path.rpartition("/")[2]
        named = next((needs for pattern, needs in HELPERS
                      if fnmatch.fnmatchcase(name, pattern)), None)
        if named is None:
            strays.append(path)
        elif named:
            helpers.append(path)
    return inputs, companions, strays, helpers


def unnamed_helpers(pa_dir, helpers, fixtures):
    """Helpers that no fixture names, directly or through named helpers."""
    def text(path):
        try:
            return (pa_dir / path).read_text(errors="replace")
        except OSError:
            return ""

    waiting = set(helpers)
    sources = [text(path) for path in fixtures]
    while sources and waiting:
        named = {path for path in waiting
                 if any(path.rpartition("/")[2] in source for source in sources)}
        waiting -= named
        sources = [text(path) for path in named]
    return sorted(waiting)


def check_assignment(root, pa, files, verbose):
    """(problems, verbose report lines) for one assignment."""
    problems, report = [], []
    pa_dir = root / pa
    inputs, companions, strays, helpers = classify(pa, files)
    for path in sorted(strays):
        problems.append(f"{pa}/{path}: neither a fixture input, a companion "
                        "of one, nor a helper")
    for path in unnamed_helpers(pa_dir, helpers, inputs | companions):
        problems.append(f"{pa}/{path}: a helper that no fixture names")

    graded, regenerated, reported = set(), set(), set()
    for _, pattern, script, mention in external_entries(pa):
        matched = {path for path in inputs if fnmatch.fnmatchcase(path, pattern)}
        script_path = root / script
        text = script_path.read_text(errors="replace") if script_path.is_file() else ""
        for path in sorted(matched):
            needle = pattern if mention == "pattern" else path.rpartition("/")[2]
            if needle in text:
                graded.add(path)
            else:
                reported.add(path)
                problems.append(f"{pa}/{path}: EXTERNAL_LANES says {script} "
                                f"runs it, but it does not name {needle}")

    targets, lanes = derive_lanes(pa_dir, root) if (pa_dir / "Makefile").is_file() else ([], [])
    report.append(f"{pa}: {' '.join(targets) or 'no lane targets'}")
    roots = set()
    for lane in lanes:
        try:
            selection = selection_for(lane.script)
        except LookupError:
            problems.append(
                f"{pa}: cannot tell what {lane.script.relative_to(root)} selects "
                f"under {' '.join(lane.roots)}; name it in SELECTIONS or "
                "NON_RUNNERS (scripts/check_fixture_lanes.py)")
            continue
        if selection is None:
            continue
        selected = lane_selects(lane, selection, pa_dir, inputs)
        report.append(f"  {lane.describe(root)}: {len(selected)}")
        (graded if lane.grading else regenerated).update(selected)
        if lane.grading:
            roots.update(lane.roots)
    for path in sorted(inputs - graded - reported):
        if path in regenerated:
            problems.append(f"{pa}/{path}: ref-test regenerates its references, "
                            "but no test lane runs it")
        else:
            problems.append(f"{pa}/{path}: no lane runs this input (test lane "
                            f"roots: {' '.join(sorted(roots)) or 'none'})")
    return problems, report if verbose else []


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=REPO_ROOT)
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("assignments", nargs="*")
    args = parser.parse_args(argv)
    root = Path(os.path.abspath(args.root))
    files = tracked_test_files(root)
    pas = args.assignments or sorted(files, key=lambda pa: int(pa[2:]))
    problems = []
    with ThreadPoolExecutor(max_workers=8) as pool:
        results = pool.map(lambda pa: check_assignment(
            root, pa, sorted(files.get(pa, [])), args.verbose), pas)
        for found, report in results:
            for line in report:
                print(line)
            problems.extend(found)
    for pa, pattern, script, _ in EXTERNAL_LANES:
        if pa in pas and not any(fnmatch.fnmatchcase(path, pattern)
                                 for path in files.get(pa, [])):
            problems.append(f"EXTERNAL_LANES: {pa} {pattern} ({script}) "
                            "matches no tracked file")
    for problem in problems:
        print(problem)
    if problems:
        print(f"fixture lanes: {len(problems)} problem(s)")
        return 1
    print(f"fixture lanes: every input of {len(pas)} assignment(s) is run")
    return 0


if __name__ == "__main__":
    sys.exit(main())
