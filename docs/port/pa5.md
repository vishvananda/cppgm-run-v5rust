# PA5 port ledger

Stage base commit: `d182aa4bc3c8d2a2bfeffb7c615bea7a77a5fd43`
Last audited commit: INITIAL

## Exception/allocation and connected names checkpoint — 2026-10-12

Entry commit: `064a6a51fa51717d27ffddfe9349dae560dbdf73` (clean).
Stage base and last-audited commit above remain unchanged. The preceding
foundation turn is **progress**, proven by its implementation commit and
0 -> 63 exact matches. The intervening supplied stageProgress result showed
63 -> 63; entry revalidation found no live compiler/check process to resume.
This checkpoint takes the next independent grammar port action and remains a
coherent partial PA5 handoff. Ralph owns the progress/final audit and advancement.

Read the repository guide, clone contract, specification, clone plan and both
PA5 handouts. Inspected the actual frozen C++ owners under
`/home/vishvananda/work/references/v5rust-cpp/dev/src/` before translating:

| C++ owner | Rust owner and completed responsibility |
| --- | --- |
| `syntax/parser_statements.cpp`, `parser.cpp`, `ast_statements.h`, `ast_declarations.h` | `syntax/statements.rs`, `declarations.rs`, `ast.rs`: try/catch, nonempty ordered handler sequences, ellipsis and typed catch declarators, ordinary function-try bodies |
| `syntax/parser_expressions.cpp`, `ast_expressions.h` | `syntax/expressions.rs`, `ast.rs`: throw at assignment precedence, optional rethrow operand, global new/delete, array delete, placement versus parenthesized type, allocation type and initializer syntax |
| `syntax/parser_declarators.cpp` | `syntax/declarators.rs`, `declarations.rs`: distinct New/Conversion modes, pointer/cv and allocation array derivations, initializer parentheses excluded from New type syntax, special-name parameter clauses in definite type context |
| `syntax/parser_names.cpp`, `parser_specifiers.cpp`, `ast_names.h` | `syntax/names.rs`, `declarations.rs`, `ast.rs`: located typed identifier/operator/literal-operator/conversion/destructor/decltype components, full overloadable operator vocabulary and linked right-shift halves, decltype operands/specifiers/qualified types |
| `lex/post_tokenizer.cpp`, `syntax/parse_token_stream.cpp` | `lex/mod.rs`, `syntax/cursor.rs`: retain already interned adjacent keyword suffix identities in literal-operator names; token kind/output stays the existing PA2 contract |
| `syntax/ast_printer.cpp`, `ast_render.cpp` | `syntax/printer.rs`, `render.rs`: iterative exact views, handler child order, placement labels, qualified operator spacing, decltype operand/qualified-rest children; typed data never travels through rendered text |

The unit owns one tree of flat records and typed list spans. `FunctionBody`
references its single compound body and an optional handler list; ordinary try
statements reference the same typed Handler/ExceptionDeclaration model. Catch
parameters and the outermost catch body share a scope, restored before the next
handler. New expressions retain optional placement, parenthesized-type identity,
type-id and initializer separately; delete retains global/array facts. Name
components carry source offsets and typed payload IDs. `names.rs` owns name
grammar/category decisions formerly in the shared parser; this is a responsibility
boundary corresponding to the C++ name owner, not a numbered file split.

Handler list storage participates in complete nested transactions, including
committed inner attempts. Parsing and printing reuse the established demand
cursor and syntax slabs. No whole-unit token retention, cloned syntax tree,
semantic evaluation, lowering, optimizer or backend demand was introduced.
Allocation/exception syntax does not perform overload resolution or cleanup
lowering; those later phases will consume these retained facts. The code
generator, optimizer and register allocator were not changed.

Three new executed Rust invariant tests cover nested handler/list rollback and
catch shadowing, allocation derivations versus initializer/placement syntax, and
typed qualified special-name/decltype rendering. Debug and release pass all 61
library and eight tool tests. There are no new lint suppressions or dependencies.
The companion audit examines 76 files with zero findings.

Artifacts for this checkpoint:
`/home/vishvananda/work/private/v5rust/artifacts/pa5-exceptions-allocation/`.
`entry.json`, `start-results.json`, `progress.json`, `pa1-results.json` through
`pa5-results.json`, `passed-case-ids.txt` and `failure-index.json` retain the
baseline, exact identities, unchanged manifest SHA-256, tool hashes and raw
protected traces. `cases/<case-digest>/trace.jsonl` and `course/test/` archive the
full PA5 receipts and raw mismatch diagnostics/ASTs. `course/pa1/` through
`course/pa4/` archive earlier full course evidence. No fixtures, references,
oracle source, checker, gate policy or Ralph state were edited.

