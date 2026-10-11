# Phase 4: the course and regression lanes moved into the assignment suites

Applied on 2026-09-06 on branch `v4`, on top of the Phase 3 regeneration.

## What moved

5,282 tracked files left `cppgm.tests/course/paN/` and
`cppgm.tests/regression/paN/` by `git mv`; the `paN/course` symlinks and
the two shared directories are gone.  Destinations, by lane:

| destination | files | destination | files |
| --- | --- | --- | --- |
| pa1/tests | 70 | pa22/tests | 106 |
| pa2/tests | 39 | pa23/tests | 50 |
| pa3/tests | 38 | pa25/tests | 73 |
| pa4/tests | 142 | pa26/tests | 30 |
| pa5/tests | 29 | pa27/tests | 3 |
| pa6/tests | 64 | pa28/tests | 11 |
| pa7/tests | 71 | pa29/tests (strict, structural, behavior) | 591 |
| pa8/tests | 84 | pa29/tests/controls | 19 |
| pa9/tests | 61 | pa29/tests/regression | 707 |
| pa10/tests | 32 | pa30/tests | 66 |
| pa11/tests | 31 | pa31/tests | 73 |
| pa12/tests | 63 | pa32/tests | 105 |
| pa13/tests | 65 | pa32/tests/controls | 3 |
| pa14/tests | 16 | pa33/tests | 15 |
| pa15/tests | 44 | pa34/tests (preproc, compile, run) | 30 |
| pa15/tests/controls | 5 | pa35/tests/compile | 200 |
| pa16/tests | 189 | pa36/tests/link | 5 |
| pa16/tests/controls | 4 | pa37/tests (o0 to o3, driver, object-roundtrip, debuginfo) | 503 |
| pa17/tests | 63 | pa37/tests/regression | 768 |
| pa17/tests/controls | 10 | pa38/tests (o1 to o3, behavior, driver, debuginfo) | 227 |
| pa18/tests | 25 | pa38/tests/regression | 367 |
| pa19/tests | 81 | pa20/tests | 36 |
| pa21/tests | 68 | | |

A course fixture went to the bucket of the same name under `paN/tests/`
(flat suites stay flat).  Controls went to `paN/tests/controls/`; the
regression roots of PA29, PA37 and PA38 went to `paN/tests/regression/`
with their bucket structure intact, and the PA37 and PA38 regression
controls to `paN/tests/regression/controls/`.

Two PA1 fixtures collided with local fixtures of the same stem and were
renamed to say what distinguishes them: `100-raw-string-literal` became
`100-raw-string-literal-nested-delimiter`, and `200-header-name` became
`200-header-name-forms`.

`cppgm.tests/` keeps only `undefined/` (inputs the course leaves
unspecified; no lane runs them) and a README that says so.  The regression
lane's policy moved from `cppgm.tests/regression/README.md` into
`docs/student-export-root/TESTING_AND_REFERENCES.md`.

## Harness and Makefiles

- `scripts/run_all_tests_common.pl` and `scripts/CppgmBatchWorker.pm` prune
  `controls` and `regression` subdirectories from a sweep, so `make test`
  over `tests` reaches a control or a regression fixture only through the
  lane that knows how to judge it.
- Every assignment Makefile lost its `COURSE_*` roots and the
  `ifeq ($(COURSE_TEST_ROOT),)` fork: the suite is the one `tests` tree.
  PA15, PA16, PA17 and PA32 read their controls from `tests/controls`;
  PA29 from `tests/controls` and its regression oracles from
  `tests/regression/{strict,structural,behavior}`; PA37 and PA38 from
  `tests/regression/...` with the controls under
  `tests/regression/controls`.  PA31 re-runs the moved
  `330-host-eh-nested-catch-forward-at-o1` fixture at `-O1` after the
  default sweep, as the course lane did.  PA38's driver census root is
  `tests/driver`.
- `make ref-test` now regenerates the PA37 and PA38 regression roots too
  (PA29 already did), so a maintainer's regeneration leaves a clean tree.
- The `INCEPTION_COURSE_STAGE_paN` variables of PA39 had no reader and are
  gone.

## References

`make ref-test` after the move regenerated every reference.  Differences
against the moved course references are of three kinds: references that
embed the fixture path (`start translation unit tests/...`), the expected
byte-level drift of outputs the relaxed comparison already absorbed (the
course references were produced by an older build), and the
`.ref.stdout` of negative fixtures, which the harness rewrites from the
current diagnostics.  Success-case `.ref.stdout` files the course lane did
not track (73 of them) are tracked now, as the local suites always were.

## Results

