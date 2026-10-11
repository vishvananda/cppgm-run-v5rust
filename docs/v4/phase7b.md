# Phase 7b: pre-combination audits

The four audits the plan gates assignment combination on (plan section
"Phase 7b").  Each section records what was checked, what was found, and
what changed.  Status lines are updated as the work lands.

## 2. Assignment export (first, because the other audits ship through it)

Verification: `scripts/export_student_repo.sh --force <dest>` from `v4`, then
in the export: `make test-report` (the student scaffolds fail every lane by
design -- the check is that the harness runs to its summary), and each
assignment's `make test` with the shipped reference tools
(`make -C paN test CPPGM_TEST_APP=../dev/<tool>-ref`, pa37 with
`LOWIROPT_APP`/`CPPGM_DRIVER_APP`, pa38 with `LOWIR2NATIVE_APP`/
`CPPGM_DRIVER_APP`), with `CPPGM_REFERENCE_BUNDLE_FILE` pointing at the
locally packaged bundle so nothing downloads.

Found (2026-09-06, all fixed in `scripts/export_student_repo.sh`):

- `make test-report` died at once: the root Makefile's `test-report-nobuild`
  depended on `audit-compiler-exceptions`, whose script (a maintainer audit,
  `scripts/audit_compiler_exceptions.pl`) does not ship.  The export now
  strips the maintainer-only targets from the exported root Makefile: the
  nine `audit-*` targets, `build-telemetry-off`/`test-telemetry-off`, the
  `HARNESS_TESTS`/`test-harness` block (`scripts/tests/*.py`), their
  `.PHONY` names, and that prerequisite.  Convention (user, 2026-09-06):
  student-facing scripts are Perl, maintainer scripts are Python; the
  audits are the blurred case (`.pl` but maintainer-only) and are stripped
  by target, not by extension.
- 40 control-lane scripts that `make test` runs in pa15, pa16, pa17, pa29,
  pa32, pa37 and pa38 (`scripts/check_*.pl`; pa37 alone names 24) were not
  in the copied-path list, so a student's `make -C pa37 test` failed on the
  first missing script.  Every `scripts/check_*.pl` now ships (their only
  dependency is the shipped `CppgmBatchWorker.pm`), and pa16's seams lane
  drops its maintainer unit-test line (`scripts/tests/test_compare_lowir_results.py`).
- `scripts/expect_ir.pl`, loaded by `compare_results_common.pl` for every
  fixture judged by a `.ref.expect` sidecar (pa29, pa37, pa38 course
  buckets), did not ship, so those lanes failed even with the reference
  tools.  It ships.
- The export appended its own `reference-binaries`/`setup` targets to a
  Makefile that already defines them ("ignoring old recipe" warning on every
  make).  The append is gone.
- The export now checks itself: after copying, every `scripts/*` name in an
  exported Makefile (comments ignored) and every quoted script name inside a
  shipped script must exist in the export, or the export fails.

The export is also the only gate that sees a changed failed-case diagnostic.
A negative test's stdout is deliberately not tracked -- the export refuses to
run if one is checked in -- and is regenerated into the student repository,
so `make test-report` never compares it; the export regenerates every
reference with the built tools and holds the *tracked* ones (18,722 files,
7,087 of them exit statuses) to what the tree carries.  That is how six
references moved by the diagnostics work were caught while the byte-exact
report was green at 5960/5960.  Until now that gate ran only on `main`,
after a merge, so `export-assignments.yml` gained a `validate` job that runs
the same export on every pull request without the push token or the publish
steps.