| Command | Final result and receipt |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` / required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | 71/193 exact matches, 122 failures; checker exit 1 / make exit 2; `stage.log`, `pa5-results.json`, archived per-case traces |
| `make course-pa5` | FAIL, 71/193, failure marker retained; `course.log`, `course/test/counts.txt`, `course/test/course-result.json`; no expected-failure exceptions |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa5` | PASS; PA1–PA4 fixed counts 58/58, 26/26, 20/20, 106/106 and full course counts 58/58, 28/28, 20/20, 112/112; `prior.log` and archived results/course receipts |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests and companion audit; `quality.log` |
| `make audit-rust` | PASS, 76 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS, 63/63 prior passing identities; `preserve.log`; independent `progress.json` also proves zero losses and 130 -> 122 failure reduction in the same 193-case manifest |

Exact newly passing fixed case IDs:

```text
tests/general/100-catch-declaration.t
tests/general/100-new-delete-traits.t
tests/general/200-allocation-array-operator-ids.t
tests/general/200-literal-operator-id.t
tests/general/200-overloadable-operator-ids.t
tests/general/200-parenthesized-new-type-vs-placement.t
tests/spec/100-switch-try.t
tests/spec/300-declaration-statement-ambiguity.t
```

Additional owner validation: run `python3` on the artifact directory's
`differential.py`. All 32 scratch programs compare full exit/stdout/stderr/AST
bytes against the frozen C++, including nine malformed-input forms and valid
nested handlers, catch pointers/function/array declarators, throw/rethrow,
placement/parenthesized/array new, global/array delete, all special-name kinds,
alternative operator spellings and adjacent keyword literal suffixes. Inputs and
raw outputs remain in `differential/`, with `differential.json` receipts. These
supplement the fixed manifest and do not contribute to checkpoint progress.
No oracle/harness defect was found.

Resource evidence: run the artifact directory's `measure.py`. Fixed generated
sources at 1000/4000/16000 functions exercise the existing function path, nested
exception handling, and allocation/decltype/operator-name hot paths. Oracle
A/A calibration and A/B/B/A blocks verify full output/stdout/stderr/exit equality
on all 54 invocations. `resource-metadata.json` freezes binary hashes, Linux
x86-64, rustc 1.98.0 and UTC/C locale; `measurements.json`, `.time`, `.stdout`,
`.stderr` and `.ast` retain every observation and source/output digest. These
are whole-invocation warm wall times/RSS, including preprocessing, the clock
helper and dump I/O; they are neither a PA35 target nor hardware-counter proof.

| Workload | Functions | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| functions | 1000 | 23.73 | 8004 | 10.18 | 6536 |
| functions | 4000 | 50.36 | 8104 | 24.94 | 8616 |
| functions | 16000 | 164.69 | 19832 | 87.57 | 21300 |
| exceptions | 1000 | 37.75 | 8032 | 17.63 | 7200 |
| exceptions | 4000 | 104.69 | 10628 | 57.04 | 13552 |
| exceptions | 16000 | 377.34 | 36004 | 210.27 | 38516 |
| allocations | 1000 | 62.77 | 8024 | 22.58 | 7964 |
| allocations | 4000 | 139.49 | 12792 | 74.50 | 16428 |
| allocations | 16000 | 499.80 | 44144 | 291.27 | 50064 |

The 4000 -> 16000 increase takes roughly 3.3–3.6 times the Rust wall time
with storage growth tracking introduced syntax. Existing compaction/long-chain
invariants still pass. No performance acceptance policy was changed.

Next dependency boundary: class specifiers and indexed member scopes in
`parser_specifiers.cpp` / `name_scopes.cpp`, then special-member declarations,
base/mem-initializers, bit-fields and deferred inline bodies. The new function
body/handler representation is ready for constructor initializers; constructor
and member function-try syntax still awaits that class owner. Template
parameter categories and typed template arguments remain a separate predecessor
for many class cases; operator/destructor template-id suffixes explicitly remain
status 86 until that owner is ported. Qualified conversion-result lookup in a
class depends on its member scope, and full dependent lookup/conflict handling,
attributes, lambdas, packs, hosted grammar/driver surfaces and the host byte-stack
budget remain pending. Unsupported grammar continues to report status 86;
three missing template-id interpretations still produce nonmatching real syntax
failures, not successful clone rejections.

