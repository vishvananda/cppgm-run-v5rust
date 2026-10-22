#!/usr/bin/env python3
"""Check the compiler options of required course lanes, including their wrappers.

PA32 introduces IR optimization; PA33 introduces native optimization.
Inspect expanded course recipes (both batch
modes), not host build flags or fixture names. Follow local runner/wrapper
scripts and their literal per-test guards. Separate debug preservation and
solution regression targets do not define an earlier assignment's contract.
"""

import re
from pathlib import Path
import subprocess
import tempfile

from check_fixture_lanes import (
    lane_from_command, lane_selects, lane_targets, make_environment,
    selection_for, simple_commands,
)


OPTIMIZER_PA = 32
NATIVE_OPTIMIZER_PA = 33
POSITIVE_OPT = re.compile(r"(?<![\w/])-O(?:[1-9]\d*|fast|[sgz])?(?![\w-])")
CANDIDATE = re.compile(r"CPPGM_APP_ARGS|cppgm\+\+|lowiropt|lowir2native|candidate|"
                       r"\b(?:\w+_)?(?:app|driver|backend|compiler|command|args|options)\b", re.I)
HOST = re.compile(r"(?:g\+\+|clang\+\+|\bCXX\b|\bHOST_CXX\b|\$host\w*)")
SCRIPT = re.compile(r"(?:\.\./)?scripts/[\w.+-]+\.(?:pl|py|sh)")
TEST_GUARD = re.compile(r"\b(?:if|elsif)\s*\(\s*\$test\s*(=~|!~)\s*"
                        r"/((?:\\.|[^/])*)/[a-z]*\s*\)\s*\{")
INLINE_GUARD = re.compile(r"\bif\s+\$test\s*(=~|!~)\s*/((?:\\.|[^/])*)/[a-z]*")


def positive_options(text):
    return sorted({m.group() for m in POSITIVE_OPT.finditer(text)})


def candidate_options(text, context=None):
    tools = re.findall(r'(?:cppgm\+\+|lowiropt|lowir2native)(-ref)?(?![\w+.-])', text)
    if tools and all(tools):
        return []
    app_args = re.search(r'''CPPGM_APP_ARGS=(?:"([^"]*)"|'([^']*)'|(\S+))''', text)
    if app_args:
        return positive_options(next(value for value in app_args.groups() if value is not None))
    if CANDIDATE.search(context or text) and not HOST.search(text):
        return positive_options(text)
    return []


def candidate_owner(text):
    return NATIVE_OPTIMIZER_PA if re.search(r'lowir2native(?!-ref)', text) else OPTIMIZER_PA


def guarded_tests(tests, match):
    """Only exclude a branch when its literal fixture guard proves it unreachable."""
    try:
        regex = re.compile(match[2])
    except re.error:
        return tests
    return {test for test in tests if bool(regex.search(test)) == (match[1] == '=~')}


def script_requirements(script, tests):
    depth, guards = 0, []
    lines = script.read_text(errors='replace').replace('\\\n', ' ').splitlines()
    for number, line in enumerate(lines, 1):
        if line.lstrip().startswith('#'):
            continue
        closing = re.match(r'\s*(}+)\s*(?:else|elsif)\b', line)
        if closing:
            guards = [(level, g) for level, g in guards
                      if level <= depth - len(closing[1])]
        selected = set(tests)
        for _, guard in guards:
            selected = guarded_tests(selected, guard)
        guard = TEST_GUARD.search(line) or INLINE_GUARD.search(line)
        if guard:
            selected = guarded_tests(selected, guard)
        context = ' '.join(lines[max(0, number - 3):number])
        options = candidate_options(line, context)
        if selected and options:
            yield number, options, selected
        # Count structural braces, excluding quoted strings and literal regexes.
        structural = re.sub(r'''"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*' ''', '', line,
                            flags=re.X)
        structural = re.sub(r'/((?:\\.|[^/])*)/[a-z]*', '', structural)
        old_depth = depth - (len(closing[1]) if closing else 0)
        depth += structural.count('{') - structural.count('}')
        if guard and TEST_GUARD.search(line):
            guards.append((old_depth + 1, guard))
        guards = [(level, g) for level, g in guards if level <= depth]