Verified after the fixes (2026-09-06): the exported tree's `make
test-report` runs to its summary; with the shipped reference tools, pa16's
control lanes, pa29 (course, behaviour, contract properties, regression),
pa34, pa37 (course buckets and every control lane) and pa38 (course,
behaviour, census, structural and survivor controls) pass.  Convention
deviations to resolve later: `check_lowir_seams.py` and
`lowir_seam_rewrite.py` are Python but student-facing (the PA16 seams lane),
and the nine `scripts/audit_*.pl` are Perl but maintainer-only.

## 4. Tests from `~/work/fable`

Inventory (2026-09-06): fable carries 4,698 `.t` fixtures against 6,000
here.  3,071 fable fixtures have no same-named file here, but 3,045 of them
are byte-identical to a fixture that exists here under another name or root
(the v4 test migration renamed and re-rooted them).  Twenty-six carry
content this tree lacks:

| fable path | verdict |
| --- | --- |
| `pa13/tests/spec/200-*` (9: parameter access/capture metadata, call-boundary metadata, `unary decay`, relaxed arity, `projection=base_subobject`) | obsolete: every form was retired from the LowIR contract (`doc/lowir-contract-ledger.tsv` rows `capture=`, `access=`, `arity=fixed\|prototype_relaxed`, `projection=base_subobject\|reference_field`; `lowir.md`: decay is an ordinary `ptr`); not imported |
| `pa15/tests/general/200-parameter-access-metadata-emission.t` | passes with the fable reference; imported |
| `pa18/tests/general/300-dependent-anonymous-member-field-lookup.t` | passes with the fable reference; imported |
| `pa22/tests/general/500-tcc-member-constructible-pack-sfinae.t` | the fable reference was wrong: the SFINAE condition is false, so `make(...)` (4) is selected; g++ and our compiler agree (exit 249); imported with our reference |
| `pa23/tests/spec/200-tag-parameter-constructor-template-partial-ordering.t` | passes with the fable reference; imported |
| `pa25/tests/general/200-local-static-pointer-new-init.t` | program behaviour matches g++ (exit 0); LowIR differs only in presentation (`operatornew` role metadata, `__local_static__get__decl0__p` naming, zero-init lowering); imported with our reference |
| `pa28/tests/behavior/*` (7) and `pa28/tests/structural/700-call-pass-mode-address-materialization.t` | already here: fable's pa28 native lane is pa29 in v4, and the Phase 4 migration had translated all eight to the current contract (`pass=reference` -> `pass=by_address`, `projection=base_subobject` -> a plain `index`, `unary decay` -> `copy ptr`); the by-name comparison missed the root change.  Re-verified: the seven behaviour programs and the structural fixture pass with this compiler.  A translation experiment on the structural fixture that dropped `pass=by_address` showed the validator accepted a scalar slot for a plain pointer parameter and the program dereferenced the contents; the validator now rejects that (`pa13/tests/spec/400-bad-slot-argument-type.t`) |
| `pa28/tests/strict/*` (2) | the strict lane was purged in Phase 1; not imported |
| `pa34/tests/run/800-hosted-{polymorphic,sse-eightbyte}-value-run.t` | pass with the fable references (with their `.lib.provider.cpp`, `.link.flags` and `.t.1` sidecars); imported |

## 1. Student scaffolding headers

Compared against `main`, which is the pattern to follow: `main` ships
purpose-written student model scaffolds (`dev/src/lowir_model.h` 381 lines,
`dev/src/mir_model.h` 317, `dev/src/x86_register_model.h`, with shared
symbol facts in `ir_symbol_model.h`), simple by design -- names are plain
strings, `struct LowType { std::string text; }` -- and its compiler reuses
them: `lowir_internal.h` is a 51-line header that does `using namespace
lowir_model;` and declares the operations over the scaffold's types.

v4 had lost that boundary.  Phase 2 deleted `lowir_model.h` and
`mir_model.h` (`x86_register_model.h` survives as
`native/mir/registers.h`, byte-identical), the handouts were repointed at
the implementation's own `lowir/model/program.h` and `native/mir/model.h`,
and `ir_symbol_model.h` was cut down to one enum -- losing the
`ExportedSymbol` the handouts say it carries and the two scaffolds used.

Shipping the implementation's headers instead was the wrong repair, and was
tried first: they are 647 and 579 lines and carry interned `SymbolId` /
`StringId`, `PresentationName`, `GeneratedNameReservations`,
`PresentationPolicy` and stats counters -- optimizer and presentation
internals no assignment requires -- and pulling them in dragged
`identity.cpp`, `native/errors.cpp` and `exception_types.cpp` along just to
link.  The READMEs promise "one possible representation" a student "may use
directly, adapt, or replace"; the implementation's model is *the*
representation.

Restored instead, close to `main` with changes only where the contract
moved:

- `dev/src/lowir_model.h` and `dev/src/mir_model.h` are `main`'s scaffolds.
  Against `main` the LowIR scaffold differs by six comment lines, one
  `using namespace ir_model;`, the vocabulary that moved to the shared
  header, and one dropped field (`object_trivial_lifecycle`, retired).  The
  machine-IR scaffold differs by three comment lines and the register
  header's v4 path.
- The metadata vocabulary is now defined once, in `ir_symbol_model.h`: the
  twelve contract enums and `FunctionBoundaryMetadata`.  Both scaffolds and
  `lowir/model/program.h` take it from there, so the student interface is
  the compiler's interface and the two cannot drift.  The contract audit
  reads that header, so the student-facing vocabulary is checked against
  the parser and the serializer (140 public enum values, unchanged).
- The vocabulary matched `main`'s except where v4 moved the contract, and
  the scaffold was updated to v4: `SymbolRole` drops the six roles the
  ledger disposes of (`eh_top`, `eh_value`, `eh_type`, `eh_unhandled`,
  `eh_call_unexpected`, `eh_current_exception_type`) and gains the eleven
  v4 roles (memory, RTTI, `dynamic_cast`, `terminate`, `pure_virtual`);
  `ParamPassingMode` drops `PPM_REFERENCE` and `PPM_DECAY`;
  `ParamCaptureMode` and `ParamAccessMode` are gone; `CallQueryMode` is
  added for `query=stable_prefix`.
- `ir_symbol_model.h` regains `ExportedSymbol` and its helpers from `main`.
- The handouts name the scaffolds again, matching `main` word for word
  (pa29 differs only in the register header's path).  pa38's design note
  still describes the allocator seam, but now says its files are the course
  solution's and not part of the starter kit.

No performance risk: only enum and metadata definitions moved between
headers, and nothing in the compiler's hot path changed
representation -- the compiler keeps its interned model, and the plain-string
scaffold is not used by it.  The byte-exact report and inception are
unchanged.

The export ships the four student headers and no implementation source, and
builds and runs a unit that includes both scaffolds with only the shipped
student headers on the include path (g++ and clang++/libc++), so the
scaffolds cannot rot again.  Deviations from `main` to keep in mind: the
register model lives at `native/mir/registers.h` rather than top level
(byte-identical, shared by scaffold and compiler), and
`check_lowir_seams.py` / `lowir_seam_rewrite.py` are Python but
student-facing while the `audit_*.pl` are Perl but maintainer-only.

## 3. Diagnostics

Inventory (2026-09-06): 901 `ThrowSemanticError` sites, 288 `ThrowInternal*`,
47 `ThrowSource`, 46 preprocessing source errors, 39 `ThrowSyntaxError`, 17
lexical.  Semantic errors already carried `at file:line:column` from a hook
the analyzer installs; a sample of one program per class showed every other
stage naming no position, and two messages leaking internal identifiers.
All of it is closed:

- **Lexical errors** (`invalid escape sequence`, UTF-8 and UCN errors,
  unterminated comments): the tokenizer knows the line and column but not
  the file, so the macro processor, which drives it, adds the file and the
  position the tokenizer last reported.  A diagnostic raised through the
  sink that already names a position is rethrown unchanged.
- **Preprocessing**: all 43 source-error sites name a position -- the
  directive's name token inside directive parsers, the macro head or the
  current token where one is in scope, and the last reported position for
  the end-of-input cases.  `#error` carries its own message text.  One
  static helper that decodes a `#line`/`#include` string operand has no
  position to name.
- **Parser**: `expected` names the token's spelling (``expected `;` ``)
  rather than `OP_SEMICOLON`, through a new `SimpleTokenKindSpelling`.  The
  other syntax sites already carried a position and the offending token
  (`unterminated namespace at s1.cpp:1:20 at token 6`).
- **Type names**: a class template specialization printed as
  `std::__cppgm_class_template_identity_266_178_0`; diagnostics now render
  it as the source wrote it (`std::vector<int, std::allocator<int>>`).  The
  renderer post-processes the existing type text so the presentation
  contract that 137 reference files depend on is untouched.
- **Instantiation context**: an error inside a template body pointed at the
  pattern's source with no hint of which instantiation reached it.  A
  specialization published for the duration of the deferred body analysis
  adds `while instantiating twice(struct S)`.
- **Link stage**: `undefined native symbol: f` now names the definition that
  referenced it (`referenced by main`), found from the materialized label
  that encloses the fixup.
- **LowIR input**: reader errors named no place at all; the reader now
  publishes its source name and line, so every one of the 112 input-error
  sites and the validator's report `at recognizer.lowir:3`.  This also gave
  PA9's CY86 diagnostics positions, which moved four checked-in
  failed-case references.

PA6's two failed-case references and PA9's four moved, because those tools
share the lexical and CY86 error paths that gained positions.

One caution learned here: `program.h` declares the LowIR throw helpers with
`__attribute__((cold, noinline, noreturn))` on the line before each
declaration, so inserting a declaration after an attribute silently makes
the *new* function `noreturn`.  That produced a segfault during unwinding
rather than a compile error.