Architecture review should inspect complete rollback of handler/name-component
lists, catch and parameter scope restoration, source-faithful special-name IDs,
allocation ambiguity boundaries and iterative rendering without reparsing or
semantic/backend demand. This is a documented coherent group handoff, not full
PA5 completion or permission to advance the stage.

Current exact remaining fixed-manifest failures, clustered from final raw trace
messages at their earliest divergent owner (`failure-index.json` retains paths):

### attribute syntax and retained attribute facts (2)

```text
tests/general/100-balanced-attributes.t
tests/general/200-trailing-parameter-carries-dependency-attribute.t
```

### template declarations and parameter categories (65)

```text
tests/general/100-class-alignas-after-class-key.t
tests/general/100-decltype-qualified-id-expression.t
tests/general/100-dependent-logical-template-argument.t
tests/general/100-function-type-template-argument-leading-named-parameter.t
tests/general/100-member-operator-name-call.t
tests/general/100-parameter-clause-trailing-ellipsis.t
tests/general/100-parameter-shadows-template-comparison.t
tests/general/100-parameter-shadows-template-strict-comparison.t
tests/general/100-template-argument-paren-greater-nested.t
tests/general/100-template-template-parameter-name-reuse.t
tests/general/100-unary-forms.t
tests/general/200-attributed-partial-specialization-current-class-constructor.t
tests/general/200-attributed-virtual-destructor-member.t
tests/general/200-decltype-less-partial-specialization.t
tests/general/200-dependent-member-template-call.t
tests/general/200-dependent-sizeof-pointer-type-id.t
tests/general/200-dependent-template-id-names.t
tests/general/200-dependent-template-keyword-nested-angle.t
tests/general/200-forward-unknown-nested-template-in-ctor-body.t
tests/general/200-friend-function-template-declaration.t
tests/general/200-inline-namespace-template-visibility-base.t
tests/general/200-malformed-template-parameter-clause.t
tests/general/200-member-relational-ignores-using-template.t
tests/general/200-member-template-parameter-value-vs-template-name.t
tests/general/200-namespace-alias-using-directive-imported-template-id-type.t
tests/general/200-nested-qualified-template-id-template-args.t
tests/general/200-nested-template-return-assignment-operator.t
tests/general/200-operator-less-followed-by-operator-greater.t
tests/general/200-pack-base-initializers.t
tests/general/200-parenthesized-qualified-function-template-call.t
tests/general/200-partial-specialization-current-class-constructor.t
tests/general/200-placement-new-pack-init.t
tests/general/200-postfix-template-suffixes.t
tests/general/200-qualified-base-member-call.t
tests/general/200-qualified-base-template-id-logical-argument-value-shadow.t
tests/general/200-qualified-call-direct-initializer.t
tests/general/200-qualified-call-member-body.t
tests/general/200-qualified-member-comparison-template-arg.t
tests/general/200-qualified-template-id-parameters.t
tests/general/200-relational-qualified-template-static-calls.t
tests/general/200-reopened-namespace-template-function-template-name.t
tests/general/200-template-class-body.t
tests/general/200-template-conditional-simple-type-shift-return.t
tests/general/200-template-function-body.t
tests/general/200-template-function-type-alias-declaration.t
tests/general/200-template-function-type-alias-pack-declaration.t
tests/general/200-template-id-function-pointer-argument.t
tests/general/200-template-id-function-pointer-initializer.t
tests/general/200-template-id-return-type-inline.t
tests/general/200-template-id-value-if-condition.t
tests/general/200-template-qualified-conversion-operator-dependent-result.t
tests/general/200-template-qualified-inline-attribute-constructor-definition.t
tests/general/200-template-qualified-inline-constructor-definition.t
tests/general/200-using-declaration-imported-template-id-type.t
tests/general/200-using-directive-imported-template-id-type.t
tests/general/300-nested-angle-and-shift.t
tests/general/300-operator-template-calls.t
tests/general/300-value-comparisons-in-template-arguments.t
tests/spec/100-template-class.t
tests/spec/100-template-parameters.t
tests/spec/200-explicit-instantiation-declaration.t
tests/spec/200-explicit-specialization-syntax.t
tests/spec/200-non-type-template-parameters.t
tests/spec/200-qualified-special-member-definition.t
tests/spec/300-template-id-less-expression.t
```