- `CPPGM_LOWIR_DIRECT_TEXT_COMPARE=1 make test-report` (the CI setting) first
  passed 5,930 of 5,933.  The three failures were the special member order
  violations already recorded in `ref-deltas.md`
  (`pa22/tests/general/300-member-template-assignment-not-special-member`,
  `pa28/tests/general/100-diamond-virtual-destructor-slot-merge`,
  `pa28/tests/general/100-multibase-implicit-virtual-destructor-slot-merge`):
  the compiler emitted the move assignment before the copy assignment when
  the move was demanded first, and the complete-entry vtable thunk before
  the deleting-entry thunk, against the order `pa13/lowir.md` states.  A
  presentation pass in the lowering driver now orders each special member
  family (source commit "Open a constructor loop's cleanup segment before
  the loop"); the three references were regenerated and the report passes
  5,934 of 5,934.
- The PA39 selfhost lane failed on
  `dev/src/native/allocation/location_planning.cpp`: an array of more than
  eight objects with destructors, declared while another such object was
  alive, made the emitted construction loop open the enclosing cleanup
  region once per element and close it once.  The same source commit opens
  the segment before the loop; `pa16/tests/general/300-constructor-array-loop-enclosing-cleanup`
  is the new fixture, and no other reference changed.
- The PA38 regression control
  `tests/regression/controls/459-o3-parameter-address-rematerialization`
  reported that native `-O3` did not reduce call-preserved address
  pressure.  It last passed at `f59dcabf` and failed from `7b97d1e9` ("Keep
  a replayed index base live until its last replay"), which deliberately
  keeps only the base of a replayed index address across a call from `-O1`
  on, so the O2 baseline already has the pressure the control expected O3
  to remove.  The control now pins that decision (only the base is
  preserved at either level, no derived address is rebuilt before the
  call, O3 adds no homes or frame) and the PA38 handout paragraph says the
  same.
- `make -C pa16 test-seams` passes (18 rewrites classified).

## Phase 5: placement and numbering

The placement auditor (`scripts/audit_pa_feature_placement.py`) is now the
only judge of where a fixture sits, and it judges the whole tree:

- The course lane is gone from it (`--include-course`/`--no-course`
  removed); every `paN/tests/**/*.t` outside `regression/` and
  `controls/` is a placed fixture.  The regression lane pins the course
  solution's own outputs and the controls are judged by their checkers,
  so neither is a placed suite.
- PA1 to PA9 must carry a three-digit prefix; from PA10 on the prefix is a
  cluster, a multiple of one hundred (the rule that already existed).
- A numbered companion unit (`x.t.1`) of a host-interop lane (PA31 to
  PA34, PA36) is compiled by the host compiler, so its hosted includes are
  not early.

After the move the auditor reported 740 findings (93 fixtures using a
feature before its owning assignment or cluster, 647 hygiene findings).
`scripts/v4_renumber.py` (a scratch tool, not kept) turned the findings
into 620 renames, applied with `git mv` together with every sidecar:

| reason | fixtures |
| --- | ---: |
| individual number to its cluster (`320-x` to `300-x`) | 253 |
| flat course fixture into the suite's `general/` bucket | 297 (overlapping the rows above and below) |
| unprefixed LowIR-input fixture, cluster by the words its numbered siblings use | 120 |
| unprefixed source fixture, cluster of the latest feature its assignment owns | 43 |
| same assignment, later owning cluster | 29 |
| feature owned by a later assignment: moved there | 24 |
| PA25 fixture whose reference carries unwind lowering: moved to PA26 | 1 |

Two PA17 fixtures used `__attribute__((noinline))` only decoratively; the
attribute went instead of the fixtures.  The ledgers
(`doc/lowir-contract-ledger.tsv`, `doc/compiler-refactor-output-cases.tsv`),
the survivor-property checkers, the PA29 and PA31 Makefiles, the PA13
handout and the backend review records name the fixtures by their new
paths.  `make ref-test` and `make ref-test-debuginfo` regenerated the
references that embed a path.

After the renames and three further moves the auditor found nothing
(`--fail-on-early` exits 0), and the byte-exact `make test-report` passes
with the regenerated references.

## The selfhost lane

`make -C pa39 test-through-pa10 CXX=../dev/cppgm++` (a CI gate) failed at
PA9: every test of the self-compiled `cy86` assembler died with "invalid
ELF header size".  Two compiler defects, both older than the move, both
fixed in the source tree and synced here:

- The optimizer forwarded a load through a phi of addresses to the value a
  retyping store had stored (`store i64 %n` with `%n` a pointer
  difference, as `vector::size()` inlined into `_M_check_len` leaves it),
  so its own -O1 output carried an `i64` phi with a `ptr` incoming and the
  LowIR reader rejected it.  `pa37/tests/regression/o1/549-retyping-store-through-address-phi`
  and `pa38/tests/behavior/o1/500-retyping-store-through-address-phi` hold
  the shape.  `make -C pa38 ref-test` now regenerates the behaviour buckets
  it had skipped, and the behaviour lane's `.ref.mir`/`.ref.cmir` dumps are
  ignored like its `.ref.program`.
- The native backend folded a constant index into a local's frame operand
  even when the result reached one past the local's last byte, abstract
  offset zero, which the encoder reads as the caller's frame and does not
  move past the saved registers: the `last` pointer of a local array passed
  to a range insert arrived 8 bytes too high whenever the function preserved
  a register.  `pa29/tests/behavior/200-one-past-local-array-call-argument`
  holds the shape (it fails before the fix at `-O0`).

To reach the defects, `cppgm++ --emit-*` now accepts `-I`, `-D`, `-U` and
`--hosted`, so one translation unit of the compiler itself can be emitted
as LowIR and optimized or lowered on its own; the reader's phi mismatch
error names the value, both types and the function.

One frontend defect the lane exposed is recorded rather than fixed: our
compiler cannot resolve `std::pair::swap` for a pair whose first member is
itself a pair (`std::sort` over
`std::vector<std::pair<std::pair<int, std::size_t>, std::size_t>>` fails
with "no viable overload for swap"; the inner pair's specialization
receives two class-template identities).  The ordering pass was rewritten
around it; the eight-line reducer is
`docs/v4/reducers/nested-pair-sort.cpp`, to become a PA35 compile fixture
once the frontend resolves it.

## Phase 7: the export

`scripts/export_student_repo.sh` copies the student support files at their
paths in this tree (`dev/src/abi/itanium/abi_mangle*.h`,
`dev/src/preprocess/tokens/*PPTokenStream.h`, `dev/src/support/not_implemented.h`,
`dev/src/support/tool_help_text.h`, `dev/src/support/testing/test_runner.cpp`,
`dev/src/ir_symbol_model.h`), writes the student `frontend_source_sets.mk`
with the test runner's source id, ships the two seams scripts PA16's lane
runs, and no longer validates a witness lane.  The scaffolds include those
paths.  A local run exported 18,651 verified reference files (7,059 exit
statuses, 411 retained failed-case diagnostics), packaged the reference
bundle, and the exported `dev/` builds its scaffolds.  CI gained an
`audits` job (the architecture audits, the file audit, `make test-harness`).

## Phase 7: the host cells

The first CI runs of `v4` were green only on the cell the local machine
matches (Ubuntu 26.04, gcc 15, libstdc++ 15).  The other three cells build
the compiler with a different host compiler and compile the hosted lanes
(PA35, PA36) against a different standard library, and each exposed defects
the local run could not.  Every fix landed in `~/work/v3codex` first and was
synced here; every language defect has a fixture in its owning assignment.

| cell | symptom | cause | fix | fixture |
| --- | --- | --- | --- | --- |
| all but 26.04 gcc | garbage field offsets, `invalid PA11 type identity`, a selfhost segfault | three references held across table growth (`PublishUsingAccess`, `TryAnalyzeFloatingIntrinsicCall`, `PublishFunctionTemplateSpecialMemberRole`) | copy the record, or evaluate before taking the reference | existing PA16 and PA30 fixtures; the selfhost lane |
| 24.04 gcc (libstdc++ 13) | `expression kind 31 does not designate scalar storage` in `_Hashtable::_M_insert_unique_node` | `const T& r = call();` never materialized storage for a scalar prvalue | the reference-initialization path materializes any non-class prvalue | `pa17/tests/general/200-scalar-prvalue-reference-binding.t` |
| 24.04 gcc | `ambiguous overload` in `_Rb_tree::_M_erase` | `f(B*)` and `f(const B*)` ranked equal for a `D*` argument | derived-to-base conversions of equal depth prefer the less qualified target | `pa12/tests/general/200-derived-to-base-pointer-prefers-less-qualified-overload.t` |
| 24.04 gcc | `no viable overload for _S_right` | a pointer to a class template specialization the program had only named was never completed for the derived-to-base check | call conversions complete the pointee on demand | covered by the PA35 map and set fixtures on that cell |
| 24.04 clang (libc++ 18) | `expected binary operand` at `pair.h:125` (86 fixtures) | `name<>()` read as less-than when another class had declared a non-template member of that name | an empty angle pair after a name is a template-id | `pa22/tests/general/300-member-template-empty-argument-list-after-same-named-member.t` |
| 24.04 clang | `duplicate default template argument` on `__invoke` | two function templates differing only in their decltype result matched as one declaration | the whole decltype result takes part in redeclaration identity; roots still decide leading-versus-trailing spellings | `pa23/tests/general/300-decltype-result-distinguishes-function-template-overloads.t`, `300-friend-function-template-alias-result-definition.t` |
| 24.04 clang | `expected OP_RPAREN` at `__math/traits.h:50` | `(typename T::type)x` not parsed as a cast | `typename` starts a cast type-id | `pa22/tests/general/300-dependent-typename-cast-expression.t` |
| 24.04 clang | `invalid universal character name` in `<sstream>` | `\u{` inside a comment | a backslash without a hex quad stays a backslash | `pa1/tests/200-malformed-universal-name-in-comment.t` |
| 24.04 clang | `expected parameter declaration` at `vector:2601` | `vector(*this, …).swap(*this);` read as a declaration in a template member | `T(*this, …)` and `T(this, …)` are expressions | `pa17/tests/general/100-injected-class-name-functional-cast-this-statement.t` |
| 24.04 clang | `structured template type was not found`, `invalid signedness transform operand`, `unknown expression name` while registering `__allocate_at_least` | a shape-only completion (parameters standing in for arguments) treated members the stand-ins cannot reach as errors | such members stay dependent shapes or keep no constant until a concrete specialization is completed | the libc++ cell (no reduction reproduces it outside the header chain) |
| 24.04 clang | `unknown expression name: __libcpp_compute_min<type,digits,is_signed>::value` | a non-type argument spelled as a bare name rejected because a namespace-scope class template shared the name of the class's own static constant | a value in a nearer scope hides the type | `pa22/tests/general/100-nontype-argument-member-hides-namespace-template.t` |
| 24.04 clang | `direct base must name a complete non-union class` in `constexpr_c_functions.h` | libc++'s `__libcpp_datasizeof` takes its `template <> struct _FirstPaddingByte<true>` branch when `__has_cpp_attribute(no_unique_address)` reads as absent | the probe answers true for the attribute the compiler implements | `pa5/tests/500-attribute-probe.t` |
| 24.04 clang (PA36) | `multiple definition of __do_deallocate_handle_size` | a function template specialization whose only arguments sit in an empty pack was emitted as a plain strong function with a non-template mangling and without its abi tag | the specialization keeps its template identity (`IJEE`, weak) and the pattern's abi tags | `pa32/tests/general/100-empty-pack-function-template-duplicate.t` |
| every cell (latent) | member function template symbols one substitution short of the host's (`S2_` where clang writes `S3_`) | the `<template-prefix>` of a member template never took a substitution number | the encoder numbers it before the template arguments | `pa32/tests/general/100-member-template-prefix-substitution.t`; every LowIR reference naming such a symbol changed |
| 24.04 clang | `ambiguous overload` in `__tuple_leaf`'s reference-binding assertion (`tuple:346`, six fixtures) | retained call facts are replayed for every specialization of a class template, and an unqualified call recorded no naming class to check the facts against, so a sibling specialization's member stayed a candidate | the enclosing class stands in for the naming class | `pa24/tests/general/200-member-template-call-per-specialization.t` |
| 24.04 clang | `base polymorphism facts are incomplete` in `std::function` | a lambda closure made an empty base of libc++'s compressed pair never had its facts computed | a complete base's facts are computed on demand; the diagnostic names both classes | `700-hosted-function-nullary-base-reentry-compile` on that cell |
| 24.04 gcc | `no viable overload for _S_right` in `_Rb_tree::_M_erase` | a derived-to-base pointer conversion on a class template specialization the program had only named | call conversions complete the pointee on demand | the PA35 map fixtures on that cell |

With these fixes the Ubuntu 24.04 clang cell's test-report is green, and
all four build cells, both audit jobs, every test-debuginfo job and three of
four test-report jobs pass.  Four CI checks remain red, root-caused to two
classes:

**Retained dependent-qualifier resolution (test-report, 24.04 gcc).**
`700-libstdcxx-regex-compiler-member-alias-call` fails because
`__copy_move_backward<_IsMove, true, rait>::__copy_move_b` calls
`std::__copy_move<_IsMove, false, rait>::__assign_one`, a qualifier
dependent on the enclosing template's `_IsMove`.  `retained_call_template_sets_`
caches the member-template pattern (and `retained_call_naming_classes_` the
naming class) from whichever specialization is instantiated first
(`__copy_move<false,false>`, entity 604).  A later instantiation
(`__copy_move<true,false>`, 605) replays the same callee node; the
naming-class guard in `RetainedFunctionCallCandidates` compares the pattern
owner (604) against the *recorded* naming class (604) and so accepts it,
and the qualified-call rebuild in `CompleteFunctionCallTemplateCandidates`
does not fire because it looks up `active_name[0]` ("std") in the active
class's member scope and finds nothing.  A fix must re-resolve the
dependent qualifier in the active specialization's scope rather than reuse
the recorded set; it touches the same retained-replay logic that a broad
first attempt regressed by ~350 tests, so it needs its own validated pass.

**RESOLVED** 2026-09-06 ("Re-resolve a replayed dependent call's naming class
per specialization").  The narrow fix: when
`CompleteFunctionCallTemplateCandidates` re-resolves a dependent qualifier's
patterns from the active scope (the branch that already handles a retained
specialization without patterns), adopt the freshly-resolved patterns' owner
as the naming class, but only when they share one owner.  The access check then
tests the member against the class the call names in this specialization, not
the sibling's, so the public static `__assign_one` is accessible.  Verified:
the libstdc++ 13 regex fixture compiles and the full byte-exact report is
5951/5951 in the 24.04 gcc cell, no regressions.  Clears the 24.04 gcc
test-report check.

**Host-config self-host codegen (test-through-pa10, 24.04 gcc + clang, 26.04
clang).**  A `cppgm++` built by gcc 13 or clang miscompiles the recognizer
so its grammar-terminal map drops `KW_TRUE`, and PA6's empty test fails at
grammar load.  Reproduced with a fresh gcc-13-configured self-build; the
obvious suspects each compile correctly in isolation (`SimpleTokenKindName`'s
122-entry static `const char*` table, the enum-derived `kSimpleTokenCount`,
and the `unordered_map` fill), so it is a subtler uninitialized-read or
codegen bug in `cppgm++` itself, in the same class as the three
table-growth use-after-frees already fixed, not yet isolated to a function.
The 24.04 clang lane also hits `unknown type name: __alloc` in
`basic_string::__assign_with_sentinel`, a declaration-vs-expression
ambiguity where a value-name parameter (`u` in `holder temp(tag(), u,
__alloc())`) is wrongly accepted as a parameter type; the narrow guard for
it must not disturb genuine parameter declarations (a first attempt did).

Conversion function templates also still mangle their conversion type from
the deduced argument (`cvi` where the host writes `cvT_`).

### Update: two distinct gcc self-host issues

A first investigation, conducted with the local host `g++` (which is g++-15,
not g++-13 -- passing `CPPGM_HOST_CXX=g++-13` sets only the *embedded* config,
not the build compiler), found that g++-15 at `-O3` miscompiles the parser
through a strict-aliasing assumption when it later parses int128-configured
headers.  `-fno-strict-aliasing` on the tool build fixes that and is retained
(`dev/Makefile`, commit 63c4f466); it is byte-exact-neutral.  It does **not**
fix the CI 24.04 gcc cell, which builds with the real g++-13.

Building with the real g++-13.4 (in the Ubuntu 24.04 container, and locally
with `CXX=g++-13`) reproduces the actual 24.04 failure: the compiler builds
cleanly but the self-compiled `recog` cannot map the grammar terminal
`KW_TRUE`, so pa6 reports every test failing with `BAD` output.

Refined 2026-09-06.  It is **config-sensitive, not host-sensitive**: a cppgm++
built by g++-15 (`CXX=g++`) but targeting the libstdc++ 13 config
(`CPPGM_HOST_CXX=g++-13`) miscompiles `recog` too, so it is a cppgm++ codegen
defect triggered by libstdc++ 13 headers, not g++-13 miscompiling cppgm++.
Object-swap bisection pins the miscompiled object to `recognition/recognizer.o`
(swapping a g++-13-built `recognizer.o` into the self-linked `recog` makes pa6
pass).  Instrumenting `BuildSimpleTokenNames` (`recognizer.cpp:694`, which fills
the `std::unordered_map<std::string,uint32_t> simple_tokens_` by
`simple_tokens_[SimpleTokenKindName(i)] = i` over the 122 kinds) shows the exact
mechanism: after the loop the map holds 122 entries and the `"KW_TRUE"` node IS
present -- a manual iteration finds key `"KW_TRUE"`, length 7, value 60 -- but
`count("KW_TRUE")` and even `count(nm)` with the same pointer return 0.  The
entry is in the **wrong bucket after a rehash** (bucket_count 127 at that
point): the node's stored/cached hash disagrees with the hash `count`
recomputes, so the lookup probes a different bucket and misses.  Every reduction
-- the standalone 122-name fill, the member-map-in-constructor with the exact
`uint16_t` loop and `static_cast` -- compiles correctly, so this is a
context-sensitive backend codegen defect (register allocation / optimization in
the large `recognizer.cpp`), corrupting one `unordered_map` node's cached hash
on rehash; it does not reduce out of the full unit.  Same class as the prior
"not isolated to a construct" finding, now localized to the rehash/cached-hash
path.  The real remaining 24.04 gcc self-host blocker; needs a backend session.

Isolated further 2026-09-06 to a single optimizer pass.  Compiling
`recognizer.cpp` with the reproducing compiler at `-O0` yields a correct
`recog`; `-O1`, `-O2`, `-O3` all break it, so it is an optimizer pass, not
baseline codegen.  Bisecting the O1 per-function pipeline
(`pipeline.cpp:optimize_function_bodies`, disabling passes by their
`level >= 1` gate) pins it to **`promote_slots`** alone -- disabling only that
pass makes `recog` correct with every other O1 pass on.  `promote_slots` is the
slot-to-SSA (mem2reg) promotion (`promote_slots_with_analysis`, a
dominance-frontier phi placement plus a sparse abstract-state rename); it
miscompiles a slot in the libstdc++ 13 `unordered_map` insert/hash path within
`recognizer.o` so the "KW_TRUE" node caches a wrong hash and rehashes to the
wrong bucket.  It does not reduce to a small unit, but it HAS a LowIR-level
reproducer: `--emit-lowir` works for this config, and
`docs/v4/reducers/recog-kwtrue-promote-slots.lowir` (the `-O0` LowIR of
`recognizer.cpp`) miscompiles when fed back through `cppgm++ -O1 -c` -- so the
next session can reduce at the LowIR level and read exactly which promotion
`promote_slots` gets wrong, rather than fight the C++ front end.  Local repro of
the whole failure: build cppgm++ with `CXX=g++ CPPGM_HOST_CXX=g++-13`, then
`make -C pa39 test-pa6 CXX=../dev/cppgm++ CPPGM_HOST_CXX=g++-13`.

Two items remain on the self-host lanes after that fix:

- **`__alloc` declaration-versus-expression ambiguity (clang cells) — fixed.**
  libc++'s `basic_string::__assign_with_sentinel` writes
  `const basic_string __temp(__init_with_sentinel_tag(), std::move(__first),
  std::move(__last), __alloc());`.  The multi-argument direct-initializer
  disambiguation already reads `std::move(a)` as a call, but treated the
  last argument `__alloc()` -- a `name()` with an empty parameter clause --
  as a value-initialized temporary of type `__alloc`, reporting
  `unknown type name: __alloc`.  A `name()` whose name is callable and not a
  type is a nullary call, so the statement is an initialization; recognized
  now in `AnalyzeAmbiguousMultiDirectInitializer` (both passes) via
  `NamesCallableNonType`.  Fixture: `pa17/tests/general/200-local-class-
  direct-init-nullary-member-call.t`.  This unblocked the clang and libc++
  self-host lanes at `post_tokenizer.cpp`.

- **libc++ `basic_streambuf::seekpos` vtable reference (clang cells, PA10).
  RESOLVED** (commit "Name undefined vtable relocations by their object
  symbol").  With `__alloc` fixed the clang/libc++ self-host reaches PA10
  (compiling `cppgm++` with itself), which failed at link: `driver.o` and
  `lowering/core/driver.o` held a `.data` vtable slot with an undefined
  reference to `std::__1::basic_streambuf<char>::seekpos`, a virtual the
  driver's stream type inherits from libc++ but that no object defines.
  Reduced to eight lines (`docs/v4/reducers/streambuf-vtable-seekpos.cpp`).
  The earlier "`MangleFunction` returns empty" diagnosis was wrong:
  `MangleFunction` returns the correct Itanium mangling
  `_ZNSt3__1...7seekpos...`, and the vtable slot's symbol carries it as its
  object name.  The real cause was in the LowIR adapter
  (`dev/src/lowir/io/frontend_adapter.cpp`): it built the program's
  `symbol_names` from the internal presentation names only, dropping the
  mangled object name for a symbol that has no local definition and no
  declaration (an inherited virtual reached only through a vtable slot is
  never called, never defined, so nothing puts it in `function_declarations`).
  Native object emission then had no object symbol for the fixup and spelled
  the undefined relocation with the internal name.  Fix: carry each external
  reference's mangled object name in a parallel `symbol_object_names` channel
  (populated only for symbols with no local definition, retained in the
  object-only pruning pass) and, in `elf_format.cpp`, name an undefined fixup
  by that object symbol when it is neither locally defined nor declared.  The
  channel is consumed only by native object emission, so LowIR text is
  unchanged (byte-exact report identical); under libstdc++ `seekpos` is
  inline and stays a weak definition, so the branch never fires there.
  Validated: the reduction compiles, links against libc++, and runs; the
  24.04 clang self-host now links the PA10 stage and passes pa1-pa9.

- **AST-writer self-host miscompile (clang cells, PA10, exposed by the
  seekpos fix).  RESOLVED** (commit "Reach a reference or pointer parameter's
  virtual bases through the vtable").  Root cause: cppgm++ carried a hidden
  companion pointer ("__pvbptr") for a virtual base of every parameter,
  including references and pointers.  How many it carried was derived from the
  parameter's virtual-base count, which requires the complete class type; a
  caller seeing only a forward declaration (a unit passing `std::ostream&`
  through) computed zero while the definition, compiled with the complete
  type, expected the carry-all set, and the two disagreed across the
  translation-unit boundary, so the callee read a companion pointer the caller
  never passed.  Fix: restrict the companion pointer to by-value class
  parameters (whose complete type both sides must see) and reach a reference or
  pointer parameter's virtual bases through the object's vtable at each use
  (`VirtualBoundaryEntity` returns no entity for reference/pointer types, so the
  access site falls through to the existing `RuntimeVirtualBaseAddress` /
  vtable path).  The clang self-host now passes PA1-PA10 (158/158), `make
  inception` still matches byte for byte, and the byte-exact report is green
  (5951/5951) after regenerating the 18 PA28 references that carried the old
  companion arguments; the PA28 handout documents the two shapes.  Original
  investigation notes retained below.  The crash is a SIGSEGV in libc++'s `__pad_and_output` reached from
  `SyntaxArena::Write`'s `output << strings_.Get(node.tag)`; `--emit-types`,
  `--emit-semantics` and `--emit-lowir` all work, so it is specific to the
  AST text dump.  It is not the seekpos fix: the self-compiled `arena.o` is
  byte-identical whether compiled by the fixed or unfixed compiler.  It is
  not `arena.o` or the front end: swapping clang-compiled `arena.o`,
  `frontend_intern.o`, or the whole preprocess+syntax+support group into the
  self link does not fix it.  It is a weak libc++ template instantiation:
  `__pad_and_output` is emitted weak by many objects, ld takes the first
  (`cppgm++-runner.o`), and that instantiation is miscompiled; forcing a
  clang-built `operator<<(ostream, string)` object to win the weak
  resolution makes the crash vanish and the AST dump correct.
  Runtime tracing of the crashing call shows the miscompiled function is one
  frame up: `__put_character_sequence` forwards the string bytes correctly
  (`__ob`/`__ep` point at "translation-unit") but passes a garbage ostream
  (`__iob` is a code address, `__s` is garbage), so `__pad_and_output`
  dereferences a bad streambuf.  The backtrace is itself corrupt (a
  constructor "calls" `WriteTranslationUnit`), i.e. the call chain's stack is
  smashed.

  Deeper root cause: cppgm++ accesses a virtual base's members through a
  companion `__pvbptr` pointer parameter it threads alongside every
  `basic_ostream&`/`basic_ostream*`, rather than reading the virtual-base
  offset from the object's vtable the way clang does
  (`mov rax,[obj]; mov rax,[rax-0x18]; obj+rax+field`).  A five-line
  reproducer confirms the ABI: `long get_width(basic_ostream& os){return
  os.width_;}` where `basic_ostream : virtual basic_ios : ios_base` lowers to
  `@get_width(%os, %__pvbptr0)` and reads `%__pvbptr0 + 8`; clang emits the
  vtable vbase-offset load.  Machinery: `dev/src/lowering/objects/virtual_bases.h`
  (the `__vbptr`/`__pvbptr` parameter synthesis, `CarriesVirtualBase`,
  `VirtualBoundaryEntity`, and the contract built in `CacheVirtualBaseBoundary`).

  Two compounding defects were isolated, and this crash needs both fixed:

  1. **Contract inconsistent across translation units.**  `CacheVirtualBaseBoundary`
     narrows a function's carried virtual bases below the carry-all default by
     scanning the body (which bases are forwarded/demanded).  That scan is not
     TU-stable for a weak/inline function: `arena.o` calls
     `__put_character_sequence` with three arguments (demand-reduced, no
     `__pvbptr` in `rcx`) while the definition the linker keeps, from
     `cppgm++-runner.o`, reads `rcx` as `__pvbptr`.  Restricting the reduction
     to TU-local (static / unnamed-namespace) functions and keeping carry-all
     for weak/external ones makes `arena.o` pass `rcx` and matches the
     definition -- verified at the object level -- but it changes the LowIR
     parameter lists of cross-TU virtual-base functions (reference churn) and
     did NOT by itself stop the crash, so it was reverted pending (2).

  2. **`__pvbptr` corrupts as a stack argument.**  With (1) applied the pointer
     is threaded correctly down to `WriteTranslationUnit` (gdb: its `__pvbptr`
     equals the vtable-computed ios subobject exactly).  `RunTranslationUnit`
     and `SyntaxArena::Write` have more than six parameters, so their appended
     `__pvbptr` lands in a stack argument slot; by `__put_character_sequence`
     it has become a code address (a return address, `Stats::Stats`), and the
     backtrace is stack-smashed.  Traced to the origin: `WriteTranslationUnit`
     receives the correct `__pvbptr` in `r9`, then calls `RunTranslationUnit`
     (declared in `driver_detail.h`, defined in another TU) and stores nothing
     into that call's `__pvbptr` stack slot -- it clobbers `r9` with `stats`
     and passes no companion pointer at all.

  The two defects share one cause: the contract is computed from two sources
  that disagree.  A definition's contract is built by scanning the body
  (`CacheVirtualBaseBoundary` -> forwarded/demanded -> a possibly reduced
  carry set); a caller that sees only the declaration counts hidden pointers
  from the parameter *types* instead (`CountVirtualBaseParameters` /
  `VirtualBaseParameterCount`, effectively carry-all).  Body-derived and
  type-derived counts need not match, and neither is visible to the other
  side of a translation-unit boundary, so caller and definition pass and read
  different numbers of `__pvbptr` arguments.  Restricting the reduction to
  TU-local functions (defect 1) was necessary but not sufficient, because the
  caller-side count for a declaration-only external callee is computed by a
  different routine that still diverged; it was reverted.

  This is the `__pvbptr` companion-pointer scheme, a cppgm++ invention in
  place of the Itanium vtable vbase-offset.  Same class as the gcc-cell
  KW_TRUE self-host miscompile.  The robust fix makes the contract a pure
  function of the signature (so declaration and definition agree without
  seeing each other's body), or -- better -- reads virtual-base offsets from
  the vtable when the most-derived type is not statically known and retires
  the companion pointer.  Either is a large, high-risk change to a core ABI
  subsystem needing a dedicated pass and full byte-exact + audit validation.

- **A flaky build/test race in the self-host chain.**  `make -C pa39
  test-pa6 CXX=../dev/cppgm++` under `-j` intermittently reports the pa6
  suite failing (0/N) while the freshly built `recog-self` is byte-correct
  and `test-pa6-nobuild` passes 48/48; `-j1` always passes.  A missing
  ordering between the checkpoint link and the sub-make test lets the test
  occasionally run before the staged binary is in place.

- **The gcc-24.04 KW_TRUE self-host miscompile: RESOLVED** (v3codex "Plan
  loop-carried phis by cycle membership").  Not the LowIR optimizer:
  `promote_slots` produced correct LowIR for libstdc++-13's
  `_Hashtable::_M_insert_unique_node`; the native backend miscompiled that
  LowIR.  The bucket-index merge `phi` in `if_end_3` has a critical incoming
  edge from `entry`, and the native driver (`driver/session.cpp`,
  `split_critical_phi_edges`) splits it into a block appended at the *end* of
  the function -- invisible to `--emit-lowir`.  `scan_function_layout`
  (allocation/location_planning.cpp) called any predecessor terminator laid
  out at or after the phi a layout backedge, so the merge was planned as a
  loop-carried "local phi": its register (rdi) was promoted on the
  fall-through edge only, `if_end_3` read rdi, but nothing pinned the
  register (the reactive walk pins loop-carried registers through
  `cyclic_register_assumed_` / `CurrentBlockIsCyclic`, which are CFG facts,
  and a split block makes no cycle), so `spill_candidate`'s free-eviction
  clause (`has_spill_home && !cyclic`) dropped it in `if_then_4`; the split
  block's transfer then wrote the frame home while `if_end_3` still read rdi
  -> `buckets[garbage]` on every no-rehash insert, i.e. the node chained into
  the wrong bucket: iteration finds `KW_TRUE`, `count()` does not.  Fix: a
  phi is loop-carried only when the layout-later predecessor shares the phi
  block's cycle (`ControlFlowQueries::BlocksShareCyclicComponent`); a
  split-fed merge keeps the frame home.  Fixture
  `pa38/tests/behavior/o1/100-split-edge-merge-phi-home.t` (the insert shape
  in LowIR; the pre-fix `lowir2native` segfaults it).  Byte-exact report
  5952/5952 unchanged, u24-gcc cell self-hosts PA1-PA10.  Why only the
  libstdc++-13 config: libstdc++-15's `_M_insert_unique_node` has a different
  shape and never planned the merge.  Follow-up worth considering: place split
  blocks before their target so a forward critical edge never becomes a layout
  backedge (also saves the `jmp`); not done here because it moves machine-IR
  references broadly.

- **The gcc-24.04 `__assign_one` ambiguity: RESOLVED** (v3codex "Drop
  sibling replays when a dependent call adopts its owner").  Surfaced by the
  naming-class fix (0d379e7c) in the u24-gcc cell only:
  `ERROR: ambiguous overload for __assign_one among 2 candidates at
  /usr/include/c++/13/bits/stl_algobase.h:439:6` compiling
  `abi/itanium/abi_mangle.cpp` and `semantic/templates/function_instantiation.cpp`.
  A temporary candidate dump showed both viable candidates were
  `__assign_one<unsigned long, unsigned long>`: one from the active owner
  `__copy_move<true,false,RA>`, one retained from the sibling replay
  `__copy_move<false,false,RA>`.  `ResolveRetainedDependentCallPatterns` (the
  dependent-qualifier branch, now its own helper so
  `CompleteFunctionCallTemplateCandidates` stays under the 240-line file-audit
  limit) adopted the fresh patterns' owner as naming class but kept the
  sibling's retained specialization beside the new deduction.  It now drops
  retained candidates whose owner the adopted class cannot reach
  (`QueryBasePath`), mirroring the guard in `RetainedFunctionCallCandidates`.
  Reproduction needs the CI cell: the same binary compiles the file on the
  host, because `/usr/include/x86_64-linux-gnu/c++/13/bits/c++config.h`
  differs (13.3: `__GLIBCXX__ 20240904`, `_GLIBCXX_TIME_BITS64_ABI_TAG`; 13.4:
  `20260327`, `_GLIBCXX_HAVE_O_NONBLOCK`) and changes which `__assign_one`
  specializations the earlier replays retain; `/usr/include/c++/13` itself is
  byte-identical.  OPEN: no standalone fixture yet -- three synthetic
  reproducers (`copier<Move,false,Tag>::assign_one` with and without a
  namespace qualifier, and a libstdc++-shaped `copy_a2/copy_a1` chain)
  compile cleanly with the pre-fix compiler because the `lib::` qualifier path
  rebuilds the call from scope; the regression check is the u24-gcc cell's
  `test-through-pa10`.

- **CI: inception compares every selfhost flavor.**  `inception.yml` used to
  compare one flavor per run (the `workflow_dispatch` input, defaulting to
  `ubuntu-24.04-gcc` for the `workflow_run` trigger).  A `plan` job now emits
  the flavor list -- the named flavor for a manual run, all four after a
  completed Tests run -- and the compare job is a matrix over it.  A
  `workflow_run` trigger executes the workflow file from the default branch,
  so the four-flavor fan-out becomes active when `v4` merges to `main`; the
  run that fired for the green Tests run (34041491490) still used the old
  single-flavor file.  The rewritten file was exercised on `v4` by a manual
  dispatch (`gh workflow run inception.yml --ref v4 -f selfhost_run_id=...
  -f flavor=ubuntu-26.04-clang-libcxx`, run 34041930622): the `plan` job and
  the matrix wiring work.

- **Nested-pair `std::sort`: diagnosed, not fixed.**  Narrowed from the
  original reducer to `std::pair<std::pair<int, size_t>, size_t>`: a plain
  `std::swap` on one fails, `a.swap(b)` on one fails, and `std::sort` over a
  *single*-level pair vector passes.  The failing call is `swap(first,
  __p.first)` inside `std::pair::swap` (`stl_pair.h`), where `first` is
  itself a pair: the generic `std::swap` is SFINAE-excluded for a tuple-like
  argument, so the pair overload declared later in the header has to come
  from argument-dependent lookup at instantiation.  Self-contained 30-line
  reducers: `docs/v4/reducers/adl-late-overload-in-template.cpp` (g++
  accepts, we reject) and `adl-late-overload-no-member-clash.cpp` (the same
  shape with the member renamed).  Two contributing defects, both with
  evidence:
  1. The retained candidate set keeps the enclosing class's member of the
     same name, and `CompleteArgumentDependentCallCandidates`
     (`semantic/expressions/overload_resolution.cpp`) suppresses ADL as soon
     as any candidate has a `member_owner`.  A block-scope using-declaration
     hides that member ([basic.lookup.argdep]/1 suppresses ADL only when
     *unqualified lookup* finds a member), so neither the member nor the
     suppression belongs there.  The error names the member as the only
     candidate.
  2. Renaming the member does not fix it: the call then reports "retained
     call has no viable function", so ADL at instantiation is also not
     picking up a namespace-scope overload declared after the template's
     definition.  Eligibility is not the gate -- the unqualified publication
     sites (validation.cpp 1344, 1361, 1369) already record
     `adl_eligible = true`; the gap is in candidate collection
     (`AppendArgumentDependentCandidates`).

- **Conversion-function-template mangling (`cvi` vs `cvT_`): diagnosed, not
  fixed.**  `struct Holder { template <class T> operator T() const; };` with
  `int v = h;` mangles as `_ZNK6HoldercviIiEEv` here and `_ZNK6HoldercvT_IiEEv`
  under g++.  The Itanium ABI names a conversion function template by the
  *unresolved* target type, a reference to the template parameter, not by the
  deduced one.  The site is `lowering/abi/mangling.cpp`
  (`terminal.function.type = facts.MakeType(binding.conversion_target)`),
  which uses the specialization's substituted target; the encoder
  (`abi_mangle.cpp`, the two `output_ += "cv"` sites) then writes whatever
  type it is handed, and the vocabulary can already express the right thing
  (`ABI_TYPE_TEMPLATE_PARAMETER`, encoded at abi_mangle.cpp:1347).  The fix
  needs the *pattern's* declared target type, which lives in the analyzer's
  `function_templates_` and is not reachable from the lowering layer, so it
  is plumbing rather than a local edit: carry the unresolved conversion
  target on the function fact beside `conversion_target`.  Not attempted
  here: it changes symbol names, so it moves references, the host-compat
  lanes and the self-host link together, and wants its own pass.  Deriving
  the parameter index by matching the target against the template arguments
  would cover `operator T()` but not `operator U<T>()`, and a heuristic in
  the mangler is the wrong kind of risk.

- **The `-j` self-host race: diagnosed, not fixed.**  The checkpoint binary
  rule in `pa39/Makefile` is `FORCE`-based, so it runs on every visit, and
  `test-$(checkpoint)` reaches its suite through a recursive sub-make.  Two
  branches of a parallel build can therefore be inside the same checkpoint's
  link while a sub-make is running that checkpoint's tests.  The link itself
  is already atomic (it writes `.tmp` and `mv -f`s it), which is why the
  staged binary always turns out byte-correct and the `-nobuild` suite
  passes on a rerun; what the racing test sees is the tree mid-transition.
  A lock is the right shape -- the repository root already serializes dev
  builds with a `mkdir` lock -- but each recipe line runs in its own shell,
  so the lock has to wrap the link, the compare and the move as one command;
  a lock taken on its own line releases immediately and buys nothing.  That
  restructuring of the link recipe was drafted and reverted rather than
  landed half-done.  Reproduce with `make -C pa39 test-pa6 CXX=../dev/cppgm++`
  under `-j`; `-j1` always passes.

- **A standalone `__assign_one` fixture: still not found.**  Three synthetic
  reproductions of the shape (sibling specializations of a class template
  whose member template is called through a dependent qualifier, replayed by
  two instantiations) compile cleanly with the pre-fix compiler, because the
  qualifier path rebuilds the call from the active scope.  What makes the
  real case fail is the instantiation order libstdc++ 13.3 produces, which
  differs from 13.4 by `c++config.h` alone; the fixture would have to
  reproduce that order rather than the shape.  The regression check is the
  `u24-gcc` cell's `test-through-pa10`, which does exercise it.

- **Placing critical-edge split blocks before their target: deliberately not
  done.**  The native driver appends a split block at the end of the
  function, which is what made a plain merge look loop-carried (the
  loop-carried phi defect above).  Placing the block before its target would
  also drop the `jmp` the appended block needs.  It is an optimisation, not a
  correctness fix -- the loop-carried defect is fixed at its own cause -- and
  it moves machine-IR references across pa29 and pa38, so it belongs to a
  pass that can regenerate and review them.
