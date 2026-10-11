# v5 review: student-facing material and reference pinning

Review of branch `v5` (112 commits past `main`, head `8a70a2de`), done on
2026-10-11 in the worktree `../cppgm-extended-v5`. The questions were:

1. Which instructions and tests are specific to this implementation and
   would stop a student from finding a better solution, given the v5 goal
   that a student can reach equal or better compile time and generated-code
   performance than the reference?
2. Which student-facing documents mention maintainers, people, or project
   history?

Everything below was checked against the harness (Makefiles, comparison
scripts, sidecars) rather than taken from handout text. Line numbers refer
to the v5 worktree. "Graded" means a lane that the student export's
`make test` or `make test-report` runs; `tests/regression/` lanes are not
graded and are only mentioned where something should move there.

This file is a maintainer record; it must not be added to
`scripts/student_export_documents.txt`.

A second, independent review of the same commit is
[second-review.md](second-review.md). Section 0 evaluates its suggestions
and section 0.1 is the consolidated fix list for both reviews; sections 1-4
are this review's original findings.

---

## 0. Evaluation of the second review

Every claim below was re-checked in this worktree before being accepted.

| Second review item | Verdict | Notes |
| --- | --- | --- |
| §1 PA24 controls prescribe backend decisions (compare chain for a large switch, `scratch_carried_reloads`, deferred-address carrier, copy thresholds, seven exact-MIR fixtures) | Agree | Same finding as 2.4 here; the jump-table example is a good addition: `scripts/check_pa24_native_contracts.pl:351-362` requires at least 16 `cmp.i64 reg, imm` lines and 16 encoded immediate compares, so a jump table fails. |
| §2 PA32 rejects deletion of an internal function; policy fixtures | Agree | Same as 2.3 here (`scripts/expect_ir.pl:296-301`). |
| §3 PA10-PA23 require unnecessary structural agreement | Agree on direction, with a nuance | Branch sense and the retype `copy` are conventions documented in `pa8/lowir.md:1754-1790` and are predictable; the real problems are the undocumented conventions, the dead-region shapes and the superset fallback (2.1 here). Replacing the comparison wholesale with execution is a larger course change; the middle path in 0.1 keeps the comparison, makes it symmetric and moves every enforced convention into the documented list. The PA10-PA23 controls already execute. |
| §4 PA27 pins clang's local-symbol spellings | Agree | Same as 2.5 here (44 lines). |
| §5 Passing PA35 does not establish parity; the 0.80 limits allow 5.6 to 6.2 times the reference's compile time | Agree | Arithmetic from `pa35/calibration.tsv` checks out. The cheap implementation already exists: `pa35/scripts/ab.pl` builds the student's own source with any compiler and compares at constant work, so `ab.pl reference-binaries/cppgm++` is a same-machine, identical-input codegen parity check; `speed.pl --compiler reference-binaries/cppgm++` is the compile-time one. Neither is documented as such. A `parity` report target plus two or three larger shared kernels covers the rest. |
| §6 PA35 separates compiler-speed from codegen work too strictly | Partly agree | The "Boundaries" bullet at `pa35/README.md:223-227` is unenforceable and discourages combined changes; the byte-identity advice at `:327-329` is in the non-normative notes and is sound methodology. Reword the boundary as method; keep `identity` as a diagnostic; the seed/self/inception identity is a separate requirement and stays. |
| §7 PA32 accepts an optimizer that changes a program's result (handwritten lanes) | Agree, HIGH | Verified: `check_source_behavior` returns early for LowIR inputs (`scripts/compare_results_common.pl:503-507`), `check_lowir_lowers` only produces an object, and `.ref.exit_status` records the tool's status. 69 of the 179 handwritten fixtures in `pa32/tests/o0..o3` have an entry function and can be executed; the other 110 are helper-only. |
| §8 scaling passes after a timeout | Agree | `pa35/scripts/scaling.pl:124-131`: growth is taken from the last two successful sizes; a later timeout is ignored. |
| §8 child signals become status 0 | Agree | `pa35/scripts/PA35.pm:111` returns `$? >> 8`. |
| §8 measurement cache omits inputs | Partly agree | The workload is manifest-verified, but the preprocessed text depends on libstdc++, and counts on the valgrind version; neither is in the key (`PA35.pm:348-353`). Add them and a `--no-cache` switch. |
| §8 build discovery assumes the reference layout | Agree | Same as 2.7 here. |
| §8 calibration reuses correctness across builds | Agree | `pa35/maintainer/calibrate.pl:106-108`. |
| §9 maintainer material: `pa35/README.md:109+`, `pa8/lowir.md:1690+`, 365 sidecar comments naming unshipped generators, Makefile comments naming `docs/student-export-root/...` | Agree | The generator scripts are not in `shared_scripts` (`scripts/export_student_repo.sh`), so the comments point at nothing in the export; `pa24/Makefile:17` and `pa32/Makefile:61` name a path that is the repository root in the export. |
| §9 attribution (project URL, `AUTHORS`) should remain | Agree | Same conclusion here. |

Items in this review that the second review does not cover: the comparator's
superset fallback (2.1), the dead terminate guard and `typeid` test in PA21
(2.1), the PA10-PA12 checkers keyed on `object=` and `@operatornew__(`
(2.2), the PA11 `noinline` contradiction (2.2), PA26's helper name, exact
`_Unwind_Resume` counts and LSDA layout facts, the GOT/PIE mismatch, TLS
model, ostream count and feature probes (2.5), the `calibration.tsv`
identity strings, internal version numbers in two PA35 tests, stale `.env`
switches and the clang-22 preference (1), PA1's merged stderr and PA4's
false normalization claim (2.6), the PA32 debuginfo exact lanes, zero-slack
and stale budget lines, `count(push)` and the undocumented PA33 vocabulary
(2.3, 2.4), and the handout mechanism mandates (3).

## 0.1 Consolidated fix list

Priority order. [A] this review, [B] second review, [AB] both.

**P0: harness results that are wrong**

1. [B] PA32 handwritten lanes never execute the optimized program. For
   the 69 fixtures with an entry, run the input and the output through
   `lowir2native-ref -O0` (as `scripts/check_controls.pl` does) and require
   identical stdout and exit status; record the input's outcome as a
   sidecar at `ref-test` time. For the 110 helper-only fixtures, add a
   driver or an outcome sidecar where the fixture has an observable result.
   Add mutation controls to `make test-harness` (change a return constant,
   delete an observable store, invert a branch) that must fail.