### class specifiers, member scopes and deferred bodies (49)

```text
tests/general/100-known-function-name-with-type-marker.t
tests/general/100-late-nested-class-type-in-inline-member.t
tests/general/100-member-declarations.t
tests/general/100-special-member-definitions.t
tests/general/100-typedef-anonymous-union.t
tests/general/100-typedef-shadows-outer-value-in-member-function.t
tests/general/100-typedef-struct-union.t
tests/general/200-braced-return.t
tests/general/200-builtin-function-style-cast-member-body.t
tests/general/200-casts-typeid-construction.t
tests/general/200-class-definition-missing-semicolon-bad.t
tests/general/200-constructor-declaration-identifier-parameter.t
tests/general/200-decltype-base-and-mem-initializer.t
tests/general/200-elaborated-enum-member-declarators.t
tests/general/200-empty-brace-scalar-init.t
tests/general/200-extern-c-class-declaration.t
tests/general/200-friend-type-declaration.t
tests/general/200-function-throw-typed-specification.t
tests/general/200-function-try-and-exception-declarators.t
tests/general/200-function-virt-and-noexcept-suffixes.t
tests/general/200-global-qualified-pointer-conversion.t
tests/general/200-global-struct-paren-declaration.t
tests/general/200-lambda-capture-forms.t
tests/general/200-member-pointer-const-function-declarator.t
tests/general/200-member-pointer-data-declarator.t
tests/general/200-member-pointer-function-declarator.t
tests/general/200-member-relational-logical.t
tests/general/200-member-template-if-less-template-call.t
tests/general/200-nested-unary-call-arguments.t
tests/general/200-parenthesized-function-style-cast-mem-init.t
tests/general/200-placement-new-identifier-led-initializer.t
tests/general/200-qualified-base-and-ctor-init.t
tests/general/200-qualified-conversion-operator-definition.t
tests/general/200-qualified-nonconversion-operator-missing-return-type-bad.t
tests/general/200-qualified-result-parenthesized-member-pointer-declarator.t
tests/general/200-sizeof-elaborated-class-type-id.t
tests/general/200-sizeof-functional-cast-expressions.t
tests/general/200-sizeof-qualified-value-call-expression.t
tests/general/200-sizeof-zero-arg-functional-cast.t
tests/general/200-template-member-definition-inherited-typedef-cast.t
tests/general/200-trailing-parameter-vendor-attribute.t
tests/general/200-typeid-postfix-member-suffix.t
tests/general/200-zero-width-bit-field-declaration.t
tests/general/300-call-argument-prevents-function-declaration.t
tests/general/300-declaration-expression-intake.t
tests/general/300-function-object-ambiguity.t
tests/general/300-local-typedef-shadows-value-qualified-type.t
tests/spec/200-bit-field-declaration.t
tests/spec/200-class-bases-and-ctor-init.t
```

### lambda expressions and deferred bodies (2)

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-declarator.t
```

### unresolved template-id calls (syntax divergence) (3)

```text
tests/general/100-template-condition.t
tests/general/200-switch-case-qualified-template-id-call.t
tests/general/200-switch-case-template-id-call.t
```

### template-id name disambiguation (1)

```text
tests/general/200-mock-template-name-angle-forms.t
```

## Foundation checkpoint — 2026-10-12

Implementation commit: `93d81e7b0cea8d13c7d8f0f3ffe74546077290b4`.
PA5 remains a partial assignment. Implementation does not advance the audited
commit or Ralph's stage. Entry revalidation found a clean checkout, no live
compiler/check process to resume, and the `cppgm++` status-86 scaffold in every
one of the 193 fixed cases. The preceding supplied PA5 result represented no
implementation progress; this entry took the next available port action.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `docs/rust-clone-plan.md`,
`pa5/README.md`, `pa5/parsing.md`, and the previous ledger. Consulted the frozen
C++ source at `/home/vishvananda/work/references/v5rust-cpp` directly (clone
contract commit `905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`).

## Contract, owners and completed behavior

The due tool is `cppgm++ --emit-ast -o output source...`. The protected manifest
has 193 cases and SHA-256
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`.
The strict clone gate compares exit, stdout, stderr and full AST output, beyond
the course's success-output/exit comparison. No fixture, oracle, checker,
reference or acceptance policy changed, and no new fixed-manifest cases were
added. Later compiler modes, optimizers and backend owners are untouched.