def local_scripts(text, directory, root):
    for match in SCRIPT.finditer(text):
        candidates = [(base / match.group()).resolve() for base in (directory, directory.parent)]
        candidates.append(root / 'scripts' / Path(match.group()).name)
        for path in candidates:
            if path.is_file() and path.is_relative_to(root):
                yield path
                break


def scan_optimization_requirements(root, pas):
    root = root.resolve()
    findings = {}

    def record(path, options, evidence, pa, owner):
        if int(pa[2:]) >= owner:
            return
        relative = path.relative_to(root).as_posix()
        key = (relative, tuple(options), evidence)
        findings[key] = dict(path=relative, kind='early-optimization',
                             message=f"candidate {'/'.join(options)} requires PA{owner} or later",
                             evidence=evidence)

    with tempfile.TemporaryDirectory(prefix='test-requirements.') as scratch:
        for pa in pas:
            if not re.fullmatch(r'pa\d+', pa) or int(pa[2:]) >= NATIVE_OPTIMIZER_PA:
                continue
            directory = root / pa
            if not (directory / 'Makefile').is_file():
                continue
            targets = lane_targets(directory, scratch)
            target = 'test-course' if 'test-course' in targets else 'test'
            if target not in targets:
                continue
            inputs = {p.relative_to(directory).as_posix()
                      for p in (directory / 'tests').rglob('*')
                      if p.is_file() and p.suffix in ('.t', '.cpp', '.lowir')
                      and not {'regression', 'debuginfo'} & set(p.parts)}
            scripts = {}
            test_owners = {}
            for batch in ('', '1'):
                result = subprocess.run(
                    ['make', '--no-print-directory', '-C', str(directory), '-n', '-k',
                     'MAKE=true', 'CPPGM_SKIP_DEV_REBUILD=1', 'TEST_DEPS=',
                     'RUN_CHECK_DEPS=', f'OBJ={scratch}', f'GENERATED={scratch}/generated',
                     f'CPPGM_BATCH_TESTS={batch}', target],
                    env=make_environment(), capture_output=True, text=True, timeout=30)
                if result.returncode:
                    findings[(pa, 'make')] = dict(
                        path=f'{pa}/Makefile', kind='test-requirement-scan',
                        message='cannot inspect the required course recipes',
                        evidence=result.stderr.strip())
                    continue
                for line in result.stdout.replace('\\\n', ' ').splitlines():
                    options = candidate_options(line)
                    if options:
                        record(directory / 'Makefile', options, f'{target}: {line.strip()}',
                               pa, candidate_owner(line))
                    for words in simple_commands(line):
                        lane = lane_from_command(words, directory, root)
                        selected = inputs
                        if lane:
                            try:
                                selection = selection_for(lane.script)
                                selected = lane_selects(lane, selection, directory, inputs) if selection else set()
                            except LookupError:
                                pass  # Unknown runner: conservatively inspect it for every course input.
                        if not selected:
                            continue
                        owner = candidate_owner(' '.join(words))
                        for test in selected:
                            test_owners[test] = max(test_owners.get(test, owner), owner)
                        for script in local_scripts(' '.join(words), directory, root):
                            scripts.setdefault((script, owner), set()).update(selected)
            visited = {}
            while scripts:
                (script, owner), selected = scripts.popitem()
                selected -= visited.setdefault((script, owner), set())
                if not selected:
                    continue
                visited[(script, owner)].update(selected)
                for number, options, reachable in script_requirements(script, selected):
                    evidence = f'{pa} course: {script.relative_to(root)}:{number}; {sorted(reachable)[0]}'
                    record(script, options, evidence, pa, owner)
                for child in local_scripts(script.read_text(errors='replace'), script.parent, root):
                    scripts.setdefault((child, owner), set()).update(selected)
            for test in inputs:
                for suffix in ('.env', '.flags'):
                    sidecar = (directory / test).with_suffix(suffix)
                    if sidecar.is_file():
                        text = sidecar.read_text()
                        options = positive_options(text) if suffix == '.flags' else candidate_options(text)
                        if options:
                            record(sidecar, options, f'{pa} course {suffix} sidecar', pa,
                                   test_owners.get(test, OPTIMIZER_PA))
    return sorted(findings.values(), key=lambda f: (f['path'], f['evidence']))