2. [A] Make the LowIR comparison symmetric: restrict the fallback at
   `scripts/compare_results_common.pl:4085` to
   `lowir_static_finalizer_action_is_projection`.
3. [B] `pa35/scripts/scaling.pl`: a timeout or failure at any measured size
   is growth `inf`/`failed`, whatever the earlier sizes showed.
4. [B] `pa35/scripts/PA35.pm` `run_capture`: decode signal termination
   (`128 + signal`) instead of `$? >> 8`.
5. [B] `pa35/maintainer/calibrate.pl`: run the correctness group for each
   measured build; a missing result fails.

**P1: graded checks that reject an equal or better design**

6. [AB] PA32: treat an absent function as count 0 for `<=`/`<` predicates
   in `scripts/expect_ir.pl`; move the cap-only fixtures
   (`o1/500-shared-loop-inline-policy`, `o1/400-inline-discardable-size-cap`,
   `o1/400-inline-single-call-caller-budget`, `o1/500-cold-path-discounted-inlining`,
   `o1/400-inline-hint-late-nonleaf`, `o3/500-late-inline-complex-size-cap`,
   `o3/500-full-unroll-small-constant-loops` `@keep_clone_budget`, the three
   `o1/400-inline-*-recursive-callee-blocked`,
   `o1/400-inline-eh-bearing-callee-at-depth-zero-blocked`,
   `o1/400-direct-throw-callee-not-inlined`, `o1/400-retain-unsafe-landing-calls`)
   to `tests/regression` or keep only their outcome lines; fix
   `o1/300-inline-growth-budget` `@add_nine`; give per-function budgets a
   slack floor in `scripts/make_ir_expect.pl` and never tighten on
   regeneration; regenerate the 9 zero-slack and 27 stale lines; add
   location sidecars to `pa32/tests/debuginfo/*`; drop `count(push)` from
   the PA33 generator; delete `pa32/README.md:233-240`'s "apply it from a
   level the fixture does not test".
7. [AB] PA24/PA33 native controls: move to the regression lane, or restate
   as bounds, `910-large-switch-immediate-cases` (allow a jump table),
   `907`/`909` (`rep movsb` required), `911-915` (`strlen_prefix`,
   `copy_bytes_dynamic`), `904-o0-layout-policy`, the
   `scratch_carried_reloads` and `deferred-address-parameter-carrier-reuse`
   behavior checks, and the sidecar-less structural fixtures
   `700-call-setup-forwarding-no-preserve` and
   `800-single-edge-callee-saved-retention`. Give PA24's `-O0` budgets a
   larger tolerance so a plain `-O0` passes. Rewrite `pa24/README.md`'s
   goals 1-18 as contract, bound and example. Reconsider "`-O0` output
   remains PA24's" in PA35 afterwards.
8. [A] PA21: make the reference open a terminate guard only when the body
   can unwind, and drop the `typeid` null test on reference operands;
   regenerate the affected references; delete `pa21/README.md:239-242` and
   `:198-201`.
9. [AB] PA26-PA31 inspection: grade terminate and resume by the runtime
   oracle plus minimum counts (drop `cppgm_call_terminate` and exact
   `_Unwind_Resume` counts); drop `call_site_has_sparse_gap` and
   `call_site_starts_at_zero` from `scripts/dump_host_eh_object_facts.pl`
   and accept any DW_EH_PE encoding, call-relocation kind and
   symbol-or-section target; pin names only for external and weak symbols
   and test local entities by binding (44 lines in PA27/PA28); add a `-pie`
   link lane or accept both relocation classes for imported and defined
   data; accept any TLS model; make emission-policy absence checks advisory
   except where the standard forbids emission; fix the ostream count check
   and the PA29 feature-probe references. Rewrite PA26 items 10-12 and PA27
   items 1, 6, 7 and "Using PA9 ABI Names" as observables.
10. [A] PA10-PA12 checkers: locate functions by role or either name form,
    parse metadata groups, drop shape regexes that `expect exit` already
    covers, accept `div exact` and `cmp lt`; fix `pa11/README.md:93-96`
    (`noinline` is required from PA11); document `cppgm_builtin_memcpy` and
    `memmove`.
11. [AB] `pa8/lowir.md`: state that "the C++ lowering ..." sentences are
    enforced; rewrite them as rules; move the PA10-PA12/PA21/PA23
    conventions into the enforced list or absorb the ones that cost nothing
    (dead `eh_cleanup` regions, continuation sharing); list the silent
    absorptions; list the rejected legacy keys; fix the "ignores names,
    order and layout" summaries in PA10-PA16.

**P2: PA35 measures a floor, not parity**