| C++ responsibility consulted | Rust owner and checkpoint coverage |
| --- | --- |
| `dev/cppgm++.cpp` AST invocation/source loop | `dev/bin/cppgm.rs`, `dev/src/driver/ast.rs`: per-source reset, ordered multi-unit wrappers, located failures and sticky output failure; shared invocation clock captured in `dev/bin/source_tool/build_time.rs` |
| `syntax/syntax_unit.*`, `parse_token_stream.*` | `syntax/mod.rs`, `cursor.rs`: shared preprocessing/post-token demand, compact token descriptors, linked `>>` halves, retained lookahead/compaction and unit-owned literal byte spans |
| `syntax/ast*.h`, `scratch_list.h`, `support/memory/arena.h` | `ast.rs`, `storage.rs`: distinct typed IDs and enums in flat node/list slabs, reusable nested scratch, checked u32 indices and explicit truncation |
| `syntax/parser.cpp`, `parser_state.*`, `parser_regions.h` | `parser.rs`: ordered declaration sequences, Result alternatives, syntax/list/scope/function-context transactions and furthest absolute-token diagnostic position |
| `syntax/parser_declarations.cpp`, `parser_specifiers.cpp` | `declarations.rs`: empty/simple/function declarations, namespace definitions/aliases, using/aliases/typedef, linkage/static-assert, builtin/storage/cv/named/enum specifiers and scalar/paren/braced/default/delete initializers |
| `syntax/parser_declarators.cpp` | `declarators.rs`: pointers/references/cv, nesting/arrays, function clauses, parameter/default syntax, clause ellipsis placement, function-vs-initializer preference, cv/ref/noexcept/throw/trailing return qualifiers and type-ids |
| `syntax/parser_expressions.cpp` | `expressions.rs`: precedence/associativity, literal/keyword/qualified IDs, parens, unary/binary/assignment/comma/conditional operators, call/subscript/member/postfix, functional/C-style/keyword casts, sizeof/typeid/alignof/noexcept and braced lists |
| `syntax/parser_statements.cpp` | `statements.rs`: ordinary declaration preference, blocks, expression/declaration conditions, if/switch/while/do/for/range-for, labels, jumps, return and throw statements |
| `syntax/name_scopes.*`, `parser_names.cpp` | `scopes.rs`, `parser.rs`: indexed typed categories, lexical parents, namespace reopen/alias/import edges, anonymous namespace identity, enum members and retained enum scope through aliases/typedefs; declared categories override C/Y/E/T hints |
| `syntax/ast_printer.*`, `ast_render.*` | `printer.rs`, `render.rs`: independent borrowed views with iterative action stacks, exact child order and compact label spelling; no rendered-text phase transport |
| `lex/token_kind.*`, `identifier_table.*` | Existing lex owners: canonical token/keyword spellings, typed keyword/operator inspection and borrowed interned-name bytes |

This is one coherent source-to-AST foundation group. The driver owns the source
buffer; each syntax unit owns its preprocessor, post-token pull adapter,
lookahead, syntax slabs and categories. Semantic, lowering and backend phases
are not demanded by AST printing. Literal scalar/string/suffix facts remain
typed instead of being recovered from dump text. Names and type derivations
remain structured. Each successful unit is parsed once; only bounded ambiguity
regions are retried. IDs and mutation APIs stay private to their unit.

A failed attempt rewinds every node/list store, list scratch, binding/edge and
anonymous-scope mutation, scope allocation/current scope, token position and
function parameter scope. Token spellings and interned names remain stable
across retries. Nested committed attempts are still reversible by an outer
attempt. Declaration traversal and print order use explicit source lists,
never hash iteration. Scope lookup visits indexed parents/import edges and uses
a reusable visited set, without scanning unrelated declarations. Consumed tokens
compact at 4096 outside retained regions. No per-node Box tree or recursive
syntax destruction is introduced; printers/renderers also handle long operator
chains without recursive traversal.

Five Rust invariant tests cover nested commit/rollback including namespace and
alias bindings, failed parameter-name shadowing, 6000 declarations crossing
window compaction followed by a located error, a 2000-operand iterative dump,
and enum aliases/typedefs across reopened unnamed namespaces. Both debug and
release runs pass. The source map records only the translated responsibilities
as partial; no broad stage-completion claim follows from the case count.

## Validation evidence

