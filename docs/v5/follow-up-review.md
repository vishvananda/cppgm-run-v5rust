# Recommendations after the v5 review fixes

Reviewed on 2026-10-11. The main review and reproducible checks used
`0b4ea76e`, compared with the original review at `8a70a2de`. While this review
was running, the branch advanced to `de66c75f`; I inspected the intervening
documentation and allocator changes. They do not resolve the findings below.
The fresh export described below was produced from the isolated review worktree
at `0b4ea76e`, not from the concurrently changing worktree.

The changes address much of the earlier feedback: reference-specific PA24/PA33
controls moved out of the required lanes, absent internal functions can satisfy
size bounds, the LowIR comparison no longer accepts arbitrary inserted work,
PA35 handles child signals and scaling failures, calibration checks both builds,
and the handouts distinguish more implementation suggestions from requirements.
The tighter compilation-time gates and the new reference-parity command also
improve the performance goal's visibility.

I recommend the following additional changes. The first four have concrete
reproductions showing incorrect acceptance or rejection. No implementation or
test changes were made as part of this follow-up; this file records the work.

## 1. Finish PA32 behavioral coverage, especially parameterized and EH fixtures

**Priority: high. Remaining gap in the original correctness finding.**

Locations: [scripts/compare_results_common.pl:483](scripts/compare_results_common.pl#L483),
[scripts/compare_results_common.pl:518](scripts/compare_results_common.pl#L518),
and [pa32/README.md:176](pa32/README.md#L176).

The new execution check is useful, but its actual coverage is substantially
smaller than the plan's count of fixtures with an entry function. I enumerated
the handwritten `pa32/tests/o0` through `o3` fixtures and exercised the same
entry-detection rule and reference-backend build route:

| Disposition | Fixtures |
| --- | ---: |
| Total handwritten fixtures | 176 |
| Expected successful optimizer outputs | 175 |
| Excluded by entry detection: helpers or parameterized entries | 137 |
| Entry detected, but reference program fails to link | 13 |
| Built and executed | 25 |

The 13 link failures involve unresolved helpers such as `may_throw`, `cleanup`,
`safe`, and `end_catch`. Thus several exception-handling optimizer controls still
receive no behavioral check. The implementation silently treats these cases as
success and deletes the diagnostic artifacts.

There is still a demonstrated wrong-result acceptance. In
[pa32/tests/o1/500-shared-loop-inline-policy.ref](pa32/tests/o1/500-shared-loop-inline-policy.ref),
I replaced every `return i64 %...` with `return i64 0`. The actual comparator,
with the object-lowering and new behavior checks enabled, reported `PASS (1/1)`.
Adding a small entry function that calls `shared_a(3)` makes the original exit
with 3 and the mutated output exit with 0.

**Recommended fix:** add deterministic drivers for parameterized/helper-only
fixtures and providers for their external helpers, including both normal and
exceptional paths. Record which fixtures are intentionally structural-only and
why; report behavioral checks run and skipped separately. Unexpected reference
build failures should fail the harness instead of silently weakening a test.
Use independently established expected outcomes where possible. Add mutation
tests against actual helper and EH fixtures, not just self-contained examples.

## 2. Distinguish a normal exit status of 124 from a timeout

**Priority: high. New correctness defect in the PA32 execution check.**

Location: [scripts/compare_results_common.pl:510](scripts/compare_results_common.pl#L510).

`lowir_program_outcome` treats any status 124 from GNU `timeout` as a timeout.
That is also the result when the program itself returns 124. Since a timed-out
input is skipped, this creates another false pass even for an otherwise
executable fixture.

Reproduction: an input and reference containing `return i64 124`, with a
candidate containing `return i64 0`, passed the new comparator. The input
finished immediately; it did not time out.

**Recommended fix:** use a runner that reports timeout separately from child
exit status, as PA35's runtime helper already does. Add a mutation test for this
specific case. Treat an unexpected baseline timeout as a harness failure; an
intentionally nonterminating fixture needs an explicit contract rather than
being inferred from an overloaded exit code.

## 3. Correct the new LSDA encoding support

**Priority: high. New rejection of a valid backend representation.**

Location: [scripts/dump_host_eh_object_facts.pl:506](scripts/dump_host_eh_object_facts.pl#L506).

The length of the call-site table is encoded as ULEB128 independently of the
encoding used for each entry's start, length, and landing-pad fields. The change
currently reads that table-length field using `read_encoded(...,
$call_site_encoding)`.

I compiled a C++ exception-cleanup example with GCC, then changed its assembly
to use `DW_EH_PE_udata4` (`0x03`) for call-site entries, retaining the ULEB128
table length and action fields. Both the original and modified objects linked
and executed correctly: the destructor ran once and the exception reached its
catch. The checker reported the expected LSDA facts for the original object
and **no LSDA facts** for the modified object.

**Recommended fix:** read the table length with `read_uleb128`; use
`read_encoded` only for the encoded entry fields. Add a fixture using fixed-width
entries and a ULEB128 table length, with a real runtime unwind check as well as
the inspection assertion. Keep malformed/truncated-table rejection tests.

## 4. PA27 still requires specific local helpers and naming schemes

**Priority: medium. Incomplete resolution of the local-symbol finding.**

Locations:
[pa27/tests/general/200-local-unnamed-symbol-spelling.inspect.expect:1](pa27/tests/general/200-local-unnamed-symbol-spelling.inspect.expect#L1),
[scripts/check_object_expectations.pl:756](scripts/check_object_expectations.pl#L756),
and [pa27/README.md:203](pa27/README.md#L203).

Replacing literal names with patterns admits more spellings, but a pattern such
as `(?:8__invokeEv|4_FUNEv)` still limits students to two implementation choices.
The checker also requires such a symbol to exist even where no address or body
is needed by the program.

Two reproductions demonstrate the remaining restriction:

- GCC 15's O0 object for the fixture's C++ companion executes successfully, but
  fails the nested-lambda invoker expectation. The nested closure is called
  directly; GCC does not emit the unused conversion invoker being demanded.
- The reference compiler's object passes the inspection. Renaming only its
  local invoker with `objcopy --redefine-sym` to `_local_lambda_thunk` leaves
  execution successful but makes the inspection fail. This changes neither
  instructions nor external linkage.

**Recommended fix:** retain exact names for genuine cross-object ABI contracts.
For local entities, check the behavior and binding of definitions/relocation
targets that must exist, allowing unused helpers to be omitted. Do not require
the Clang or GCC name for a private thunk. Update the fixture's leading comment,
which still states the reference's `$_0` numbering policy as a requirement.

## 5. Make the parity report a direct comparison and report missing results

**Priority: medium. New measurement/reporting issue.**

Locations: [pa35/scripts/parity.pl:37](pa35/scripts/parity.pl#L37) and
[pa35/scripts/parity.pl:69](pa35/scripts/parity.pl#L69).

The report discards absolute compiler time and RSS values from `speed.pl`, keeps
each compiler's ratio to a separately measured host run, then divides those
ratios. Its displayed comparison is therefore:

```text
(student / host_run_2) / (reference / host_run_1)
```

It is not the direct `student / reference` ratio. A deterministic reporting
probe supplied equal compiler times of 1 second and host times of 10 and 5
seconds. The report displayed `2.000` and stated that the reference was ahead,
although the supplied compiler times were equal. The same normalization affects
the reported memory comparison.

The final loop also silently omits nonnumeric or missing rows. Because
`speed.pl` can write `failed` or `timeout` while exiting successfully, a failed
speed measurement can disappear from an otherwise successful parity report.

**Recommended fix:** retain absolute measurements and calculate the direct
ratio from them. For time, run reference and student in paired blocks with A/A
calibration and retain every observation. Host-relative values can remain a
separate contextual column. Print explicit failed/unavailable rows and return
a nonzero status for an incomplete requested comparison. Add report tests for
equal absolute times with different host denominators, missing rows, and
timeouts.

## 6. Complete the performance evidence promised by the plan

**Priority: medium. Unfinished plan items, not demonstrated compiler bugs.**

Locations: [pa35/scripts/parity.pl:55](pa35/scripts/parity.pl#L55),
[pa35/scripts/ab.pl:35](pa35/scripts/ab.pl#L35), and
[pa35/scripts/PA35.pm:382](pa35/scripts/PA35.pm#L382).

The parity command currently reports instruction counts and text size for the
two builds of the student's source. It does not invoke paired timing or enable
cache/branch simulation: `ab.pl` does not pass `--sim` to `measure.pl`. The two
or three larger shared kernels proposed in the consolidated plan have not been
added; the kernel gates still cover the original four kernels.

These are useful measurements, but they leave the central generated-code
runtime comparison incomplete. Instruction count alone cannot establish that a
different scheduling, layout, unrolling, or vectorization choice is faster.

**Recommended fix:** add the planned shared container/virtual-call/string
workloads, and offer paired native timings for the binaries produced by the
reference and student on identical source. Keep simulated metrics clearly
separate from time. Do not tighten the minimum gates again merely to substitute
for measuring reference parity directly.

The cache key is improved, but still identifies the host environment only by
the `g++`-reported `__GLIBCXX__` date, libc version text, and Valgrind version.
It does not capture actual preprocessed input, the linked library binaries,
compiler arguments, hardware-counter environment, or cache-simulation geometry.
The simulator's cache geometry is not fixed in its command. This can reuse
incompatible results after a header/configuration change or when a workspace
and cache move between machines.

**Recommended fix:** key measurements by the effective input and configuration,
record the environment in the result, and either fix simulated cache geometry
or include it in the key. Make cache bypass available through the main gate and
parity commands as well as the low-level measurement command.

## 7. Finish the small student-document and export cleanups

**Priority: low. Confirmed in the fresh student export.**

- [pa35/README.md:121](pa35/README.md#L121) still quotes the previous calibration:
  instruction ratios `1.02 / 1.05`, cycle proxies `1.05 / 1.05`, and text `1.11`.
  The checked-in [pa35/calibration.tsv](pa35/calibration.tsv) instead records
  approximately `0.9665 / 1.0212`, `0.9736 / 1.0116`, and `1.0500`. Update the
  prose or avoid duplicating those volatile numbers.
- Ten sidecars under `pa33/tests/behavior/o1`, `o2`, and `o3` still contain the
  old `scripts/make_ir_expect.pl` provenance comments and temporary/historical
  names such as `fx.mir` and `320-indexed-dead-setup-load.my.mir`. The generator
  does not ship. An example is
  [pa33/tests/behavior/o1/100-split-edge-merge-phi-home.ref.expect:1](pa33/tests/behavior/o1/100-split-edge-merge-phi-home.ref.expect#L1).
  Regenerate/sanitize these through the owning tooling, or omit unused sidecars
  from behavior-only lanes; do not hand-edit generated references.
- [pa35/calibration.tsv:3](pa35/calibration.tsv#L3) still names
  `make -C pa35 ref-test`, whose maintainer target is absent from the export.
  [pa35/README.md:132](pa35/README.md#L132) still describes the release-margin
  procedure. Keep student-relevant calibration provenance and baseline meaning;
  move the release command/procedure to maintainer documentation.

I found no new inappropriate named-person references in the exported teaching
documents. Project links, authorship, and inherited attribution should remain.

## Validation and scope

- `make test-harness` passed: **301 tests across 23 groups**.
- A fresh export from the isolated worktree completed successfully, including
  reference and debug-info regeneration, reference equality checks, bundle
  packaging, and the export's own checks. It verified **20,306 portable
  reference output files**, including **7,962 exit-status files**. The export is
  at `/tmp/cppgm-v5-followup-student`.
- The PA32 coverage audit, actual-fixture mutation, exit-124 mutation, local
  symbol probes, real LSDA unwind probe, and injected parity-report probe were
  run. The parity example tests reporting arithmetic; it is not a new hardware
  performance result.
- Logs are `/tmp/cppgm-v5-followup-harness.log` and
  `/tmp/cppgm-v5-followup-export.log`. Probe scripts and results are under
  `/tmp/cppgm-v5-followup-probes/`; these are temporary local artifacts.
- Full PA35 performance gates, full inception, and a new full strict test report
  were not run in this follow-up. The export and harness checks do not establish
  performance parity or resolve the acceptance/rejection examples above.

Fix the PA32 false passes and LSDA parser first, then the remaining PA27 local
symbol restrictions and parity-report arithmetic. The additional performance
coverage and document cleanup can follow without redesigning the compiler.