12. [B, implemented with A's observation] Add `make -C pa35 parity`: run
    `speed.pl --compiler reference-binaries/cppgm++` and
    `ab.pl reference-binaries/cppgm++` (the student's own source built by
    the reference compiler and by the student's, measured at constant work)
    and print the student and reference values side by side for compile
    time, memory, own-binary instructions, cycle proxy and text size, with
    `abba.pl` timing as confirmation. Document it in `pa35/README.md` as
    the reference-parity check; keep the gates as the minimum. Add two or
    three larger shared kernels (containers, virtual calls, strings) so the
    GCC-relative comparison covers more than four loops.
13. [B] Reword the "codegen work changes the optimizer and backend, not
    the compiler's own hot code" boundary (`pa35/README.md:223-227`) as
    method: use fixed-work A/B (`ab.pl`) to attribute a change, and
    `identity` as a diagnostic; combined improvements are allowed.
14. [AB] `build_with`: document the contract (a `make build` whose dry run
    links `cppgm++.tmp` from objects under `obj/dev`) or add a stable
    `dev/Makefile` target that prints compile and link lines, and support
    `OBJ=` overrides. Add libstdc++ release and valgrind version to the
    measurement cache key and a `--no-cache` switch.

**P3: maintainer traces and handout text**

15. [AB] Scrub: `calibration.tsv` machine identity strings and the
    course-build column; `pa35/README.md:109-137` (keep the student-build
    baseline, move release procedure and maintainer build history to
    `pa35/maintainer/`); "previous campaign" in `PA35.pm`; internal version
    numbers in `pa35/tests/behavior/byte_combines.cpp` and
    `branching_unroll.cpp`; stale `.env` switches in PA30/PA31; clang-22
    preference and `CPPGM_HOST_CC` default in the shipped workers;
    `CPPGM_CMAKE_COMPILER`; removed-feature error texts; maintainer-mode
    sentences in `pa8/lowir.md:1690-1711`; the help text's admissions;
    2013-handout comparisons in PA5/PA6; `shared/README.md:7-8`;
    `dev/preproc.cpp:141`.
16. [B] Sidecar comments: have the generators write a comment that explains
    the property ("budget: the reference's count plus 10% and one; outcome:
    what the fixture removes") instead of naming unshipped scripts and
    old fixture names; fix `pa24/Makefile:17` and `pa32/Makefile:61` to
    name `TESTING_AND_REFERENCES.md` at the root.
17. [A] Handout mechanism mandates to non-normative notes, rewritten as
    observables: PA4 pragma-once walk and `asctime`; PA2 decoders; PA3
    parse tree; PA8 model shape (`pa8/README.md:22-31`); PA9 typed-target
    layer and module layout; PA10 symbol-identity sentences; PA14/PA15
    mechanism wording; PA21 internal vocabulary; PA25 serialized-program
    prohibition; PA29-PA31 "no hosted-only path". Label "Later scale" and
    "Handoff" sections optional. Define batch execution once or drop the
    word. State the file audit's status.
18. [A] Harness honesty: PA1 capture profile or documentation of merged
    stderr; PA4 normalization claim; PA4 `##` rule into the contract; PA25
    driver surface (`-D`, `-pedantic-errors`, `-fvisibility`, `-O<n>`,
    `-g0`); PA17/PA18 `N3485 focus` claim and the `// VALIDATION:` header;
    the region invariant into `pa8/lowir.md` Structural Rules; delete the
    dead PA25 sidecar, unreferenced `pa31/scripts/*`, empty
    `.ref.stderr`/`.ref.stdout` sidecars in PA1-PA3; document PA8's debug
    lanes; `pa16` kit listing.
19. [A] PA5-PA7: ship the node vocabulary and the anonymous-entity naming
    rule, or print `<anonymous>`; say the in-memory AST is free.
20. [AB] After the changes: regenerate references, run the strict report,
    `make test-harness`, the PA35 release check, and validate a fresh
    student export (commands, links, test discovery, the PA35 build
    interface with an `OBJ=` override).

---

## 1. Maintainer, people and history references in shipped material

Shipped means: listed in `scripts/student_export_documents.txt`, under
`docs/student-export-root/`, or any file under `paN/`, `scripts/`,
`shared/` or `student.tests/` that `scripts/export_student_repo.sh` copies.

| Where | What | Fix |
| --- | --- | --- |
| `pa35/calibration.tsv:11,14` | `meta kernel Linux 7.1.6-1rodete1-amd64` and `ldd (Debian GLIBC 2.43-4+gl0)`: the maintainer's employer-internal distribution strings. Also records CPU model, processor count and the full `g++`/valgrind version lines. | In `pa35/maintainer/calibrate.pl:195-201`, record only what the comparison needs (`uname -m`, kernel major.minor, host compiler major version, libstdc++ release, valgrind major). Scrub the shipped file at export or regenerate it. |
| `pa35/README.md:121-122` | "One earlier implementation of this course started PA35 at 2.12 / 2.51 own-binary instructions." | Replace with a generic statement ("a backend without register allocation typically starts near 2 to 2.5"). |
| `pa35/README.md:117-121` | Describes the maintainer build (`-fno-strict-aliasing`, jemalloc) to explain the second column of `calibration.tsv`. | Ship only the student-build column in `calibration.tsv` and drop the paragraph; keep the course-build column in a maintainer-only file. |
| `pa35/scripts/PA35.pm:371-372` | "the same classes the previous campaign used, so its numbers stay comparable." | "the classes below". |
| `pa35/tests/behavior/byte_combines.cpp:1-2`, `branching_unroll.cpp:1-2` | Internal iteration numbers of the reference: "7.46 combined them into one wider load; 7.50 dropped that", "fully unrolled at -O3 since 8.6". | Rewrite as what the test checks; 24 of the 93 behavior comments also narrate what "host objects" or "host O2+" do, which is fine as context but should not read as a requirement. |
| `pa32/README.md:185-186` | "The budgets were generated from an earlier course solution's output". Also true (see 2.3): 27 budget lines are tighter than the current rule because they were never regenerated. | Regenerate and say "from the course solution's output". |
| `pa8/lowir.md:1696`, `1709-1711` | "The maintainer byte-exact mode", "`scripts/check_lowir_seams.py`, which the student export does not ship" (the export does ship that script; only the `test-seams` target is stripped, `scripts/export_student_repo.sh:95,296-306`). | Drop both sentences from the student document. |
| `pa8/README.md:181-183` | "The course solution recounts a function's uses each time its body has changed, a cost that later showed up as a noticeable share of its optimizer's work." | "Recounting every use after each edit is measurably expensive; keep counts current incrementally." |
| `pa33/design-notes.md:49-52`, `pa24/design-notes.md:49-56` | Narrative of the reference's later tuning ("One of the largest single steps in the course solution's later tuning", "each disagreement later work found was wrong code", "which later work pays for in spills"). | Keep the lesson, drop the history. |
| `pa21/README.md:341-343`, `pa23/README.md:291-292`, `pa18/README.md:168-170`, `pa17/README.md:229-230` | Earlier-implementation narrative ("constant-time state interning without comparing rendered LowIR", "instead of using one class-wide negative-row count", a regression note on a cache, "the remaining ... timing work that is better framed as..."). | Rewrite as the rule or delete. |
| `pa5/parsing.md:3-4,166-173`, `pa6/scopes-and-types.md:3-5,146-148,164-170` | References to the 2013 handouts and "old recognizer traces", "old extreme stress inputs", "mock memory image" the student never sees. | Keep attribution in `NOTICE`; delete the comparisons. |
| `shared/README.md:7-8` | "README files should use this standard wording:" is an editorial instruction to maintainers. | "Each of those handouts states:". |
| `pa30/tests/compile/700-hosted-function-nullary-base-reentry-compile.env`, `pa31/tests/link/600-hosted-basic-ios-disable-eh-fallback-link-smoke.env`, `.../600-hosted-stringbuf-disable-eh-fallback-link-smoke.env` | `CPPGM_STRICT_TYPE_TEXT_FALLBACKS=1`, `CPPGM_STRICT_SEMANTIC_FALLBACKS=1`, `CPPGM_DISABLE_HOST_EXCEPTION_COMPILE_FALLBACK=1`: switches of an earlier implementation that nothing in `dev/` or `scripts/` reads, and that the shipped file audit forbids a student from honoring (`getenv`). | Delete the lines; rename the two PA31 fixtures. |
| `scripts/run_cpptoolchain_tests_worker.pl:118-123`, `scripts/run_cpphostinterop_tests_worker.pl:150-155` | Shipped workers try `clang-22`/`clang++-22` first, a maintainer-machine toolchain; the PA25-PA28 handouts list a different order. The PA25-PA31 Makefiles never set `CPPGM_HOST_CC`, so C helpers can build with clang-22 while C++ links use `g++`. | Drop the versioned names; set `CPPGM_HOST_CC ?= $(CC)`. |
| `pa29/Makefile:224`, `pa31/Makefile:144` | `CPPGM_CMAKE_COMPILER` has no reader anywhere. | Delete. |
| `scripts/check_pa10_lowir_controls.pl:124-125,139-140`, `scripts/check_pa11_lifecycle_policy.pl:54-55`, `scripts/check_pa11_unreachable_terminator.pl:48-49`, `scripts/compare_results_common.pl:2213-2229` | Error text narrates removed reference features ("retained removed pointer promises", "removed trivial_lifecycle classification escaped into LowIR", "removed source-origin index projection"). | Plain validation messages ("`capture` is not a defined parameter metadata key"). |
| `dev/src/support/tool_help_text.h:3-6,35-36` | Help text shipped as a scaffold admits that `--stats`, `--validate`, `--verify-passes`, `--inline-limit` "are the course solution's own", and documents a `*.lowir` output-name dialect switch that no handout defines. | Trim to contract flags. |
| `dev/preproc.cpp:141` (not shipped; the scaffold replaces it) | `options.author = "Vish Abrams"`. | Neutralize anyway. |

Acceptable and left alone: the project URL in
`docs/student-export-root/README.md:11`, the `AUTHORS` file, "report
suspected reference bugs to this project's maintainers"
(`pa1/README.md:207`), neutral mentions of "the course solution".
`scripts/check_lowir_native_structural_controls.pl` mentions `v4opt` five
times but the export deletes it (`scripts/export_student_repo.sh:316-320`).

No person, handle, employer, codename or AI-agent reference was found in any
`paN/README.md`, design note, graded fixture, sidecar or graded script.

---

## 2. Harness behaviors that reward the reference's shape

Ordered by how directly each one blocks a student whose design is equal to
or better than the reference.

### 2.1 The LowIR comparator (PA10-PA23, PA32 `driver/o0`, PA8 debug lanes)

**Superset passes, subset fails.** `scripts/compare_results_common.pl:4085`
falls back to `lowir_reference_is_generated_projection` after the canonical
texts differ. `lowir_reference_entry_is_projection` (`:3804`) requires each
reference line to appear, in order, as a subsequence of the student's
function; leftover student lines are never examined, and
`lowir_reference_is_generated_projection` (`:3915`) never examines extra
student entries. Verified empirically on
`pa17/tests/general/100-dependent-pack-owner-skips-fixed-partial-nested-definition`:
the reference plus an extra `load` passes, plus an unused extra function
passes, minus one `store` fails. `pa8/lowir.md:1706` says the absorb list
"is the whole list"; this fallback is not on it and predates v5 (commit
`9e17ff76`). Fix: restrict the fallback to the one documented case
(`lowir_static_finalizer_action_is_projection`), so the comparison is
symmetric.

**Descriptive sentences are enforced rules.** Because everything not on the
absorb list is compared instruction for instruction, every "the C++ lowering
produces ..." sentence in `pa8/lowir.md` lines 1-1407 is a requirement the
student cannot recognize as one. The design-pinning ones: conditional values
carried through a slot and "the lowering emits no `phi`" (`:936-941`; no
PA10-PA23 reference contains `= phi`), every parameter stored to a slot at
entry (`:708-710`), the `select` restriction (`:1115-1119`), the ctz/clz
widening sequence (`:1158-1163`), handler numbering (`:1399-1406`), when
`obj` storage type and `classes` are stated (`:499-503`, `:819-823`). The
same holds for `pa10/README.md:443-445` ("every local lives in a slot and
every use loads it ... there is no need to avoid the loads here"), which
reads as advice but is the contract. Fix: in Core Design, state that
sentences about "the C++ lowering" are conventions the PA10-PA23 comparison
enforces, and rewrite each as a rule; state the slot-per-local rule in
PA10's Output Format. (Industry practice also lowers to slots and promotes
later, so this pin costs little performance, but it must be visible.)

**Conventions stated only in a handout, or nowhere.** `pa8/lowir.md:1754-1790`
claims to list every enforced convention, yet PA10-PA12 and PA21 fix more
`-O0` shapes in words (`pa10/README.md:186-189,320-323`,
`pa11/README.md:303-312,337-344`, `pa12/README.md:162-166,180-199,219-222`,
`pa16/README.md:185-187`, `pa21/README.md:236-242,303-305`,
`pa23/README.md:247-248,360-374`). Two of them reject a strictly better
lowering:

- `pa21/README.md:239-242`: a destructor with a `delete` of a trivially
  destructible pointee gets a whole-body terminate guard even though nothing
  in it can unwind (`pa21/tests/general/200-class-value-argument-transfers-caller-cleanup.ref:148-171`).
  The handout itself says functions with regions keep cross-block values in
  memory in the course runtime. A lowering that opens no region fails.
- `pa21/README.md:198-201`: a dynamic `typeid` on a reference operand emits
  a null test and `__cxa_bad_typeid` call (`pa21/tests/general/100-typeid-polymorphic.ref:54-59`,
  `300-rtti-evaluated-call-demand.ref:33-37`); 5.2.8p2 requires the test
  only for `*p`. Omitting the dead test fails.

Fix: change the reference and regenerate both, then delete the sentences;
move the remaining conventions into the enforced list of `pa8/lowir.md` or
absorb them (dead `eh_cleanup` regions, continuation sharing by "equal
lexical destructor suffix", `pa21/README.md:303-305`).

**Smaller comparator items.** `validate_lowir_text` rejects student output
carrying `capture=`/`access=` or `projection=base_subobject|reference_field`
(`:2226-2229`) while `pa8/lowir.md:1679,1738` says `projection` and those
keys are validated-not-compared. The comparison silently drops reference
declarations and weak/internal definitions the student does not emit
(`:3679-3725`), strips `resume` blocks of `unwind=no` functions
(`:3634-3660`) and ignores `alias object` lines (`:2852-2854`); none of that
is in the absorb list, so demand-driven emission is not provably safe from
the text. The handouts' summary "the comparison ignores names, order and
layout" (`pa10/README.md:133-139`, echoed in PA11-PA16) overstates it:
instruction order, slot count, temporaries and block structure are compared.

### 2.2 PA10-PA12 control checkers (graded)

- All PA11/PA12 property checkers locate functions by `object=<mangled>`
  metadata (`scripts/check_pa11_goto_lifetime_cleanup.pl:35-41`,
  `check_pa11_lifecycle_policy.pl:59-62`, `check_pa11_unreachable_terminator.pl:50-52`,
  `check_pa12_survivor_properties.pl:88-131`), which `pa8/lowir.md:1736-1739`
  lists as validated-not-compared and no handout tells the student to
  emit. A student who writes `@_ZN5GuardC1Ei` passes the `.ref` lanes and
  fails five controls with "definition is missing".
- `scripts/check_pa12_survivor_properties.pl:185,199` searches for the
  literal presentation name `call ptr @operatornew__(`; `pa8/lowir.md`
  makes `@` names presentation. Use `role=allocate_memory`, as `:178-179`
  already does for `bad_array_new_length`.
- `scripts/check_pa10_lowir_controls.pl:116-123` requires the undocumented
  runtime identities `object=cppgm_builtin_memcpy`/`memmove` and fixes the
  order `unwind=no` before `object=` inside one group, contrary to the
  group-order absorption (`pa8/lowir.md:1733-1735`).
- `check_pa11_goto_lifetime_cleanup.pl:91-120` requires slot names equal to
  source identifiers (absorbed elsewhere) and a fresh `addr` before each
  destructor call, while `check_pa12_survivor_properties.pl:109-120`
  requires the opposite (reuse of the earlier `addr` temporary). Both are
  the reference's context-specific shapes; the fixtures' `main` already
  encodes the expected trace, so `expect exit 0` would grade them.
- `check_pa12_survivor_properties.pl:206-237` fixes metadata order inside a
  group (`pass=` before `object_bytes=`), requires `this` to carry no other
  key, and requires `linkage=c` (validated-not-compared). `:171-175` is
  vacuous unless the parameter is named `%value`.
- `check_pa10_lowir_controls.pl:55-68` rejects `div exact ..., 8` in favor
  of `shr exact ..., 3`; `check_pa12_survivor_properties.pl:200-202`
  requires `binary sub ..., 3` where `cmp lt` is equally valid.
- `pa11/README.md:93-96` says GNU `noinline` "arrives with PA27", but
  `pa11/tests/controls/420-lifecycle-inline-policy.cpp:10` uses it and
  `check_pa11_lifecycle_policy.pl:63-64` fails without `no_inline=yes`.
  PA27's handout never mentions the attribute.

### 2.3 PA32 expectation sidecars (graded)

Verified: no `scripts/check_lowir_*.pl` script runs in a graded PA32 lane
(`pa32/Makefile:69,140-165,206`: the control bucket is regression-only).
Budgets are `ceil(1.1 n) + 1` (`scripts/make_ir_expect.pl:89-93`), and the
current reference passes all 250 sidecars that have reference IR. The
problems are narrower:

- `scripts/expect_ir.pl:296-301` fails any scoped predicate whose function
  is absent, so every `instructions <= N in @f` line also demands that `@f`
  survive. A student who inlines and prunes `@too_large`
  (`pa32/tests/o1/400-inline-discardable-size-cap`) or `@fifth`
  (`o1/400-inline-single-call-caller-budget`) fails with "no function".
- Fixtures whose only content is the reference's cap, which a more
  aggressive and otherwise correct optimizer fails (the PA35 handout's
  advice, "apply it from a level the fixture does not test", has no level
  above `-O3`): `o1/500-shared-loop-inline-policy` (inlining a 7-instruction
  loop callee into two callers fails three budgets),
  `o3/500-late-inline-complex-size-cap`, `o1/500-cold-path-discounted-inlining`
  (passes only with a cold-path discount in a narrow window),
  `o1/400-inline-discardable-size-cap`, `o1/400-inline-single-call-caller-budget`,
  `o1/400-inline-hint-late-nonleaf`, `o3/500-full-unroll-small-constant-loops`
  (`@keep_clone_budget`; the other two `keep_*` functions are not actually
  pinned), the three `o1/400-inline-*-recursive-callee-blocked` fixtures
  (one-level inlining of a recursive callee fails),
  `o1/400-inline-eh-bearing-callee-at-depth-zero-blocked`,
  `o1/400-direct-throw-callee-not-inlined`, `o1/400-retain-unsafe-landing-calls`
  (the design notes at `pa32/README.md:221-225` invite exactly the
  handler-stack tracking these fixtures punish).
- `o1/300-inline-growth-budget` requires `instructions <= 4 in @add_nine`,
  which needs reassociation of nine constant adds, not the inlining the
  fixture is named for.
- Nine budget lines have zero slack (`o1/500-edge-known-branch-chain`
  `instructions <= 9 in @fold_through_jump_and_unrelated_branch`;
  `pa33/tests/o1/400-loop-and-eh-placement` `instructions <= 11 in @call_crossing_eh_value`;
  `pa33/tests/o2/400-eh-edge-placement-barrier` `mem <= 3`; `count(phi) <= 1`
  with reference 1 in four fixtures; `count(call) <= 1` with reference 1 in
  two), and 27 lines are tighter than the generator rule because they were
  generated from an older output. 200 per-function lines are `<= 3` or
  `<= 4`, where one legitimate extra instruction (a retyping `copy`, an
  unmerged `jump`) fails.
- `pa32/tests/debuginfo/*` (six fixtures, run by `make test-debuginfo`, which
  the handout asks students to run) have no sidecar, so they fall through to
  canonical comparison with the reference: `driver/o3/500-source-full-unroll-debug.ref`
  requires a `select` where a `branch` is equally valid.
- Sidecar headers cite the regression lane's old fixture numbers (239
  files) or scratch names (`fx.mir`).

### 2.4 PA24/PA33 native controls and the `-O0` contract

Inherited from `main` but material to the v5 goal, since PA35 states
"`-O0` output remains PA24's" (`pa35/README.md:283`).

- `scripts/check_pa24_native_contracts.pl` is graded in PA24
  (`pa24/Makefile:57-69`, 14 controls plus three behavior fixtures) and in
  PA33 (`pa33/Makefile:89-91`, five controls). It checks reference selection
  policy by instruction encoding:
  - `907-weakly-aligned-medium-copy-compact` and
    `909-oversized-aligned-copy-compact` require `rep movsb` (bytes
    `f3 a4`) for a 48-byte unaligned and a 72-byte copy; `905/906/908`
    forbid it below those thresholds. Unaligned 16-byte vector moves, which
    GCC emits for these sizes, are faster and fail the test.
  - `pa33/tests/controls/native/911-913` require the `strlen_prefix=16` call
    fact at `-O1`, its absence at `-O0`, and SSE2 `pcmpeqb`/`pmovmskb` bytes
    in `main`; `915` requires `copy_bytes_dynamic` for a builtin `memcpy`
    whose result is unused. Neither spelling appears in any shipped handout
    (`pa33/README.md:100-101` calls these "native correctness controls").
  - `904-o0-layout-policy` requires `frame_pointer keep` and
    `epilogues shared` at `-O0`; `900-scratch-carried-frame-reloads` requires
    a `--stats` counter named `scratch_carried_reloads` to be positive;
    `900-deferred-address-parameter-carrier-reuse` requires a specific
    register-reuse pattern (`[this+104]`).
- PA24's structural fixtures without a sidecar compare the canonical dump
  (`pa24/tests/structural/700-call-setup-forwarding-no-preserve`,
  `800-single-edge-callee-saved-retention`): a placement policy, not an ABI
  fact, at `-O0`. The 117 PA24 budgets were generated from a reference `-O0`
  that keeps values in callee-saved registers across calls and edges; a
  plain `-O0` (every value in a frame slot, as GCC does) can exceed the
  `mem`/`instructions` budgets.
- `pa24/README.md:286-650` lists 18 "goals" with dozens of selection rules
  ("Multiplication by 3, 5, or 9 times a power of two uses one
  indexed-address calculation", "A fixed 16-byte zero may avoid
  string-operation setup with a cleared reserved vector scratch", carried
  reload windows). These are the reference's `-O0` micro-policies presented
  as the contract.
- PA33 sidecars: `count(push) <= 1` is in all 49 generated sidecars and the
  reference never emits `push`, so a dump with explicit callee-saved pushes
  fails; `count(eh_push) == 1`/`count(eh_pop) == 1` (four fixtures) and the
  `load.*`/`store.*` mnemonic convention (`o1/400-volatile-access-emission`)
  are documented only in the deleted `pa33/maintainer/design-notes.md`.

### 2.5 PA26-PA31 object inspection (graded)

The reviewer compiled the fixtures with the host's `g++ 15.2` and
`clang++ 22` and ran the course's inspection on those objects; "fails" below
means a link-valid host object fails the course check.

- `pa26/tests/general/400-shared-terminate-action.inspect.expect:1-3` and
  `pa28/tests/general/200-host-eh-noexcept-terminate.inspect.cmd:2` require
  the reference's helper symbol name `cppgm_call_terminate` and exactly one
  `__cxa_begin_catch` and one `std::terminate` call. g++ and clang fail.
  `pa26/README.md:211-214` (item 10) mandates the design.
- `pa26/tests/general/300-lsda-distinct-cleanup-landings` and
  `400-shared-resume-after-stack-call-cleanup` require exactly two
  `_Unwind_Resume` relocations (g++ emits four); `pa26/README.md:215-218`
  (item 11) mandates "one physical `_Unwind_Resume` terminal".
- `scripts/dump_host_eh_object_facts.pl:476-481` emits LSDA layout facts
  (`call_site_has_sparse_gap`, `call_site_starts_at_zero`) compared for
  exact text equality; clang's dense tables fail two fixtures.
  `pa26/README.md:219-223` (item 12) mandates the sparse layout. The dumper
  also accepts only PLT32 calls (`:343-350`, `-fno-plt` objects fail), one
  DW_EH_PE encoding set (`:439-450`), and section-symbol FDE/LSDA targets
  (`:352-355`).
- 44 `local_defined_symbol_canonical` lines in PA27/PA28 pin clang's
  implementation-defined spellings of unit-local symbols (`$_0` lambdas,
  `__invoke`, `_ZGR` numbering, `_ZTC` as LOCAL); `pa27/README.md:203-204`
  makes them normative. g++ fails six fixtures; clang 22 itself fails two.
  Local names are invisible to the linker.
- `pa27/tests/general/200-host-imported-global-data-got` and
  `pa31/tests/link/700-hosted-imported-global-got-load-link-smoke` require
  GOT-class relocations for imported data; g++'s default `-fPIE` objects
  fail. The harness only ever links `-no-pie`
  (`scripts/run_cpphostinterop_tests_worker.pl:422`), while
  `pa26/README.md:200-201` and `pa27/README.md:273-274` motivate the rule by
  a PIE link.
- Emission-policy absence checks (`pa27/.../100-unreachable-internal-function-pruning`,
  `200-trivial-default-constructor-symbol-pruned`, `300-qualified-inline-member-linkage`,
  `pa28/.../200-host-thunk-emission`, `200-host-destructor-entry-emission`,
  and the closure in `pa31/README.md:137-153`) fail link-valid objects that
  keep an unused local or weak definition.
- `scripts/check_object_expectations.pl:374-381` requires the exporting TU
  to define the `_ZTW` wrapper, a `.tdata` section and a `TPOFF32`
  relocation; g++ fails, and GOTTPOFF/TLSGD sequences fail.
- `pa31/tests/link/700-hosted-ostream-ctor-entry-coalescing.inspect.cmd:3-10`
  requires exactly one `nm` line for the `basic_ostream` constructor; a TU
  that legitimately emits C1 and C2 fails.
- `pa29/tests/preproc/300-feature-probes.t:11-14` requires
  `__has_attribute(always_inline)` and `__has_cpp_attribute(nodiscard)` to
  answer 0 although PA29 requires `always_inline` (`pa29/README.md:329-333`)
  and the handout says lying about an implemented feature is worse than
  lacking it (`:270-274`).

### 2.6 PA1-PA4

- PA1 captures stderr into the compared output (`pa1/scripts/run_all_tests.pl:7`,
  profile `stdin-combined`), so any stderr output on a passing input fails;
  the handout says nothing (`pa1/README.md:68-72`). PA2/PA3 ignore stderr.
- `pa4/README.md:112-113` and `pa4/directives.md:54` claim the comparator
  normalizes `__DATE__`, `__TIME__` and the author macro; it does not
  (`compare_text` is exact). Nothing breaks only because no fixture prints
  them.
- `pa4/macros.md:198-199` states a graded rule (invalid `##` result rejects
  the file, `pa4/tests/macros/300-invalid-token-paste`) only in the optional
  section.
- Eight PA1 fixtures pin `EXIT_FAILURE` while `pa1/README.md:230,473` says
  the suite is "quite forgiving".

### 2.7 PA35 harness

PA35 itself is in good shape for the goal: gates are absolute limits, the
reference's values are shown, GCC and clang pass the kernels, no sidecar
pins a shape, and the scripts accept `--compiler` so the reference binaries
can be measured on the student's machine. Items to tighten:

- `pa35/scripts/PA35.pm:213-252` (`build_with`) reconstructs the self build
  by parsing `make -n -B build` for a link line containing `-o cppgm++.tmp`
  and objects under `../obj/dev/`. A student who restructures `dev/Makefile`
  gets "no cppgm++ link line". The handout never states this contract; say
  it, or give `dev/Makefile` an explicit target that prints its compile and
  link lines.
- The self/seed ratio compares each student's compiler compiled by itself,
  so it is not the same program as the reference's; only the four kernels
  compare against GCC on common code. Consider one larger kernel (a few
  hundred lines of ordinary C++ with containers and virtual calls) so that
  "better than the reference" can be verified on identical input.
- `pa35/README.md:283` "-O0 output remains PA24's" freezes the `-O0`
  policies of 2.4 for the rest of the course.
- The ladder `from` values in `pa35/gates.txt` and the limits were loosened
  in `56a9feaf` (own instructions 1.05/1.08 to 1.08/1.12, kernels 1.25 to
  1.30); the handout's calibration paragraph is consistent with the file.

---

## 3. Handout text: normative mechanism mandates and gaps

Per assignment; "normative" means outside a section labelled non-normative.

- **PA1-PA4.** `pa4/directives.md:143-163` mandates a `stat` per `#include`
  and a `std::set` for `#pragma once` (the contract is at `:165-168`);
  `pa2/README.md:120,236` mandates the scaffold's stream-based decoders;
  `pa4/directives.md:64,88` mandate `std::asctime` and "a string variable
  `__FILE__`"; `pa3/README.md:269-271` mandates a parse tree;
  `pa4/README.md:120-149` ("Later scale") and `pa1/README.md:483-505`,
  `pa2/README.md:539-544` state the reference's representations (interned
  hide-set ids, flag bits, one location space) as imperatives without an
  "optional" label. `pa3/README.md:304` "we are not too worried about
  performance" contradicts the v5 goal. `pa3/README.md:293` cites "the
  usual course restrictions" that no shipped document states. No handout
  before `PROJECT_LAYOUT.md` tells the student to register new sources in
  `dev/frontend_source_sets.mk`, which the shipped Makefile requires.
- **PA5-PA7.** The dump vocabulary is defined only by the references: 132
  node kinds in PA5, additional ones in PA7, and generated names
  (`__anonymous_union_type__1_10`, `__local_type1`, `constructor-action`
  for an anonymous-struct global) that encode the reference's choices
  (`pa6/tests/spec/200-namespace-anonymous-union-injected-members.ref`,
  `pa7/tests/general/100-distinct-anonymous-type-identities.ref`). "Batch"
  execution is referenced in four handouts but defined nowhere a student can
  read; it silently requires `main` to live in `dev/<tool>.cpp`.
- **PA8.** `pa8/README.md:22-31` turns model-shape advice into a
  requirement ("the model's shape is yours, under the requirements above");
  `pa8/Makefile:67-144` defines 13 debug-info lanes the handout never
  mentions; `pa8/.gitignore` names the original course's `lowir2cy86`;
  `lowir_model.h:177-178` cites a section that does not exist.
  `pa8/lowir.md:717-720` describes an output-filename-keyed dialect
  (`*.lowir`) that no handout defines.
- **PA9.** `pa9/README.md:55-60,219-221,452-456` mandate a "typed ABI
  target" layer, a module layout and that later stages "call the same
  encoder directly"; the only observable is the emitted name, and the
  handout's own design note recommends a design these sentences forbid.
- **PA10-PA16.** `pa10/README.md:112-121,377-383` mandate the PA9 module,
  one compact symbol identity and no presentation strings per operand, in
  Output Format; `pa14/README.md:239-242`, `pa15/README.md:218-219`
  describe mechanisms ("deferring", "stale-primary refresh") instead of
  behavior; `pa11/README.md:424-432` puts a rule the comparison enforces
  (bit-field storage-unit width) under "non-normative". `pa16/README.md`
  omits `pa16.gram` from its kit and claims a grammar explorer that does
  not ship.
- **PA17-PA23.** `pa17/README.md:133-136` and `pa18/README.md:96-99` claim
  every test starts with an `N3485 focus` comment (10 of 105 and 26 of 156
  do); 500 fixtures carry an unexplained `// VALIDATION:` header no script
  reads; `pa21/README.md:245-247,258-260,213,217-218` use internal
  vocabulary ("demanded", "unwind snapshots", "result slot"); the region
  invariant that PA21 and PA26 each attribute to the other
  (`pa21/README.md:364-366`, `pa26/README.md:202-204`) is stated in neither
  contract nor in `pa8/lowir.md` "Structural Rules".
- **PA24.** `pa24/README.md:286-650`: see 2.4. The design notes are well
  labelled; the README's goals are not.
- **PA25-PA31.** `pa26/README.md:211-223` (items 10-12) mandate the
  terminate helper, single resume terminal and sparse LSDA;
  `pa27/README.md:203-204,273-274,281-286` make clang's local spellings,
  GOT materialization and dead-function pruning normative;
  `pa29/README.md:166-170`, `pa30/README.md:85-88`, `pa31/README.md:89-93`
  forbid "a hosted-only lowering path that carries backend facts outside
  serialized LowIR", which also forbids passing typed in-memory facts to the
  backend (a legitimate speed design; the only observable is PA32's object
  round trip); `pa25/README.md:341-344` forbids a link-time compilation
  design; `pa25/README.md:99-101` says `-D` is not required while three
  fixtures pass `-D`, `-pedantic-errors` and `-fvisibility=hidden`; no
  handout through PA27 lists `-O<n>` or `-g0` although PA26's `make test`
  passes `-O1` and PA27's controls pass `-g0 -O0`.
- **PA32/PA33/PA34/PA35.** The three-way "contract / quality bar / one
  design" framing is good and the design notes are honest. Remaining:
  `pa32/README.md:233-240` names the policy-pinning fixtures instead of
  fixing them; `pa33/README.md` never defines `strlen_prefix`,
  `copy_bytes_dynamic`, `eh_push`/`eh_pop`, the `load.*`/`store.*`
  mnemonics or the push-free prologue convention its graded lane assumes.
- **Export root.** `docs/student-export-root/AGENTS.md:29-31` mentions the
  shipped file audit without saying whether passing it is required (it is
  not in any exit criterion); the audit's limits (3000 lines per file, 240
  per function, `scripts/cppgm_file_audit.pl:14-26`) could reject a
  generated table file. State that it is advisory, or make it a lane.

---

## 4. Recommended changes for v5, in priority order

1. **Make the LowIR comparison symmetric.** Restrict the projection fallback
   in `scripts/compare_results_common.pl:4085` to the documented finalizer
   case. Then regenerate the two PA21 references that encode a dead
   terminate guard and a dead `typeid` null test, and delete the sentences
   that describe them.
2. **Say which lowering sentences are rules.** In `pa8/lowir.md` Core
   Design, state that "the C++ lowering ..." sentences are enforced
   conventions; rewrite the ones at `:708-710`, `:936-941`, `:1115-1119`,
   `:1158-1163`, `:1399-1406`, `:499-503`, `:819-823` as rules; move the
   PA10-PA12/PA21/PA23 conventions into the enforced list or absorb them;
   add the silent absorptions (dropped declarations, weak definitions,
   `resume` blocks, `alias object`) to the absorb list; fix the
   "ignores names, order and layout" summaries in PA10-PA16.
3. **Fix the PA10-PA12 control checkers** to locate functions by role or
   either name form, to parse metadata groups instead of fixing their order,
   to drop shape regexes where `expect exit` already grades the property,
   and to accept `div exact` and `cmp lt`. Fix the PA11 `noinline`
   contradiction. Document `cppgm_builtin_memcpy`/`memmove`.
4. **PA32 sidecars.** Treat an absent function as count 0 for `<=`
   predicates in `scripts/expect_ir.pl`; move the cap-only fixtures listed in
   2.3 to `tests/regression` or keep only their outcome lines; give
   per-function budgets a slack floor (`max(widen(n), n + 3)`) and never
   tighten on regeneration; regenerate the 9 zero-slack and 27 stale lines;
   add location sidecars to `pa32/tests/debuginfo/*`; drop `count(push)`
   from the PA33 generator.
5. **PA24/PA33 native controls.** Move `907`, `909`, `911-915`,
   `904-o0-layout-policy`, the `scratch_carried_reloads` and
   `deferred-address-parameter-carrier-reuse` checks, and the two
   placement-policy structural fixtures to the regression lane, or restate
   them as upper bounds any good selection meets. Relax PA24's `-O0` budgets
   (a larger tolerance at `-O0`, or budgets measured from a plain frame
   form) so a student may keep `-O0` simple and spend the effort at `-O1`
   and above. Reconsider "`-O0` output remains PA24's" in PA35 once the
   controls are relaxed. Document in `pa24/README.md` the MIR vocabulary
   that PA33's graded sidecars assume.
6. **PA26-PA31 inspection.** Replace the helper-name and exact-count checks
   with the runtime oracle plus minimum counts; drop the two LSDA layout
   facts and accept any DW_EH_PE encoding, call-relocation kind and
   symbol-or-section target; pin names only for external and weak symbols
   and test local entities by binding; add a `-pie` link lane and drop the
   relocation-class assertions, or accept both classes; make the
   emission-policy absence checks advisory except where the standard forbids
   emission; accept any TLS model; fix the ostream count check and the
   feature-probe references. Rewrite PA26 items 10-12 and PA27 items 1, 6, 7
   and "Using PA9 ABI Names" as observable requirements.
7. **Scrub maintainer traces** (section 1): `calibration.tsv` identity
   strings and the course-build column, the "earlier implementation" and
   "previous campaign" sentences, the internal version numbers in two PA35
   behavior tests, the stale `.env` switches in PA30/PA31, the clang-22
   preference in two shipped workers, the removed-feature error texts, the
   maintainer-mode sentences in `pa8/lowir.md`, the help text's admissions,
   and the 2013-handout comparisons in PA5/PA6.
8. **Handout mechanism mandates** (section 3): move PA4's pragma-once walk,
   PA2's decoder requirement, PA3's parse tree, PA8's model-shape
   requirement, PA9's typed-target layer, PA10's symbol-identity sentences,
   PA25's serialized-program prohibition and PA29-PA31's "no hosted-only
   path" into non-normative notes, rewritten as observables. Label the
   "Later scale" and "Handoff" sections optional. Define "batch" execution
   once in `TESTING_AND_REFERENCES.md` or drop the word. State the file
   audit's status.
9. **Harness honesty.** Change PA1's capture profile to `stdin` or document
   the merged stderr; correct the PA4 normalization claim; move PA4's `##`
   rule into the contract; list the real PA25 driver surface (`-D`,
   `-pedantic-errors`, `-fvisibility`, `-O<n>`, `-g0`); document the
   `build_with` contract in PA35 or give `dev/Makefile` a stable target for
   it; delete the dead sidecar `pa25/tests/general/300-main-local-type-abi-context.inspect.expect`,
   the unreferenced `pa31/scripts/*` and the empty `.ref.stderr`/`.ref.stdout`
   sidecars in PA1-PA3.
10. **PA5-PA7 dumps.** Ship the node vocabulary and the anonymous-entity
    naming rule (or print `<anonymous>`), and say the in-memory AST may
    have any shape.