Frozen checkpoint artifacts:
`/home/vishvananda/work/private/v5rust/artifacts/pa5-foundation/`.
The directory retains `metadata.json`, final `pa1-results.json` through
`pa5-results.json`, `cases/<protected-case-id>/trace.jsonl`, raw mismatch
outputs/diagnostics, full `course/` receipts, command logs, workload sources and
`measurements.json`. `passed-case-ids.txt` and `failure-index.json` retain exact
case identities and their source/trace locations. The live protected receipts
remain under `/home/vishvananda/work/private/v5rust/artifacts/clone/`.

| Command | Result and receipt |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` (required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5`) | 63/193 exact matches; 130 failures; checker exit 1 / make exit 2; `stage.log`, `pa5-results.json`, per-case raw traces |
| `make course-pa5` | FAIL, 63/193; `course.log`, `course/counts.txt`, `course/course-result.json`; missing syntax is still reported as failed comparisons, with no expected-failure exception |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa5` | PASS; `prior.log`; PA1–PA4 fixed counts 58/58, 26/26, 20/20, 106/106, and full course counts 58/58, 28/28, 20/20, 112/112 |
| `make test-report-through-pa4` | PASS before the final related fixes; final protected prior run above repeats the same differential/course lanes |
| `make lint` / protected `clone_quality.py` | PASS: rustfmt, strict Clippy, debug/release unit tests (58 library plus 8 tool tests in each configuration) and companion audit; `quality.log` |
| `make audit-rust` | PASS: 75 files, zero findings; `audit.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa5` | PASS; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa5` | PASS, 0/0 previously recorded PA5 passes; `preserve.log`; the start state had no passing PA5 identity |

Failure reduction is 193 -> 130 on the unchanged 193-case manifest. Earlier
fixed and full course lanes are all green. The phase result is a coherent
partial checkpoint; the current full PA5 gate and course lane remain failing.
Ralph owns the checkpoint progress/final audits and stage advancement.

Candidate `cppgm++` SHA-256:
`5c278963dfe2fd12b8122af7c8c2cf701566e39f0ff150801c6f1465cbf73712`.
Frozen C++ `cppgm++` SHA-256:
`b7d0c36db57478609a74ade0fbaa0b878827286581c2cf80f69d3e64597fbda3`.
All nine candidate hashes and fixed manifest hash are in `pa5-results.json`.

## Resource evidence and supported boundaries

Run `python3 /home/vishvananda/work/private/v5rust/artifacts/pa5-foundation/measure.py`.
The fixed generated declaration and function workloads contain 1000, 4000 or
16000 distinct declarations/functions. Inputs and outputs have SHA-256 receipts.
Every complete AST matches the frozen C++ bytes on all 36 invocations, with
oracle A/A calibration followed by an A/B/B/A block at each size. The tool
paths/binary hashes are frozen; Linux x86-64, UTC/C locale and rustc 1.98.0 are
recorded in `metadata.json`. Raw elapsed/RSS files accompany every run.

| Workload | Count | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| declarations | 1000 | 24.25 | 8104 | 8.85 | 5872 |
| declarations | 4000 | 45.40 | 8104 | 20.25 | 7228 |
| declarations | 16000 | 115.97 | 15604 | 63.42 | 15000 |
| functions | 1000 | 28.30 | 8036 | 13.36 | 6500 |
| functions | 4000 | 59.20 | 8032 | 33.25 | 8620 |
| functions | 16000 | 169.34 | 19136 | 93.22 | 21236 |

These are whole-invocation warm observations, including source reading,
preprocessing, parsing, printing and the once-per-invocation host clock helper;
they are not a Linux hardware-counter regression gate or PA35 target. Flat
syntax/list storage scales with introduced syntax; the 6000-declaration unit
test bounds peak token window at 4098. Long left chains are parsed iteratively
and printed by action stacks; their nested dump indentation necessarily costs
bytes proportional to the output rather than merely the input.

## Next dependency boundary and audit questions

Next port class specifiers and class-member scope ownership in
`parser_specifiers.cpp` / `name_scopes.cpp`, then special members/member
pointers, bit-fields, base clauses and constructor initializer syntax. Member
bodies require the C++ `parser.cpp` retained token-run/capture/replay boundary:
parse each body once after its enclosing class completes. The largest related
unimplemented predecessor is template declaration/parameter categories and
structured template-id arguments (including memoized closing-angle attempts).
Those groups account for most failures below and must preserve category
replacement, dependent prefixes and parse-once deferred ownership.

