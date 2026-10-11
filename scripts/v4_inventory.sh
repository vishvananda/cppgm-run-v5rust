#!/usr/bin/env bash
# Inventories for PLAN-CPPGM-EXTENDED-V4.md (docs/v4/): how this tree differs
# from the source tree whose implementation becomes the reference.
#
#   scripts/v4_inventory.sh <source-root> [<out-dir>]
#
# <source-root> is the cppgm-run-v3codex checkout; <out-dir> defaults to
# docs/v4.  Every table is computed from tracked files on both sides.
set -euo pipefail
source_root=$1
out=${2:-docs/v4}
here=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$out"
python3 - "$here" "$source_root" "$out" <<'PY'
import os, subprocess, sys, collections, re
here, src, out = sys.argv[1:4]
def tracked(root, path=''):
    r = subprocess.run(['git', '-C', root, 'ls-files', '-z', '--', path] if path else ['git', '-C', root, 'ls-files', '-z'],
                       stdout=subprocess.PIPE, check=True, text=True)
    return set(p for p in r.stdout.split('\0') if p)
def same(a, b):
    try:
        with open(a, 'rb') as fa, open(b, 'rb') as fb:
            return fa.read() == fb.read()
    except OSError:
        return False
def ext_of(name):
    base = os.path.basename(name)
    m = re.match(r'^[^.]*\.(.*)$', base)
    return m.group(1) if m else ''
here_files = tracked(here); src_files = tracked(src)
pas = [f'pa{i}' for i in range(1, 40)]
# ---- tests
lines = ['# Test inventory', '',
         f'Tracked files under `paN/tests` in this tree (`{os.path.basename(here)}`) against the source tree',
         f'(`{os.path.basename(src)}`).  "same": identical; "diff": modified in place; "source":',
         'only in the source tree; "here": only in this tree.', '',
         '| pa | same | diff | source | here |', '|---|---|---|---|---|']
mod_by_ext = collections.Counter(); only_src_by_ext = collections.Counter(); only_here_by_ext = collections.Counter()
modified_sources = []; only_here = []
for pa in pas:
    h = {f for f in here_files if f.startswith(pa + '/tests/')}
    s = {f for f in src_files if f.startswith(pa + '/tests/')}
    common = h & s
    same_n = 0; diff_n = 0
    for f in sorted(common):
        if same(os.path.join(here, f), os.path.join(src, f)): same_n += 1
        else:
            diff_n += 1; mod_by_ext[ext_of(f)] += 1
            if not re.search(r'\.(ref|my)(\.|$)|\.mir$|\.stdout$|\.expect$', f): modified_sources.append(f)
    for f in s - h: only_src_by_ext[ext_of(f)] += 1
    for f in h - s: only_here_by_ext[ext_of(f)] += 1; only_here.append(f)
    if h or s:
        lines.append(f'| {pa} | {same_n} | {diff_n} | {len(s - h)} | {len(h - s)} |')
def counter_table(title, c):
    rows = [f'| `{k or "(none)"}` | {v} |' for k, v in c.most_common()]
    return [f'## {title}', '', '| extension | files |', '|---|---|'] + rows + ['']
lines += [''] + counter_table('Modified files by extension', mod_by_ext)
lines += counter_table('Files only in the source tree, by extension', only_src_by_ext)
lines += counter_table('Files only in this tree, by extension', only_here_by_ext)
lines += ['## Modified inputs (not references)', ''] + [f'- `{f}`' for f in modified_sources] + ['']
lines += ['## Files only in this tree', ''] + [f'- `{f}`' for f in sorted(only_here)] + ['']
open(os.path.join(out, 'tests-inventory.md'), 'w').write('\n'.join(lines))
# ---- lanes (course and regression in the source tree)
units = collections.defaultdict(set)
for f in src_files:
    m = re.match(r'^cppgm\.tests/(course|regression)/(pa\d+)/(.*)$', f)
    if not m: continue
    lane, pa, rest = m.groups()
    if rest == '.gitkeep': continue
    parts = rest.split('/')
    bucket = '/'.join(parts[:-1])
    name = parts[-1]
    unit = re.sub(r'\.(t|cpp|lowir|program)(\.\d+)?$', '', name) if re.search(r'\.(t|cpp|lowir|program)(\.\d+)?$', name) else None
    if unit: units[(lane, pa, bucket)].add(unit)
