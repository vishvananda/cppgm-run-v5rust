# v5 review: student freedom, performance, and handouts

Reviewed commit: `8a70a2de` on `v5`, in a detached worktree.

The review covered the branch changes, compiler pipeline, assignment handouts,
exported-document list, test comparators, PA35 measurement scripts, and selected
tests and their acceptance criteria. The emphasis was whether a student can
choose a different implementation and achieve equal or better compilation and
generated-code performance than the reference compiler.

The main release concern is that several required tests encode reference
implementation choices, while some optimizer tests can accept incorrect results.
PA35 also needs harness fixes and a clearer distinction between passing its
minimum gates and matching the reference compiler's performance.

The new guidance on compact tokens, typed semantic facts, avoiding text round
trips, and register allocation provides useful direction. Keep that material,
but distinguish examples and suggested techniques from mandatory properties.
Students should have to preserve language semantics, ABI compatibility, and
published interfaces; they should be free to improve internal representations,
optimization policies, and instruction selection.

## 1. Required PA24 tests prescribe reference backend decisions

**Priority: high.** Several native-code controls reject legitimate alternatives
that could produce faster or smaller code.

- [scripts/check_pa24_native_contracts.pl:351](scripts/check_pa24_native_contracts.pl#L351)
  requires at least 16 literal `cmp.i64` operations and native immediate compares
  for a large switch. A valid jump table can fail this requirement.
- [scripts/check_pa24_native_contracts.pl:537](scripts/check_pa24_native_contracts.pl#L537)
  requires a `scratch_carried_reloads` statistic with a positive count. An
  allocator that avoids the reloads should be allowed to succeed.
- [scripts/check_pa24_native_contracts.pl:546](scripts/check_pa24_native_contracts.pl#L546)
  requires an incoming `%this` register, an `rbp` frame home, and particular
  subsequent loads for a deferred-address parameter case. Keeping the value
  safely in a register is a legitimate alternative.
- The copy controls around
  [scripts/check_pa24_native_contracts.pl:391](scripts/check_pa24_native_contracts.pl#L391)
  and the requirements in [pa24/README.md:556](pa24/README.md#L556) prescribe
  vector chunks, a reserved vector scratch register, and `rep movsb` at specific
  size and alignment boundaries. These are instruction-selection policies, not
  general correctness conditions.

Seven successful strict or structural fixtures also lack `.ref.expect` files,
leaving full raw or canonical MIR comparison as their acceptance criterion:

```text
pa24/tests/strict/100-object-abi-lowered.t
pa24/tests/structural/800-single-edge-callee-saved-retention.t
pa24/tests/structural/200-stack-arguments-beyond-six.t
pa24/tests/structural/700-call-setup-forwarding-no-preserve.t
pa24/tests/structural/600-indirect-mixed-gpr-xmm-call-abi.t
pa24/tests/structural/500-mixed-gpr-xmm-call-abi.t
pa24/tests/structural/800-runtime-zero-only-global-pointer-alignment.t
```

The strict cluster has 40 tests and 38 expectation files; its other uncovered
case is a rejection test. The structural cluster has 85 tests and 79 expectation
files, accounting for the six successful structural cases above.

**Recommended change:** move reference-specific policy checks into maintainer
regressions. Student acceptance should check behavior, ABI rules, truthful MIR,
and justified quality bounds. For example, check correct switch results and
reasonable dispatch cost instead of requiring a compare chain; check preservation
across calls instead of requiring a spill and reload. Convert successful exact
MIR controls to explicit properties where the exact representation is not itself
the assignment's interface.

## 2. PA32 can reject a better optimizer for deleting internal functions

**Priority: high.** Function-scoped upper bounds require the named function to
remain present. In [scripts/expect_ir.pl:296](scripts/expect_ir.pl#L296), a missing
function fails even when the predicate is an upper bound on its instruction
count.

For example,
[pa32/tests/o1/400-inline-discardable-size-cap.ref.expect:8](pa32/tests/o1/400-inline-discardable-size-cap.ref.expect#L8)
requires `instructions <= 566 in @too_large`. The input contains internal helpers
with hundreds of identical nonvolatile stores to the same global. A valid
optimizer can eliminate the redundant stores, inline the remaining work, and
remove both helpers. This candidate is sufficient for the test's behavior:

```text
global @sink : i64 = 0
function @main(%value : i64) -> i64 [role=entry] {
  block ^entry:
    store i64 %value, @sink
    return i64 %value
}
```

The candidate passed object lowering but failed the comparator with
`instructions <= 566 in @too_large: no function @too_large in output`.

[pa32/README.md:234](pa32/README.md#L234) also explicitly pins policies through
controls such as:

- `o1/500-shared-loop-inline-policy`
- `o1/400-inline-discardable-size-cap`
- `o3/500-late-inline-complex-size-cap`
- `o3/500-full-unroll-small-constant-loops`

Advice to defer another policy to a different optimization level does not fully
resolve this constraint, especially at O3.

**Recommended change:** distinguish required symbol presence from optional
function-scoped upper bounds. Removing an internal function should satisfy a
size upper bound unless another predicate explicitly requires its presence.
Separate inlining and unrolling policy regressions from required student tests.
Use behavior and meaningful runtime/code-size objectives to judge alternative
policies.

## 3. Earlier LowIR assignments still require unnecessary structural agreement

**Priority: high.** The relaxed LowIR comparison is more permissive than byte
comparison, but still constrains semantically equivalent implementations.

[pa8/lowir.md:1754](pa8/lowir.md#L1754) describes requirements including source
branch sense, particular retyping forms, metadata agreement, and ordering of
independent instructions. A comparison using `cmp ne` cannot simply become
`cmp eq` with swapped branch targets. A probe confirmed that both versions
returned 7 while the relaxed comparator rejected the alternative.

[pa12/README.md:180](pa12/README.md#L180) explicitly requires cleanup regions
around calls that cannot throw because that is where the reference places them.
Other examples include prescribed O0 copy-elision behavior, NRVO result-slot
shapes, `copyobj` prefixes, and the constant-data array initialization strategy
in [pa10/README.md:320](pa10/README.md#L320).

Some typed facts and lifetime distinctions are essential, but agreement with
the reference's exact placement or optional optimization decisions is a
different requirement. In particular, metadata must be sound; that does not
automatically require every valid producer to emit an identical set of optional
facts.

**Recommended change:** retain the documented LowIR interface and required
semantic facts, but use executable behavior, ABI checks, and lifetime checks as
the principal acceptance criteria. Convert exact reference shapes into examples
or maintainer regressions where appropriate. Avoid trying to solve this by
building an increasingly elaborate equivalence canonicalizer.

## 4. PA27 requires implementation-chosen local names to match Clang

**Priority: medium.** [pa27/README.md:203](pa27/README.md#L203) requires details such
as local `$_n` names, `__invoke`, and numbering based on first mangling.
[pa27/tests/general/200-local-unnamed-symbol-spelling.inspect.expect:1](pa27/tests/general/200-local-unnamed-symbol-spelling.inspect.expect#L1)
checks complete spellings, including:

```text
_ZZ13unnamed_localvENK3$_03getEv
_ZZ7invokervEN3$_08__invokeEv
_ZZZ6nestedvENK3$_0clEvENUlvE_8__invokeEv
```

Related controls also inspect internal entities and lambda naming choices.

**Recommended change:** retain names required for external ABI compatibility,
linkage, COMDAT behavior, and runtime interoperability. For implementation-chosen
local symbols, check binding, uniqueness, relocation targets, and behavior.
Treat exact Clang internal spelling as an optional compatibility target unless
there is a demonstrated cross-object requirement.

## 5. Passing PA35 does not establish reference performance parity

**Priority: high for the stated course goal.** The gates in
[pa35/gates.txt:15](pa35/gates.txt#L15) include:

| Measurement | Gate |
| --- | --- |
| O0 compilation time relative to GCC O0 | At most 0.80 |
| O3 compilation time relative to GCC O2 | At most 0.80 |
| Compilation RSS ratio | At most 1.00 |
| Scaling growth | At most 3.00 |
| Self/seed generated-code instruction ratio | At most 1.08 at O0 or 1.12 at O3 |
| Generated-code cycle proxy ratio | At most 1.09 or 1.10 |
| Generated `.text` size ratio | At most 1.15 |
| Four kernel instruction-count ratios against GCC | At most 1.30 |

The checked-in calibration in
[pa35/calibration.tsv:25](pa35/calibration.tsv#L25) records reference compiler
time ratios of approximately 0.142328 at O0 and 0.128986 at O3 in the student
configuration. On that recorded calibration, the 0.80 limits permit roughly
5.6–6.2 times the reference compilation time. These are calculations from
checked-in measurements, not new timings from this review.

Self-versus-seed comparisons also use each student's own compiler source, so
different students are measuring different generated programs. The four common
kernels provide useful shared coverage, but do not establish general parity.

**Recommended change:** retain achievable minimum PA35 gates and add a distinct,
explicit reference-parity target. Compare a pinned reference binary and the
student compiler on the same machine and frozen inputs. Include several shared
program families, and report compilation wall time, CPU time, memory, generated
code runtime, and code size separately. Use instruction counts and cycle proxies
as diagnostics alongside repeated native timing, with a documented way to
justify faster code that uses more instructions or space.

## 6. PA35 unnecessarily separates compiler-speed and code-generation changes

**Priority: medium.** [pa35/README.md:223](pa35/README.md#L223) restricts the kinds
of changes counted as code-generation work, while
[pa35/README.md:327](pa35/README.md#L327) requires compiler-speed changes to
preserve output bytes.

A representation, lookup-table, traversal, or combined frontend/backend change
can legitimately improve both compilation time and generated code. Such a change
may alter emitted ordering or optimization decisions while preserving behavior.
Byte equality across revisions is therefore too strong as a universal admission
rule.

**Recommended change:** require fixed-work A/B comparisons and use byte identity
as a useful isolation diagnostic. Permit combined improvements when correctness
and performance evidence support them. Preserve same-version deterministic
output and the intended self-hosting/inception identity checks; those serve a
different purpose from requiring all optimization revisions to emit identical
bytes.

## 7. PA32 can accept an optimizer that changes a program's answer

**Priority: high.**
[pa32/tests/o1/100-local-const-fold-dce.t](pa32/tests/o1/100-local-const-fold-dce.t)
computes 7. Its expectation file primarily imposes instruction, block, and
operation-count budgets. It does not require the result to remain 7.

[scripts/compare_results_common.pl:503](scripts/compare_results_common.pl#L503)
returns early from the source-behavior check for handwritten LowIR. The object
lowering check establishes that an object can be produced, not that its behavior
matches the input.

A probe changed the optimized result from `return i64 7` to `return i64 0`.
The comparator passed with the actual lowering tool enabled. Executing the
generated programs produced exit statuses 7 and 0 respectively. This is a
demonstrated false positive, not just a hypothetical coverage gap.

The C++ driver lane also compiles source and emitted forms using the same
optimizer, so agreement there is not an independent correctness oracle for all
shared optimizer defects.

**Recommended change:** add behavioral checks for handwritten LowIR, using
known outputs or an unoptimized/independent reference through a trusted backend.
Provide drivers for helper-only inputs. Add harness mutation controls that
change return constants, delete observable stores, or reverse branches, and
require those candidates to fail. Quality bounds should supplement correctness.

## 8. PA35 harness defects and portability constraints

| Issue | Evidence and impact | Recommended fix |
| --- | --- | --- |
| Scaling can pass after a timeout | [pa35/scripts/scaling.pl:124](pa35/scripts/scaling.pl#L124) handles two successful samples before the recorded stop/failure. Injected base/size timings of 0.01, 0.20, and 0.40 seconds followed by a timeout at size 4000 yielded growth 2.05 and exit status 0. | Process timeout/failure status before accepting a growth ratio. |
| Child signals can become success | [pa35/scripts/PA35.pm:111](pa35/scripts/PA35.pm#L111) returns `$? >> 8`, dropping signal bits. A child killed with SIGKILL was reported as status 0. | Decode signal termination and return a failure status. |
| Measurement cache omits relevant inputs | [pa35/scripts/PA35.pm:348](pa35/scripts/PA35.pm#L348) keys cached measurements using a shortened binary hash, optimization level, and measurement kind, without the full workload/configuration identity. | Include workload and header hashes, relevant flags/libraries, platform, and measurement-tool version; provide a no-cache mode. |
| Build discovery assumes the reference layout | [pa35/scripts/PA35.pm:196](pa35/scripts/PA35.pm#L196) scrapes `make -n -B build`, expects `-o cppgm++.tmp`, and recognizes objects under `../obj/dev/`. A probe using `OBJ=../obj/review-alternate` failed with “found no compile lines for cppgm++”. Whitespace splitting adds another portability problem. | Define an explicit build manifest or supported Make target instead of inferring the build from reference-specific command text. Support isolated object directories. |
| Calibration reuses correctness results across different builds | [pa35/maintainer/calibrate.pl:106](pa35/maintainer/calibrate.pl#L106) copies student-build correctness to the course build, although the course build adds `-fno-strict-aliasing` and jemalloc while the student build uses plain O3 and the system allocator. | Run correctness for each measured build configuration and treat missing results as failures. |

The scaling example used injected timing results to exercise control flow; it
was not a hardware performance measurement. The signal and alternate-object-path
cases were exercised directly.

## 9. Student-facing maintainer material

The exported-document scan covered the 48 Markdown manifest entries and the
root documents supplied by the student export.

- [pa35/README.md:109](pa35/README.md#L109) includes reference build history,
  private configuration details, release/calibration procedures, and a reference
  test command whose target is not exported. Keep concise baseline and
  reproducibility information where it helps students interpret results; move
  release procedures and private build history into `pa35/maintainer/`.
- [pa8/lowir.md:1690](pa8/lowir.md#L1690) discusses maintainer byte-exact mode and
  points to `pa11test-seams` / `scripts/check_lowir_seams.py`, which are not
  shipped. Remove or relocate these passages from student teaching material.
- 365 non-regression expectation files contain comments referring to unshipped
  generators such as `scripts/make_ir_expect.pl` and
  `scripts/make_ir_outcome.pl`, or historical file names. Update generator/export
  behavior so shipped comments explain the checked property. Keep generation
  provenance in maintainer material; do not hand-edit generated references.
- [pa24/Makefile:16](pa24/Makefile#L16), and a similar PA32 comment, point students
  to `docs/student-export-root/TESTING_AND_REFERENCES.md`. In the export, the
  document is installed at the root. Use the exported path in shipped guidance.

No inappropriate references to named maintainers or other people were found in
the exported teaching prose. The project URL, `AUTHORS`, and inherited notices
are legitimate attribution and should remain. In particular, the presence of
the repository owner's name in attribution is not a reason to remove it.

Worked examples from the reference compiler are useful when clearly labeled as
examples. The problem is presenting their incidental choices or maintainer
workflow as student requirements.

## Recommended v5 changes

1. Fix the demonstrated false passes: PA32 wrong-result acceptance, PA35 timeout
   handling, and child signal handling.
2. Remove implementation-specific student gates in PA24 and PA32. Allow deleted
   internal functions to satisfy optional upper bounds, and replace successful
   exact-MIR controls with explicit required properties.
3. Audit earlier LowIR controls and PA27 naming requirements against semantics
   and actual ABI obligations. Retain exact reference forms as maintainer
   regressions where useful.
4. Add a reference-parity performance target alongside minimum PA35 passing
   gates, and allow properly measured combined compiler/code-generation gains.
5. Make PA35 build discovery, cache identity, and calibration correctness robust
   across supported student implementations and isolated build directories.
6. Clean up student-facing maintainer references through the source documents
   and export/generation logic, preserving attribution.
7. Validate a fresh student export after these changes, including the exported
   commands, links, test discovery, and PA35 build interface.

## Validation performed

The following checks passed on the reviewed worktree:

| Check | Result |
| --- | --- |
| `make -j8 CXX=g++ CPPGM_HOST_CXX=g++ build` | Passed |
| `CPPGM_LOWIR_DIRECT_TEXT_COMPARE=1 make test-report CXX=g++ CPPGM_HOST_CXX=g++ TEST_REPORT_ASSIGNMENT_JOBS=3 TEST_REPORT_SUBTEST_JOBS=4` | 6,649 / 6,649 passed |
| `make test-harness` | 294 tests across 23 groups passed |
| `make test-debuginfo CXX=g++ CPPGM_HOST_CXX=g++` | Passed |
| `PA35_JOBS=6 make -C pa35 test-smoke` | 3 / 3 correctness gates passed; included 93 behavior cases and 17 generated programs using seed 5 and modes 0–3 |
| `make -j8 -C pa34 test-through-pa5 CXX=../dev/cppgm++ CPPGM_HOST_CXX=g++` | Passed through PA5: PA1 58, PA2 26, PA3 20, PA4 106, PA5 193 |

Targeted probes additionally demonstrated the PA32 wrong-result acceptance,
rejection of valid internal-function removal, rejection of equivalent branch
inversion, PA35 timeout false pass, signal-status loss, and alternate-object-path
build discovery failure described above.

Full inception, the full PA35 performance gates, and a fresh student export were
not run. Calibration ratios in this report come from the checked-in calibration
file. Passing the reference's suites does not resolve the acceptance-criteria
issues demonstrated by the probes, and this review is not a proof of exhaustive
compiler correctness.

Build and validation logs were retained under `/tmp/cppgm-v5-review-*.log`.
Targeted probe scripts were retained in `/tmp/cppgm-v5-review-probes/`; these are
temporary local artifacts rather than committed tests. No compiler, handout,
test, or generated-reference files were changed during the review. This report
is the only added repository file.