Other remaining groups are attributes and their retained facts, new/delete,
lambdas/deferred speculation, try/catch/function-try blocks, operator and
literal-operator names, decltype paths, and pack syntax. Before semantic
consumers are connected, complete parameter/initializer/name-component source
metadata, retained alternative statement readings, qualified-member lookup,
exact namespace nomination/conflict rules and remaining named-function-type
sizeof/typeid disambiguation. Later hosted options/builtins, stats/prelude/pack
observations and CLI surfaces also remain due to their owning source groups.

The safe recursive parser currently returns status 86 at nesting depth 256;
this explicitly marks the unported host byte-stack-budget policy, rather than
claiming an exact C++ language rejection. Ordinary unsupported grammar owners
also return status 86. Three unresolved template-id call cases currently reach
real syntax failures at the missing name-disambiguation boundary; they remain
failed cases, not successful clone rejections. No oracle/harness defect was
found or acceptance rule changed.

Independent audit should check transaction completeness (including inner
commits), imported/aliased enum category identity, absence of whole-unit token
retention or text transport, typed facts needed by subsequent phases, and
source/code responsibility limits. It should not infer full PA5 completion from
the 63 matches. No optimizer, allocator, MIR or object-generation policy changes
were made.

## Foundation checkpoint remaining fixed-manifest failures (historical)

The following 130 case IDs are grouped by the earliest divergent C++ owner.
The failure index links each ID to its protected digest and archived raw trace.

### attribute syntax and retained attribute facts (2)

```text
tests/general/100-balanced-attributes.t
tests/general/200-trailing-parameter-carries-dependency-attribute.t
```

### try statements and exception handlers (3)

```text
tests/general/100-catch-declaration.t
tests/spec/100-switch-try.t
tests/spec/300-declaration-statement-ambiguity.t
```

### template declarations and parameter categories (64)

```text
tests/general/100-class-alignas-after-class-key.t
tests/general/100-decltype-qualified-id-expression.t
tests/general/100-dependent-logical-template-argument.t
tests/general/100-function-type-template-argument-leading-named-parameter.t
tests/general/100-member-operator-name-call.t
tests/general/100-parameter-clause-trailing-ellipsis.t
tests/general/100-parameter-shadows-template-comparison.t
tests/general/100-parameter-shadows-template-strict-comparison.t
tests/general/100-template-argument-paren-greater-nested.t
tests/general/100-template-template-parameter-name-reuse.t
tests/general/100-unary-forms.t
tests/general/200-attributed-partial-specialization-current-class-constructor.t
tests/general/200-attributed-virtual-destructor-member.t
tests/general/200-decltype-less-partial-specialization.t
tests/general/200-dependent-member-template-call.t
tests/general/200-dependent-sizeof-pointer-type-id.t
tests/general/200-dependent-template-id-names.t
tests/general/200-dependent-template-keyword-nested-angle.t
tests/general/200-forward-unknown-nested-template-in-ctor-body.t
tests/general/200-friend-function-template-declaration.t
tests/general/200-inline-namespace-template-visibility-base.t
tests/general/200-malformed-template-parameter-clause.t
tests/general/200-member-relational-ignores-using-template.t
tests/general/200-namespace-alias-using-directive-imported-template-id-type.t
tests/general/200-nested-qualified-template-id-template-args.t
tests/general/200-nested-template-return-assignment-operator.t
tests/general/200-operator-less-followed-by-operator-greater.t
tests/general/200-pack-base-initializers.t
tests/general/200-parenthesized-qualified-function-template-call.t
tests/general/200-partial-specialization-current-class-constructor.t
tests/general/200-placement-new-pack-init.t
tests/general/200-postfix-template-suffixes.t
tests/general/200-qualified-base-member-call.t
tests/general/200-qualified-base-template-id-logical-argument-value-shadow.t
tests/general/200-qualified-call-direct-initializer.t
tests/general/200-qualified-call-member-body.t
tests/general/200-qualified-member-comparison-template-arg.t
tests/general/200-qualified-template-id-parameters.t
tests/general/200-relational-qualified-template-static-calls.t
tests/general/200-reopened-namespace-template-function-template-name.t
tests/general/200-template-class-body.t
tests/general/200-template-conditional-simple-type-shift-return.t
tests/general/200-template-function-body.t
tests/general/200-template-function-type-alias-declaration.t
tests/general/200-template-function-type-alias-pack-declaration.t
tests/general/200-template-id-function-pointer-argument.t
tests/general/200-template-id-function-pointer-initializer.t
tests/general/200-template-id-return-type-inline.t
tests/general/200-template-id-value-if-condition.t
tests/general/200-template-qualified-conversion-operator-dependent-result.t
tests/general/200-template-qualified-inline-attribute-constructor-definition.t
tests/general/200-template-qualified-inline-constructor-definition.t
tests/general/200-using-declaration-imported-template-id-type.t
tests/general/200-using-directive-imported-template-id-type.t
tests/general/300-nested-angle-and-shift.t
tests/general/300-operator-template-calls.t
tests/general/300-value-comparisons-in-template-arguments.t
tests/spec/100-template-class.t
tests/spec/100-template-parameters.t
tests/spec/200-explicit-instantiation-declaration.t
tests/spec/200-explicit-specialization-syntax.t
tests/spec/200-non-type-template-parameters.t
tests/spec/200-qualified-special-member-definition.t
tests/spec/300-template-id-less-expression.t
```