lines = ['# Course and regression lanes in the source tree', '',
         'One row per test unit (`x.t`, `x.cpp`, `x.lowir`, `x.program`) with its lane and bucket;',
         'Phase 4 moves each into `paN/tests/<bucket>/` under an audited cluster.', '',
         '| lane | pa | bucket | units |', '|---|---|---|---|']
for (lane, pa, bucket) in sorted(units, key=lambda k: (k[0], int(k[1][2:]), k[2])):
    lines.append(f'| {lane} | {pa} | `{bucket or "(flat)"}` | {len(units[(lane, pa, bucket)])} |')
lines += ['', '## Units', '']
for (lane, pa, bucket) in sorted(units, key=lambda k: (k[0], int(k[1][2:]), k[2])):
    lines.append(f'### {lane}/{pa}/{bucket or "(flat)"}'); lines.append('')
    lines += [f'- `{u}`' for u in sorted(units[(lane, pa, bucket)])]; lines.append('')
open(os.path.join(out, 'lanes-inventory.md'), 'w').write('\n'.join(lines))
# ---- handouts and wrappers
def diff_lines(a, b):
    if not (os.path.exists(a) and os.path.exists(b)): return 'missing'
    r = subprocess.run(['diff', a, b], stdout=subprocess.PIPE, text=True)
    return str(sum(1 for l in r.stdout.splitlines() if l[:1] in '<>'))
lines = ['# Handouts and wrappers', '', 'Diff lines (`<` and `>`) between this tree and the source tree.', '',
         '| pa | README.md | Makefile | other differing files |', '|---|---|---|---|']
for pa in pas:
    others = []
    h = {f for f in here_files if f.startswith(pa + '/') and '/tests/' not in f and f not in (pa + '/README.md', pa + '/Makefile')}
    s = {f for f in src_files if f.startswith(pa + '/') and '/tests/' not in f and f not in (pa + '/README.md', pa + '/Makefile')}
    for f in sorted(h & s):
        if not same(os.path.join(here, f), os.path.join(src, f)): others.append(os.path.basename(f))
    lines.append(f'| {pa} | {diff_lines(os.path.join(here, pa, "README.md"), os.path.join(src, pa, "README.md"))} | '
                 f'{diff_lines(os.path.join(here, pa, "Makefile"), os.path.join(src, pa, "Makefile"))} | {" ".join(others)} |')
for f in ('Makefile', '.gitignore', 'README.md', 'AGENTS.md', 'PROJECT_LAYOUT.md', 'TESTING_AND_REFERENCES.md', 'dev/Makefile', 'dev/frontend_source_sets.mk', 'pa13/lowir.md'):
    lines.append(f'| `{f}` | {diff_lines(os.path.join(here, f), os.path.join(src, f))} | | |')
open(os.path.join(out, 'handouts-inventory.md'), 'w').write('\n'.join(lines) + '\n')
# ---- tree
def bucket3(prefix):
    h = {f for f in here_files if f.startswith(prefix)}; s = {f for f in src_files if f.startswith(prefix)}
    differing = sorted(f for f in h & s if not same(os.path.join(here, f), os.path.join(src, f)))
    return sorted(h - s), sorted(s - h), differing
lines = ['# Tree inventory', '', 'Per area: files only here, files only in the source tree, and common files that differ.', '']
for prefix, title in (('dev/', 'dev (tool mains, build)'), ('dev/src/', 'dev/src'), ('scripts/', 'scripts'), ('doc/', 'doc'), ('docs/', 'docs'), ('docker/', 'docker'), ('shared/', 'shared'), ('student.tests/', 'student.tests'), ('.github/', '.github')):
    only_h, only_s, differing = bucket3(prefix)
    lines += [f'## {title}', '', f'- only here: {len(only_h)}', f'- only in source: {len(only_s)}', f'- differing: {len(differing)}', '']
    if prefix not in ('dev/src/', 'student.tests/'):
        if only_h: lines += ['Only here:', ''] + [f'- `{f}`' for f in only_h] + ['']
        if only_s: lines += ['Only in source:', ''] + [f'- `{f}`' for f in only_s] + ['']
        if differing: lines += ['Differing:', ''] + [f'- `{f}`' for f in differing] + ['']
top_h = sorted({f.split('/')[0] for f in here_files}); top_s = sorted({f.split('/')[0] for f in src_files})
lines += ['## Top level', '', f'- only here: {", ".join(sorted(set(top_h) - set(top_s)))}', f'- only in source: {", ".join(sorted(set(top_s) - set(top_h)))}', '']
open(os.path.join(out, 'tree-inventory.md'), 'w').write('\n'.join(lines))
print('wrote', out)
PY
