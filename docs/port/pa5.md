# PA5 port ledger

Stage base commit: `d182aa4bc3c8d2a2bfeffb7c615bea7a77a5fd43`
Last audited commit: INITIAL

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

## Exact remaining fixed-manifest failures

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