### class specifiers, member scopes and deferred bodies (49)

```text
tests/general/100-known-function-name-with-type-marker.t
tests/general/100-late-nested-class-type-in-inline-member.t
tests/general/100-member-declarations.t
tests/general/100-special-member-definitions.t
tests/general/100-typedef-anonymous-union.t
tests/general/100-typedef-shadows-outer-value-in-member-function.t
tests/general/100-typedef-struct-union.t
tests/general/200-braced-return.t
tests/general/200-builtin-function-style-cast-member-body.t
tests/general/200-casts-typeid-construction.t
tests/general/200-class-definition-missing-semicolon-bad.t
tests/general/200-constructor-declaration-identifier-parameter.t
tests/general/200-decltype-base-and-mem-initializer.t
tests/general/200-elaborated-enum-member-declarators.t
tests/general/200-empty-brace-scalar-init.t
tests/general/200-extern-c-class-declaration.t
tests/general/200-friend-type-declaration.t
tests/general/200-function-throw-typed-specification.t
tests/general/200-function-try-and-exception-declarators.t
tests/general/200-function-virt-and-noexcept-suffixes.t
tests/general/200-global-qualified-pointer-conversion.t
tests/general/200-global-struct-paren-declaration.t
tests/general/200-lambda-capture-forms.t
tests/general/200-member-pointer-const-function-declarator.t
tests/general/200-member-pointer-data-declarator.t
tests/general/200-member-pointer-function-declarator.t
tests/general/200-member-relational-logical.t
tests/general/200-member-template-if-less-template-call.t
tests/general/200-nested-unary-call-arguments.t
tests/general/200-parenthesized-function-style-cast-mem-init.t
tests/general/200-placement-new-identifier-led-initializer.t
tests/general/200-qualified-base-and-ctor-init.t
tests/general/200-qualified-conversion-operator-definition.t
tests/general/200-qualified-nonconversion-operator-missing-return-type-bad.t
tests/general/200-qualified-result-parenthesized-member-pointer-declarator.t
tests/general/200-sizeof-elaborated-class-type-id.t
tests/general/200-sizeof-functional-cast-expressions.t
tests/general/200-sizeof-qualified-value-call-expression.t
tests/general/200-sizeof-zero-arg-functional-cast.t
tests/general/200-template-member-definition-inherited-typedef-cast.t
tests/general/200-trailing-parameter-vendor-attribute.t
tests/general/200-typeid-postfix-member-suffix.t
tests/general/200-zero-width-bit-field-declaration.t
tests/general/300-call-argument-prevents-function-declaration.t
tests/general/300-declaration-expression-intake.t
tests/general/300-function-object-ambiguity.t
tests/general/300-local-typedef-shadows-value-qualified-type.t
tests/spec/200-bit-field-declaration.t
tests/spec/200-class-bases-and-ctor-init.t
```

### lambda expressions and deferred bodies (2)

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-declarator.t
```

### allocation and deletion expressions (1)

```text
tests/general/100-new-delete-traits.t
```

### unresolved template-id calls (syntax divergence) (3)

```text
tests/general/100-template-condition.t
tests/general/200-switch-case-qualified-template-id-call.t
tests/general/200-switch-case-template-id-call.t
```

### operator and special-member names (4)

```text
tests/general/200-allocation-array-operator-ids.t
tests/general/200-literal-operator-id.t
tests/general/200-overloadable-operator-ids.t
tests/general/200-parenthesized-new-type-vs-placement.t
```

### decltype operands and qualified types (1)

```text
tests/general/200-member-template-parameter-value-vs-template-name.t
```

### template-id name disambiguation (1)

```text
tests/general/200-mock-template-name-angle-forms.t
```
