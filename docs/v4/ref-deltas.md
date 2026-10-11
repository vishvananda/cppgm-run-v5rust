# Reference deltas (Phase 3 tracker)

Every difference between the references this tree carried before v4 and
the references regenerated from the new implementation is recorded here
once, by class: (a) a LowIR contract change, with the `pa13/lowir.md`
section that states it; (b) a compiler fix, with the fixture that shows
it; (c) a presentation difference the relaxed comparison absorbs.  A delta
that fits no class is a defect to fix.

## Fixtures that leave with the contract

Nine PA13 `tests/spec` fixtures existed only in this tree; the source tree
had removed them when the LowIR contract was minimized
(`docs/implemented/v3/PLAN-LOWIR-CONTRACT-MINIMIZATION.md`; commits
`e87c358c` "remove unused capture and access metadata" and `5a891f80`
"remove decay source provenance" in the source history).  They exercise
metadata the contract no longer has, and six of them fail outright under
the new `lowir2cy86` because that metadata is now rejected:

| fixture | forms | under the new tool |
|---|---|---|
| `200-call-boundary-metadata` | call boundary metadata | rejected |
| `200-index-projection-metadata-smoke` | `index` projection metadata | rejected |
| `200-parameter-access-metadata-smoke` | parameter `access=` | rejected |
| `200-parameter-capture-metadata-smoke` | parameter `capture=` | rejected |
| `200-prototype-relaxed-arity-smoke` | relaxed prototype arity | rejected |
| `200-unary-decay-pointer-smoke` | `unary decay` | rejected |
| `200-bad-nonptr-parameter-access` | `access=` on a non-pointer (negative) | still rejected |
| `200-bad-nonptr-parameter-capture` | `capture=` on a non-pointer (negative) | still rejected |
| `200-bad-unary-decay-nonptr` | `unary decay` on a non-pointer (negative) | still rejected |

Disposition: dropped with the contract (Phase 2).  The three negative
cases pass only because the whole construct is now invalid; keeping them
would document metadata a student must not emit.

## Regeneration (`make ref-test`, `make ref-test-debuginfo`)

Every reference was regenerated from the new build through the maintainer
wrappers (`dev/<tool>-ref -> <tool>`).  What changed, and why each change
is what it is:

### LowIR references: 1645 files, all class (c)

Every changed `.ref` was re-compared with its previous version through the
relaxed LowIR comparison (old as reference, new as output): all
1645 compare equal, and 0 differ.  The
previous references carried forms the comparison tolerates in a reference
but the compiler no longer emits: `unary decay` (removed with the decay
provenance, `pa13/lowir.md` "Memory and Addressing"), `pass=reference`
spelled for `pass=by_address`, legacy trivial-lifecycle calls and the
unreachable role.  CI compares byte for byte
(`CPPGM_LOWIR_DIRECT_TEXT_COMPARE=1`), so the references had to be brought
to what the compiler emits.  By assignment:

| pa | changed `.ref` |
|---|---|
| pa1 | 3 |
| pa10 | 7 |
| pa11 | 17 |
| pa12 | 29 |
| pa13 | 61 |
| pa15 | 44 |
| pa16 | 167 |
| pa17 | 186 |
| pa18 | 23 |
| pa19 | 149 |
| pa20 | 48 |
| pa21 | 65 |
| pa22 | 131 |
| pa23 | 229 |
| pa24 | 191 |
| pa25 | 96 |
| pa26 | 93 |
| pa27 | 60 |
| pa28 | 42 |
| pa34 | 1 |
| pa37 | 3 |

### Machine IR references: 172 files

84 `.ref.mir`/`.ref.cmir` files are equal after the
harness's normalization (whitespace and layout only).  The other
88 are all `pa29/tests/behavior/*.ref.mir`: that lane
judges program behaviour (`mir_behavior_t`), and its `.ref.mir` beside the
program output is the informational dump of the reference backend, now
the current backend's.

### stdout references

- 258 tracked `.ref.stdout` files were removed by the regeneration:
  every one belongs to a case whose reference exit status is `EXIT_FAILURE`.
  The maintainer harness removes failed-case stdout on purpose
  (`remove_nonportable_reference_stdout`): a diagnostic is not portable
  and the export regenerates it on Linux for the student tree.  They were
  tracked here only because the source tree began as an export.
- 601 `.ref.stdout` files appeared, all empty and all for
  successful cases; this tree tracked 3,073 such files before (2,981 of
  them empty), so they are tracked the same way.
- 31 PA6 `.ref.stdout` files became empty: `recog` no longer echoes
  `empty`/`translation-unit` on standard output, which the `.ref` beside
  each already records.  One PA6 and ten PA34/PA35 stdout references
  changed wording: the diagnostic text of a case the lane accepts with
  the tool exiting successfully (`invalid token ... near TT_LITERAL` is
  now `invalid phase-7 token`; the PA34 hosted-compile diagnostics lost
  their analysis-context trace).  Class (b).
- 8 `.ref.program.stdout` files appeared for behaviour lanes
  that had never tracked them.

### Other new references

22 `.ref.inspect` (PA32/PA33 object inspection) and
10 `.ref` files that the source tree's lanes produced but did
not track.

### Debug-info references (`make ref-test-debuginfo`)

34 files under the `debuginfo` lanes of PA13, PA37 and PA38 changed
(pa13: 17 ??, pa13: 9 M, pa38: 8 M).
These lanes were red against reference-generated files before v4 (the
"known-red" PA13 debug-info baseline); with the references regenerated from
the implementation that is now the reference, `make test-debuginfo` is
expected to pass and is a Phase 2 gate.

### Lanes the regeneration could not reach

`make -C pa37 ref-test` and `ref-test-debuginfo` named test roots that
exist only in the course tree (`tests/o3`, `tests/driver/o3`,
`tests/debuginfo/o3`, ...) and stopped on the first missing one; both
targets now skip a root that is not present, as PA38's already did.
