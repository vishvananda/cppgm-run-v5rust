# PA5 port ledger

Stage base commit: `d182aa4bc3c8d2a2bfeffb7c615bea7a77a5fd43`
Last audited commit: `0dc9f88edaafe9b64e59f440d8f9b6745dc090bc`


## Full-stage audit continuation: declaration dispatch, GNU initialization and enum ownership — 2026-10-12

Entry/reviewed commit: `bbabb409cd76f046cb599ba2ddbe784f5250e4a8` (clean).
The stage base and last accepted partial-audit commit above remain unchanged;
the new implementation/audit commit is recorded in
`pa5-declaration-audit/handoff.json`. **Full acceptance remains open**. This
continuation fixes additional source-owner gaps beyond the frozen fixtures.
The existing goal remains active; no Ralph state or acceptance policy changed.

Read the repository instructions, contract/specification/plan, full handout and
owning grammar, source map and ledger, and consulted frozen
`parser_declarations.cpp`, `parser_specifiers.cpp`, `parser_state.cpp`,
`gnu_names.h`, `parse_token_stream.h`, `lex/gnu_builtins.{h,cpp}`, the statement
owner and native-budget owners directly. `reviewed-responsibilities.json`
records ten specific responsibility groups and their Rust owners. This is a
source/control-flow review supported by raw comparisons, not a claim that a
navigation inventory or passing counts complete the implementation census.

Findings resolved:

- `declarations.rs` now dispatches known declaration keywords before probing
  special members, selects namespace aliases before collecting namespace
  attributes, and uses the Type name mode for alias targets. Operator,
  destructor and initial `typename` targets fail at the frozen token. Reopened
  named/unnamed namespaces retain their scope without republishing a binding;
  inline/unnamed nominations remain ordered as before.
- Linkage dispatch requires a string literal, including user-defined strings,
  rather than any literal. Numeric/character literals follow the ordinary
  declaration failure path. String spelling interiors, braced/single-child
  order and inherited declaration contexts remain exact. Attributed empty
  declarations retain their unconsumed entry location, while ordinary
  declarations retain their recomputed specifier start.
- Parser construction unconditionally remaps GNU reserved names, even without
  hosted mode. `lex/gnu_keywords.rs` owns all 17 keyword/drop entries.
  `syntax/cursor.rs` observes original token ordinals and packing before
  dropping spellings, and builds `__null` as typed target LongInt zero with
  shared immutable `0L` spelling/data. Ordinary PA2 phase-seven tools continue
  to produce identifier tokens. No literal spelling is reparsed.
- `parser.rs` publishes all 14 extended/complex/bfloat type categories alongside
  the four existing predefined names, before transactions. Compact immutable
  NameId groups select exactly the ten extended types and three complex
  spellings. `declarations.rs` reproduces sign-only/complex-only transitions
  through cv, ordinary and attribute specifiers. Reserved identity checks
  apply regardless of user shadowing, as in C++.
- `enum_specifier` now uses declared Type name mode; forward declarations keep
  an existing Type/member scope; definitions and unscoped enumerators publish
  at declaration home rather than a temporary template scope. A template-id
  enum head does not replace its primary template binding. Attribute filtering,
  enumerator source/value order, underlying specifier grammar, scoped member
  ownership and trailing-comma facts were compared directly.
- The malformed asm keyword/optional-volatile/left-delimiter prefix follows
  ordinary parsing failures; valid assembly bodies remain an explicit later
  grammar path. This does not implement an opaque successful assembly node.

Architecture review: syntax, literal data and names remain owned per unit;
private typed stores and compact IDs cross grammar boundaries. Namespace/enum
publication uses reversible scope APIs, preserving transaction rollback and
query revisions. The synthetic null cache is per Cursor, immutable, and has
no environment-sensitive key or cross-unit lifetime. Reserved NameId groups
are bounded parser metadata initialized once. Deferred bodies still demand
syntax only at their existing commit boundary. Backend lifetimes and optimizer
policies are untouched. No unsafe code, dependency, subprocess fallback,
fixture recognition, text round trip, environment read in compiler phases,
blanket allowance, mechanical split or PA35 objective was introduced.
`Declaration.location` now has a production reader, removing its old dead-code
expectation; the existing future semantic `Literal.data` expectation is narrow
and excluded where invariants read that field.

Nine added invariants cover namespace publication/speculation, empty source
locations, alias rollback, phase-seven versus syntax remapping, shared typed
null storage, extended specifier facts, retained forward enum scope,
declaration-home enum publication and primary-template identity preservation.
Seven new required cases and only their new expectations live in the executed
`pa5/tests/general/300-{declaration-*,parser-gnu-initialization,enum-*}` lane.
Original inputs/references and protected manifest remain unchanged.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-declaration-audit/`;
additional raw GNU and enum entry evidence is in sibling `pa5-gnu-init/` and
`pa5-enum-audit/`. `metadata.json`, `progress.json`, retained case/course
receipts, passing/failing ID lists and `checks.json` record exact invocations,
source/tool identities and commands. Fixed manifest SHA remains
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
193 entry/final passes, zero lost, no fixed failures.

| Command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make lint` / protected `clone_quality.py` | PASS debug/release 110 library + nine tool tests, strict Clippy/rustfmt; `quality.log` |
| `make test-pa5` | PASS 193/193; `stage.log` |
| exact `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS 247/247, zero failed comparisons/markers; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make audit-rust` | PASS 87 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS; all 193 retained; `preflight.log`, `preserve.log` |

Raw declaration entry comparisons were 66/139, GNU initialization 45/116 and
enum ownership 102/130. Final declaration/GNU/continuation sweep is **479/479**
(`final.json`), enum sweep **130/130** (`pa5-enum-audit/final.json`), and all
**2141/2141** prior probes still match (`prior-probes.json`). Every comparison
includes status/stdout/stderr/full AST bytes; no normalization proves equality.
All seven new fixture comparisons match (`new-regressions.json` and enum
sibling receipt). Earlier passing fixed IDs remain identical. These findings
reduce real non-fixture defects without claiming new passing fixtures are
fixed-manifest progress.

Resources are separate compiler observations, not generated-code objectives.
`measure.py` / `measurements.json` / `resource-summary.json` retain 54 warm
whole-invocation runs with oracle/oracle/oracle/candidate/candidate/oracle
order at 1000/4000/16000 independent units. Final timing runs were serial after
validation/probes completed; the earlier concurrent run is archived separately
and is not the table's evidence. All runs match complete output bytes/status,
with tool/source hashes, raw streams and ASTs retained. Median wall milliseconds
include preprocessing, process launch and I/O; RSS is maximum observed KiB.

| Workload | Rust ms (1k / 4k / 16k) | C++ ms (1k / 4k / 16k) | Rust max RSS KiB (1k / 4k / 16k) |
| --- | --- | --- | --- |
| GNU aliases/ignored tokens/typed null and extended types | 42.01 / 110.08 / 357.30 | 18.24 / 52.31 / 177.40 | 8020 / 12124 / 38164 |
| Namespace reopening/alias publication | 32.40 / 74.68 / 238.64 | 11.49 / 28.54 / 110.99 | 8032 / 9992 / 31052 |
| Forward/template enum publication and member queries | 44.65 / 135.72 / 511.95 | 18.49 / 66.59 / 237.90 | 8024 / 19600 / 68564 |

C++ maximum RSS respectively reaches 34664, 24484 and 57844 KiB at 16k.
The sampled growth is subquadratic, without a claimed speed target. Scratch
and checkout together remain well below the 100 GiB ceiling.

Acceptance is still withheld. `final-native.json` / `final-native/` reproduce
45/48 raw native-budget comparisons and these exact remaining IDs:

| Owned failure ID | C++ | Rust |
| --- | --- | --- |
| `final-native/parentheses/4800` | exit 0 | exit 1, column 3861 |
| `final-native/known-templates/2400` | exit 0 | exit 1, column 3748 |
| `final-native/known-templates/4800` | exit 1, column 7627 | exit 1, column 3748 |

Rejections say `constructs nested too deeply`; neither compiler panics, signals
or returns 86 in the native sweep. Native frame layout changes the frozen
cutoff even with identical budget arithmetic. These ordinary diagnostic/exit
mismatches are acceptance failures, not excluded runtime statistics; no
calibrated depth, multiplied budget or new exception hides them. No frozen
oracle defect is established. The next dependency boundary remains exact
native rejection parity and final per-function/header responsibility review.
`census-scope.json` explicitly keeps full census/acceptance false. Hosted-only
CLI/statistics and GNU trait/statement-expression/asm/modern grammar remain
outside the owning PA5 grammar; unconditional constructor facts above are due.

## Full-stage audit continuation: native nesting budget and boundary evidence — 2026-10-12

Entry/reviewed commit: `7df821d6b52cfc823966917bd34b579cd1d8473d` (clean).
Stage base and last accepted partial-audit commit above remain unchanged. The
commit containing this entry is recorded in `pa5-nesting/handoff.json`.
**Full acceptance remains open**, with the original goal active. The previous
turn made verified progress by committing source facts and original packing
observations. This continuation replaces the fixed nesting stub with real
Rust budget enforcement, resolves the 300-parenthesis case, and exposes more
precise remaining resource-boundary differences. It does not redefine success.

Consulted frozen `support/stack_budget.h`, `syntax/parser_regions.h`,
`parser.cpp`, parser state constructor, every C++ NestingGuard site in the
expression/declaration/declarator/name/statement owners, and required driver,
handout/grammar, repository contract/specification/plan and prior ledger.
The oracle and protected acceptance policy are unchanged.

Finding: the old Rust depth-256 status-86 path was not a successful C++
rejection. `support/stack.rs` now owns the exact half-soft-limit arithmetic,
256 MiB cap, 4 MiB unknown fallback and strict greater-than comparison in
either stack direction. `dev/bin/source_tool/stack_limit.rs` reads Linux's
current soft RLIMIT_STACK from `/proc/self/limits` and distinguishes finite,
unlimited and unavailable values. The entry point converts it once to a typed
StackBudget and passes it through `driver/ast.rs` to SyntaxUnit. No environment
or process query moved into compiler phases; no FFI/dependency was added.

SyntaxUnit records a numeric root stack address; `parser.rs` compares it with
an actual local marker at each recursive boundary, using safe standard-library
`from_ref(...).addr()` and `abs_diff`. The transient pointer is neither stored
nor dereferenced; syntax remains in private typed stores and IDs. Exceeding the
budget returns the located `constructs nested too deeply` CommittedSyntax
failure, escaping every tentative reading like C++'s ParseError exception.
Normal Result unwinding restores active nesting and transaction state.

C++ logical nesting sites now have the corresponding Rust owners: declaration,
statement, declarator, parameter clause, cast/unary operand, braced list,
template argument list and individual argument. Removed duplicate declaration
and assignment/template-parameter nesting levels, added the previously missing
parameter-clause level. Assignment/throw and template-parameter recursion still
check physical bytes without inventing C++ logical nesting levels; this keeps
the safe Rust call chain guarded even where the oracle has no extra guard.
The balanced-skip scratch remains iterative and does not demand grammar.

Four new invariants cover half/cap/fallback, odd and zero limits, unlimited/max
input, checked host conversion and address direction; the tool's soft/hard
field selection and malformed/missing data; terminal failure escaping an
attempt while typed stores/cursor/speculation restore; and a 64 KiB configured
parse budget unwinding safely inside an 8 MiB test thread. Debug and release
each pass **101 library plus nine tool tests**. Strict Clippy/rustfmt and the
**86-file** authoritative audit pass with zero findings. The two new modules
own host-limit collection and shared budget arithmetic, not mechanical chunks.
No allowance, unsafe, C++ fallback, fixture recognition, acceptance exception,
optimizer/register allocator change or PA35 target was added.

Two new required executed cases in `pa5/tests/general/300-nesting-*.t` cover
300 nested parentheses and 300 layers of unary/declarator/block/braced/template
syntax. Only their new expectations were created; original fixtures/references
and Ralph state remain unchanged. They extend the course suite without changing
fixed-manifest progress or certifying the resource boundary.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-nesting/`.
Metadata, entry/final receipts, `progress.json`, passing/failing ID lists,
`cases/` and PA1–PA5 `course/` receipts preserve commands and results. Manifest
hash remains `27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
193 entry/final passes, zero lost, no fixed failures.

| Command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make lint` / protected `clone_quality.py` | PASS; debug/release 101 library + nine tool tests, strict Clippy/rustfmt, source audit; `quality.log` |
| `make test-pa5` | PASS 193/193; `stage.log` |
| exact required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS 240/240, no failed comparisons/marker; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make audit-rust` | PASS 86 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS; 193 retained; `preflight.log`, `preserve.log` |

`validate.py` / `checks.json` record commands, exit status and elapsed costs.
All **631/631** recent comparisons and **1510/1510** prior probes still match
complete status/stdout/stderr/AST bytes. `new-regressions.json` retains both
new strict course comparisons. `pending-probes.json` now resolves the earlier
`deep-parenthesized-expression` with C++/Rust exit 0 and identical full output;
category fixes remain preserved. The fold/statement-expression exploratory
mismatches retain their established outside-PA5-grammar classification.

**Resource-boundary fidelity is not complete.** `profile.py` sweeps six owning
forms at 64, 128, 256, 300, 600, 1200, 2400 and 4800 layers, comparing raw bytes
and retaining original outputs (JSON stores their hashes/lengths, not a
normalization). `initial-native.json` matched 46/48 before guard-site alignment;
`final-native.json` / `final-native/` match 45/48 after it. Neither compiler
panics, signals or returns 86 in this sweep. The current soft limit is 8 MiB,
with the same 4 MiB budget, but C++ and Rust consume different native frames:

| Exact remaining case ID | C++ result | Rust result |
| --- | --- | --- |
| `final-native/parentheses/4800` | success | located budget rejection at column 3861 |
| `final-native/known-templates/2400` | success | located budget rejection at column 3748 |
| `final-native/known-templates/4800` | budget rejection at column 7627 | budget rejection at column 3748 |

The byte-budget algorithm and terminal-error transport are implemented, but
these complete-result differences are acceptance findings, not runtime-field
exceptions. No limit was multiplied or depth calibrated to fixtures to hide
them. `remaining-probe-ids.txt` preserves the exact next work. Their resolution
still requires investigation of the actual frozen resource rejection behavior;
matching a policy formula does not prove matching its externally visible bytes.

`cpp-functions.txt` / `function-census.json` index **220** C++ functions and map
responsibility groups to Rust owners. `census-scope.json` explicitly labels this
as a navigation inventory: anonymous helpers and inline header functions are
not enumerated and the final function-level audit is pending. Known extended
forms absent from the owning grammar are classified separately. This is not a
completion certificate or a source-owner count offered as proof of equivalence.

Resource command: `python3` on this root's `measure.py`, after all validation
and quality handles were terminal. Tool/input/output hashes, Linux/rustc 1.98.0,
UTC/C and warm whole invocation conditions are recorded. Oracle A/A calibration
then A/B/B/A blocks match all **36** complete ordinary-workload byte results;
these are separate from the failing nesting-boundary sweep. Raw outputs,
times/RSS and hashes are in `resources/`, `measurements.json` and `measure.log`.
Measurements include preprocessing, host clock/limit collection and I/O on a
shared host, not a PA35 generated-code objective.

| Workload | Count | Rust mean ms | Rust max RSS KiB | C++ mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| budgeted-functions | 1000 | 42.10 | 8020 | 20.27 | 7220 |
| budgeted-functions | 4000 | 122.85 | 13540 | 53.82 | 13512 |
| budgeted-functions | 16000 | 442.21 | 43300 | 195.90 | 39296 |
| budgeted-classes | 1000 | 42.53 | 8044 | 17.50 | 7220 |
| budgeted-classes | 4000 | 108.07 | 14208 | 47.71 | 13568 |
| budgeted-classes | 16000 | 376.13 | 46400 | 180.14 | 39136 |

Fourfold 4000 -> 16000 growth takes about 3.5–3.6 times Rust wall time on these
shallow functions/classes. Syntax and scope storage remain proportional to
introduced work; the shared byte guard stores two numeric values per parser,
not a frame/token history. Checkout/scratch occupy about 11 GiB of the 100 GiB
budget, including raw deep dumps retained for failure evidence.

Next dependency: investigate and resolve the three exact frozen resource-limit
counterexamples, then finish per-function/header responsibility review against
the authoritative PA5 grammar and required driver surface. Acceptance and goal
completion remain unproven. The goal stays active; no Ralph state was edited.

## Full-stage audit continuation: source facts and packing observations — 2026-10-12

Entry/reviewed commit: `eeb0e48954aaad0e00fbaadab74cd26328a2906a` (clean).
Stage base and last accepted partial-audit commit above remain unchanged.
The commit containing this entry is recorded in `pa5-source-facts/handoff.json`.
**Full acceptance remains open**, with the original goal active. The previous
turn made verified progress: committed category/pack and class-query owners,
zero lost fixed passes, and concrete scaling improvement. This continuation
completes the connected retained source-fact responsibility beyond dump coverage.

Consulted frozen `ast{,_names,_types,_declarations,_statements,_expressions}.h`,
`parser_names.cpp`, `parser_specifiers.cpp`, `parser_declarators.cpp`,
`parser_declarations.cpp`, `parser.cpp`, `parse_token_stream.*` and
`syntax_unit.cpp` directly at the fixed oracle commit. The repository guide,
contract, specification, plan, complete PA5 handout/parsing guide and source
map/ledger remain the requirements. No oracle/harness defect was found.

| C++ responsibility and finding | Rust correction |
| --- | --- |
| `DeclSpecifier`, `FunctionQualifier`, `Initializer` source locations | `ast.rs` now stores each as a located typed kind/payload in its existing list/record owner. All grammar producers retain the exact start, including vector Attribute locations, shared-name type-id starts and named/decltype functional casts. The printer/renderer and parser consumers read the same kinds and IDs; no second tree or later text reconstruction is required. |
| Parameter/clause/default boundaries (`parser_declarators.cpp`) | `declarators.rs` captures the clause's opening parenthesis and parameter's start before attributes/specifiers. A parameter-default Initializer's location is the token after `=`, matching C++; ordinary initializer records instead start at their introducer. Every cv/ref/noexcept/throw/trailing/virt qualifier has its own location. |
| Root names and special members (`parser_names.cpp`, `parser_declarations.cpp`) | `names.rs` retains the root Name location separately from its components and typename-specifier start. `classes.rs` now gives a special-member Declarator its Name's location; the old location incorrectly came from the outer declaration's first member-function specifier. |
| Class/enum/constructor facts (`ast_declarations.h`) | ClassSyntax owns its absolute class-key/closing-brace token interval, base-clause presence and class-key packing. The exclusive end precedes trailing GNU attributes, like C++. Enumerator locations and constructor clause colon location remain typed on their owners. Existing enum/statement/type locations cover their other C++ source responsibilities; Handler/Pointer/Capture have no missing C++ location field. Function-try location is the compound Statement's location, as the oracle constructs it. |
| Simple init-declarator token intervals (`parser_declarations.cpp`) | Each InitDeclarator records its first token and the token after its initializer/before comma or semicolon. `parser.rs` explicitly keeps the low target 32 bits of the unit ordinal, matching the frozen cast independently of host width; ClassSyntax spans retain full u64 ordinals. |
| `observe_pack`, `pack_at` (`parse_token_stream.cpp`, `syntax_unit.cpp`) | `cursor.rs` records only packing changes observed after the shared post-token demand, indexed by original absolute token ordinal. A partition query selects the state at the class-key. Lookahead, window compaction, deferred replay and parser rollback therefore cannot substitute the preprocessor's later current state. The existing Rust PA4 pragma owner supplies the typed alignment; no re-interpretation of pragma text occurs. |

This is parse-time fact preservation, not semantic layout/instantiation work.
Every location is the shared SourceOffset; names and operands are compact IDs.
The existing slabs/list marks rewind the new facts with their owning records.
Packing observations belong to irreversible preprocessing demand and survive
parser rewind; the class query uses the original token ordinal. There is no
per-token packing vector, whole-unit materialization in production, owning
spelling copy, rendered transport or fallback. `source-fact-owners.json` records
the exact owner review, including already represented or absent C++ fields.
The initializer builder also restores its delimiter angle context like C++.
No target numeric evaluation, optimizer/register allocator policy or backend
lifetime changed.

Five independent invariants inspect the facts rather than matching a dump:
individual specifier/initializer and target token bounds; parameter/default/
qualifier/Name/type/enum/ctor locations; packing and brace ranges with all tokens
already looked ahead and a deferred local class; complete rollback of source
records while retaining packing history; and an intervening macro `_Pragma`
after the class-key plus shared typename/name location differences. Debug and
release each pass **98 library plus eight tool tests**. Strict Clippy, rustfmt
and the authoritative **84-file** audit pass with zero findings. Narrow
non-test dead-code expectations on source facts retained for later consumers
have field-specific reasons and direct invariant use; unused or now-consumed
expectations were removed. No blanket allowance, unsafe, dependency, process/
environment read, mechanical split or fixture tailoring was added.

Three new required fixtures and only their new oracle expectations are in
`pa5/tests/general/300-source-facts-*.t`, the executed owning lane. Frozen
fixtures/references, acceptance policy and Ralph state are unchanged.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-source-facts/`.
Metadata, entry/final receipts, `progress.json`, passing/failing ID lists,
`cases/` and PA1–PA5 `course/` receipts preserve all results. Manifest SHA-256
remains `27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
193 entry/final passes, zero lost, no fixed failures. These facts and owner
review provide architecture evidence; new passing fixtures do not redefine
fixed-manifest progress or prove the still-pending nesting policy.

| Command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make lint` / protected `clone_quality.py` | PASS; `quality.log` (97 library tests), final `quality-facts-final.log` (98 after adding the macro/typename invariant), eight tool tests each debug/release, strict Clippy/rustfmt and file audit |
| `make test-pa5` | PASS 193/193; `stage.log` |
| exact required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS 238/238, zero failed comparisons/no marker; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make audit-rust` | PASS 84 files, zero findings; `audit.log`; repeated within final quality run |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS, all 193 retained; `preflight.log`, `preserve.log` |

`validate.py` / `checks.json` preserve commands, statuses and elapsed costs.
`final.json`, `final/` / `parameter-sources.json` retain **631/631** complete
exit/stdout/stderr/AST comparisons. Prior probe receipts retain **1510/1510**:
147 attributes, 114 earlier audit, 334 parameter bases, 135 lambdas, 704 shared
operands and 76 templates. `new-regressions.json` records all three new strict
fixture comparisons. The final extra invariant changes only test code; all
production source and fixtures were already validated by the full gate run.

Resource command: `python3` on this root's `measure.py`, after the full gate and
final quality handles were terminal. Tool/input/output hashes, Linux/rustc
1.98.0, UTC/C and whole invocation conditions are frozen in metadata. Oracle
A/A calibration then A/B/B/A blocks match all **36** complete byte results.
`resources/`, `measurements.json` and `measure.log` preserve raw times/RSS and
outputs. These are warm observations on a shared host, including preprocessing,
clock query and I/O, separate from generated-code equality and any PA35 target.

| Workload | Count | Rust mean ms | Rust max RSS KiB | C++ mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| located-parameter-functions | 1000 | 48.27 | 8024 | 16.74 | 7244 |
| located-parameter-functions | 4000 | 120.96 | 13508 | 58.52 | 13492 |
| located-parameter-functions | 16000 | 493.21 | 42452 | 206.83 | 39296 |
| packed-deferred-classes | 1000 | 42.21 | 8012 | 18.27 | 7176 |
| packed-deferred-classes | 4000 | 148.01 | 14200 | 57.55 | 13556 |
| packed-deferred-classes | 16000 | 393.25 | 46372 | 201.29 | 39144 |

Fourfold 4000 -> 16000 growth takes about 4.1 times Rust time for the larger
located parameter/specifier records and 2.65 times for packed deferred classes;
owned syntax/packing-change storage grows with source demand. These data do
not certify identical compiler speed or hardware-counter behavior. Checkout
and scratch occupy about 8.9 GiB of the shared 100 GiB capacity.

The exact remaining acceptance case `deep-parenthesized-expression` remains
C++ exit 0 / Rust 86 at 300 parentheses, preserved in `pending-probes.json` /
raw paths. The two category cases stay resolved. The exploratory unhosted fold
and statement-expression mismatches remain outside PA5's authoritative grammar,
as established in the previous owner review. Next: replace the safe parser's
nesting stub with the byte-budget policy and audit failure boundaries, then
finish the complete PA5 grammar/driver responsibility census. Full-stage
acceptance and goal completion remain unproven; no goal status change is made.

## Full-stage audit continuation: completed-class query ownership — 2026-10-12

Entry/reviewed commit: `6f8b1eb1a04aa734011a6869c5816c33edb1bca6` (clean).
Stage base and last accepted partial-audit commit above remain unchanged; this
commit is recorded in `pa5-class-lookups/handoff.json`. **Full acceptance remains
open**, with the original goal active. The prior category/pack increment and
all 193 fixed passes remain preserved. This completes the connected class
search memo responsibility, with resource evidence beyond fixture parity.

Consulted `name_scopes.cpp/.h` push/declare/complete_class/query/search,
`memo_key`, bounded memo_slot/memo_limit, name/edge epochs and undo/detach/
reattach, plus `parser_specifiers.cpp`'s completion call. Required repository,
contract, specification, plan, full handout/parsing guide and source map/ledger
remain the audit basis. Frozen oracle commit is unchanged.

Finding: Rust's iterative traversal preserved order and cycle handling, but
walked an inherited chain afresh for each method. Fourfold 4000 -> 16000 growth
cost roughly fifteenfold time, versus near-linear C++. This was missing due
compiler work despite matching all ASTs. `scopes.rs` now owns bounded completed
class query results, including explicit negative results. `classes.rs` supplies
the template/specialization completion fact; eligibility follows C++'s ordinary
complete-class and eligible non-dependent-base rule.

The memo identity is stable ScopeIdentity, interned NameId and prefix demand,
with separate ordinary/specialization base-demand slots. Both demand modes share
one census entry and the C++ capacity policy: at least 65,536 query identities,
or four times the retained binding-slot count; a full table renews old entries
and declines new ones. The base-demand slot is part of the full key even for a
class whose eligibility was established before a subsequent graph mutation.
A typed SearchDecision distinguishes a finished negative result from a search
still needing edges. A reusable explicit DFS frame stack retains last-added
edge preference, visited-cycle handling, own-binding precedence and member-name
hints. Successful unwind publishes ancestor results; completed failure records
a negative. Nothing reparses syntax or formats names.

Binding storage uses stable identity plus NameId and optional owned Binding;
rollback leaves the same conservative empty-slot census as C++'s table, rather
than recycling an abandoned scope's binding key. Name revisions invalidate
only that name (on class/enum declaration and logged undo/redo). Reverse base
edges invalidate graph revisions of the changed scope and its actual dependents,
without C++'s translation-unit-wide class-edge generation. The chosen results
and capacity/order remain unchanged, while meeting the specification's targeted
invalidation requirement. Revision exhaustion removes only the affected name
or scope's queries before restarting its epoch; no wrapping can revive stale
results. Undo/redo updates reverse edges, completion facts and revisions before
released ScopeId slots can be reused. Memos are unit owned and bounded; stale
scope identities cannot be queried as a new scope.

Four independent invariants prove linear visit count across a 1024-class chain,
warm queries after unrelated mutations, negative/prefix/base-demand distinctions,
name and dependent graph invalidation, rollback with cached discarded scope
handles and recycled ScopeIds, and full-table renewal/refusal at the 65,536
minimum. Debug and release each pass **93 library plus eight tool tests**.
Strict Clippy, rustfmt and the authoritative **84-file** audit pass with zero
findings. No allowance, unsafe, new dependency, process/environment read,
compiler fallback or code-generation policy change was introduced. The growing
scope module remains within its model/function/nesting limits; no mechanical
split was needed. The new required executed case is
`pa5/tests/general/300-class-query-contexts.t`, with only its new expectations.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-class-lookups/`.
Metadata, entry/final receipts, `progress.json`, passing/failing lists, `cases/`
and all earlier/full `course/` receipts are archived. Manifest hash remains
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
193 entry/final passes, zero lost and no fixed failures.

| Command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make lint` / protected `clone_quality.py` | PASS; debug/release invariants, strict Clippy, rustfmt, file audit; `quality.log` |
| `make test-pa5` | PASS 193/193; `stage.log` |
| exact required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS 235/235, no failed comparisons/marker; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make audit-rust` | PASS 84 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS, all 193 retained; `preflight.log`, `preserve.log` |

`validate.py` / `checks.json` retain commands, statuses and elapsed costs.
`final.json`, `final/` and `parameter-sources.json` match **631/631** complete
exit/stdout/stderr/AST results: the 597 category/pack sweep plus long inherited
chains, type/value/template members, qualified/prefix uses, shared-base diamonds
and opposing base order. Final prior probes retain **1510/1510** comparisons.
`new-regressions.json` retains the required case. Views match the oracle, and
invariants prove the hidden cache/ownership properties.

Resource command: `python3 measure.py before` / `python3 measure.py after`.
The before run finished before changing the candidate binary; the after run
started after final validation was terminal. Metadata freezes both binaries,
Linux/rustc 1.98.0, UTC/C and warm whole invocations including preprocessing,
clock query and I/O. Oracle A/A calibration then A/B/B/A blocks match all **18**
complete results in each run, with identical input/output hashes across runs.
Raw files/RSS/times are in `resources-before/`, `resources-after/`, measurement
and metadata JSON. These observations on a shared host are separate from
code-generation equality and impose no PA35 objective.

| Classes | Rust before mean ms | Before max RSS KiB | Rust after mean ms | After max RSS KiB | C++ after mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1000 | 114.94 | 8008 | 38.12 | 8104 | 15.07 | 6516 |
| 4000 | 1039.60 | 10660 | 89.73 | 12120 | 36.36 | 10728 |
| 16000 | 15745.02 | 31640 | 247.04 | 38576 | 112.68 | 29744 |

Fourfold 4000 -> 16000 growth now takes about 2.75 times Rust time; the 16000
case improves from about 15.75 seconds to 0.247 seconds. RSS increases from
about 31 MiB to 38 MiB for the bounded owned query table and reverse edges.
The input/output bytes are identical; this is a parser scaling observation.

The exact acceptance failure `deep-parenthesized-expression` remains C++ 0 /
Rust 86 at 300 parentheses, preserved in `pending-probes.json` / raw paths.
The two category failures stay resolved. Additional exploratory IDs
`unhosted-fold-expression` and `unhosted-statement-expression` produce C++ 0 /
Rust 1, but owner review now establishes that both forms are absent from
PA5's authoritative `pa5.gram` primary-expression productions. They are outside
PA5's defined syntax boundary, even though the frozen later compiler recognizes
them without hosted mode; they remain later syntax owners, not PA5 acceptance
failures. The previous entry's unreviewed fold boundary is thereby resolved
without forcing future grammar into this stage or changing any gate.

Next dependency: replace the host nesting stub with the safe byte-budget
policy and preserve failure locations; audit and complete due source facts
(specifier/parameter/qualifier/initializer locations, declaration/class token
spans and pragma-pack observation ownership). Finish the responsibility census
against the authoritative PA5 grammar/driver surface, distinguishing later
hosted grammar, statistics and semantic facts from due work. Full-stage
acceptance still cannot be inferred from green fixtures or this memo increment.

## Full-stage audit continuation: category readings and pack facts — 2026-10-12

Entry/reviewed commit: `f29a5ac9498abca8f5fb7d6661cd9decaf47a7dd` (clean).
Stage base and last accepted partial-audit commit above remain unchanged.
The commit containing this entry is recorded in `pa5-categories/handoff.json`.
**Full acceptance remains open**; the existing goal is active. This increment
resolves both previous category counterexamples and the connected C++ owners,
with zero lost fixed-manifest identities. No policy or Ralph state changed.

Consulted the frozen `name_scopes.*`, `name_conflicts.h`, `parser_names.cpp`,
`parser_specifiers.cpp`, `parser_statements.cpp`, parser state/regions,
`parser{,_declarations,_declarators,_expressions}.cpp`, AST/printer/render owners
at `905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`, under the already-read repository
contract, specification, plan, complete handout/parsing guide and ledger.
No oracle/harness defect was found.

| C++ responsibility | Rust owner and completed behavior |
| --- | --- |
| Specialization category aggregation, based specializations, late primaries, namespace lookup tie order and class forwards (`name_scopes.cpp`) | `scopes.rs` retains typed category sets by stable lexical template home/name/member identity, includes primary categories, preserves conservative hints across rewinds, and excludes recycled scope slots from identity. Type/value hiding keeps the type's scope. Primary forward preference and earliest unfinished specialized forward, qualified member owner versus publication home, inline namespace templates and absent-primary-scope linking match C++. |
| `resolve_component`, `advance_prefix`, `note_type_name` (`parser_names.cpp`) | `name_lookup.rs` owns the shared typed query context: known/unknown/dependent prefix, ordinary/specialization/dependent template kind, approximate/dependent guesses, concrete alias argument records, parameter-base queries, current instantiation and dependent conflicting-member rules. Declaration modes and namespace fallback retain their different decisions. Using declarations publish fresh category/scope facts; directives reject typename and trailing attributes. |
| `name_conflicts.h`, `read_statement`, control guards and for-init readers | `conflicts.rs`, `names.rs`, `statements.rs` retain the first conflicted Name/component and up to four successful typed statement readings, in C++ category order, with equal consumed token range and distinct reading categories. Control bodies reset conflict selection; for-init ambiguity owns the same compound wrapper as C++. Printer/render select the first owned reading. No text transport or second semantic tree exists. |
| Shared deferred lambda bodies (`parser.cpp`) | `lambdas.rs` shares one successful typed StmtId under the complete key (statement-reading region, absolute body token ordinal). Nested body readers use their own region; release/rollback removes handles before their syntax is discarded. Source-order scope detach/reattach and terminal committed body failure are preserved. |
| Parameter-pack bindings/mention consumption (`parser_names.cpp`, template/declarator/expression/body owners) | Template bindings retain a pack fact. Observations occur at the same prefix/terminal lookup boundaries; parameter, argument, throw/base/constructor and expression expansions consume their mentions transactionally. Unnamed `T...` distinguishes an unexpanded template pack from C varargs even when the dump renders them identically. `expressions.rs` also stops a binary operand before an operator followed by fold ellipsis, fixing four rejection-location differences. Full fold syntax is still an unreviewed owner. |

Architecture review: unit-owned flat stores and compact typed IDs remain the
syntax owner. Alternatives retain their real trees, with shared lambda bodies;
there is no deep clone or raw pointer graph. `Id<T>` equality compares only its
compact index. Conservative hints have stable non-reused scope identity while
active scope records remain compact and reversible. Category query keys separate
same-named templates in distinct namespaces. Transactions rewind all added
reading/list/pack/cache facts. Lambda success handles never escape the query
region, and failed parses cannot leave cached discarded IDs. Deterministic
output uses source traversal and default-first readings, not hash iteration.
The new modules own distinct algorithms, not mechanical file chunks.

Six additional invariants check alternative ownership/category/component,
parse-once shared lambdas, scope identity after rollback, independent namespace
keys, for-init wrapper ownership, specialized member forward ownership, aborted
query cache cleanup and the stored pack/varargs distinction. Debug and release
pass **89 library plus eight tool tests**. Two narrow non-test dead-code
expectations preserve the ambiguous Name/component for later selection; the
invariants inspect both. No blanket allowance, unsafe, dependency, fallback,
fixture recognition or environment read in compiler code was added. Strict
Clippy/rustfmt and the authoritative **84-file** audit pass with zero findings.
Optimizer/register allocator policy is unchanged; no target evaluation changed.

Eleven new required fixtures and only their new reference artifacts are in
`pa5/tests/general/300-category-*.t`, the executed owning lane. Original frozen
references are unchanged. The first course rerun exposed a symbolic status
format error in the two newest references; those new files were corrected.
`checks-before-final.json` and `course-before-final/` preserve the failed run;
final `checks.json` and archived `course/pa5/` contain the successful rerun.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-categories/`.
Metadata, start/final stage receipts, `progress.json`, passing/failing ID lists,
`cases/` and `course/pa1/` through `course/pa5/` preserve the evidence. Manifest
SHA-256 remains `27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
**193 entry/final passes, zero lost, zero fixed failures**. Supplemental probes
and fixtures do not redefine that manifest or certify pending owners.

| Command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests, file audit; `quality.log` |
| `make test-pa5` | PASS 193/193; `stage.log` |
| exact required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS 234/234, zero failed comparisons/no marker; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make audit-rust` | PASS 84 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS; 193/193 retained; `preflight.log`, `preserve.log` |

`validate.py` / `checks.json` retain commands, exit status and wall costs.
`probe.py`, `parameter-sources.json`, `broad-final.json` and raw `broad-final/`
match **597/597** complete exit/stdout/stderr/AST comparisons. Coverage includes
all four primary/specialization categories, dependent/typename/template and
namespace contexts, late/based specializations, independent lexical homes,
all control/for heads, shared/failed lambdas, qualified forwards and type/value
hiding, forced equal-depth namespace import heap ties, absent primary scopes,
all three kinds of template pack, nested/consumed expansions and invalid forms.
Initial category evidence `entry.json` contains 24 mismatches in 169 programs;
`broad-before-fold-boundary.json` preserves the four broader diagnostic findings.
`new-regressions.json` holds eleven strict fixture comparisons. Final prior
probes match **1510/1510**: 147 attributes, 114 earlier audit, 334 parameter
bases, 135 lambdas, 704 shared operands and 76 template cases. Counts alone do
not prove the retained facts; the invariants and owner review do that work.

Resource command: `python3` on this root's `measure.py`, after final validation
was terminal. Metadata freezes source/tool/output hashes, Linux/rustc 1.98.0,
UTC/C locale. Oracle A/A calibration then A/B/B/A comparisons match all **54**
complete byte results. These are warm whole invocation measurements, including
preprocessing, clock query, source/output I/O, on a shared host. They measure
parser work separately from generated-code equality and impose no PA35 target.
Raw invocations, RSS and hashes are in `resources/` / `measurements.json`.

| Workload | Functions | Rust mean ms | Rust max RSS KiB | C++ mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| category-statements | 1000 | 37.08 | 8020 | 15.14 | 6520 |
| category-statements | 4000 | 100.97 | 10660 | 46.89 | 11508 |
| category-statements | 16000 | 304.61 | 33924 | 131.96 | 30436 |
| category-lambdas | 1000 | 50.13 | 8020 | 21.84 | 7944 |
| category-lambdas | 4000 | 132.64 | 16280 | 55.93 | 14968 |
| category-lambdas | 16000 | 448.34 | 54336 | 219.66 | 45236 |
| category-for-init | 1000 | 45.97 | 8032 | 21.38 | 6516 |
| category-for-init | 4000 | 139.53 | 12064 | 60.97 | 12788 |
| category-for-init | 16000 | 512.49 | 38844 | 254.08 | 36788 |

Fourfold growth from 4000 to 16000 functions takes roughly 3.0–3.7 times Rust
wall time; storage grows with retained syntax. Shared deferred body demand is
one per lambda source region, directly proved by the invariant. Checkout and
scratch occupy approximately 8.2 GiB of the shared 100 GiB budget.

`pending-probe.py` now resolves `specialization-namespace-type` and
`dependent-conflicting-category` with identical complete results. The exact
remaining case `deep-parenthesized-expression` still has C++ exit 0 / Rust 86
at 300 parentheses; raw `pending-probes.json` / `pending-probes/` retain it.
Next: review and port the host stack-budget boundary, inherited class lookup
memo/scaling and missing source locations/token spans/pragma-pack observations.
Full non-hosted grammar/driver ownership review remains necessary; folds and
GNU/hosted owner boundaries must be established from the frozen implementation,
not dismissed by fixture coverage. Future-stage semantic/backend work stays in
its owning stage. This increment is progress, not full acceptance or a goal
completion request.

## Full-stage audit continuation: shared named operands — 2026-10-12

Entry/reviewed commit: `edaf54fa531a6d1dc3507d7d343cbc29f0fdbfaa` (clean).
Stage base and last accepted partial-audit commit above are unchanged. The
implementation/audit commit containing this entry is recorded externally in
`pa5-shared-operands/handoff.json`. The previous goal turn is **progress**:
its committed parameter-base fixes and receipts prove 193 fixed passes with
concrete additional parity findings. This continuation resolves both pending
record-capacity counterexamples and their shared grammar owner. **Full stage
acceptance remains open**; Ralph's existing goal and stage are unchanged.

The repository guide, clone contract, specification, clone plan, complete PA5
handout/parsing guide, grammar, source map and historical ledger remain the
audit requirements. Revalidated the worktree and preceding handoff. Consulted
the actual frozen `parser_expressions.cpp`, `parser_declarators.cpp`,
`parser_specifiers.cpp`, `parser_names.cpp`, parser/state and AST owners at
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`; no oracle/harness defect was found.

| Frozen C++ responsibility | Finding and Rust correction |
| --- | --- |
| `parser_declarators.cpp::parse_type_id_from_name/finish_type_id`, `parser_names.cpp::parse_template_argument` | Rust's shared-name type builder allocated an alias specialization record that C++ does not: C++ deliberately leaves `SpecifierInfo::type_name` unset in this builder. Nested template argument readings consumed the 255-record capacity and changed a later inherited alias declaration to multiplication. `declarations.rs::type_from_name` now uses the existing named syntax, ordinary owning specifier continuation and abstract declarator parser, with no alias record. `templates.rs` consumes that same builder. Both prior capacity failures now match; no speculative-slot workaround is retained. |
| `parser_expressions.cpp::parse_sizeof_operand/parse_type_trait/parenthesized_from_name` | Rust reparsed a name and its argument syntax after a failed type reading. `expressions.rs` parses the name outside the transaction, then tentatively parses only its remaining derivations. Expression fallback consumes the same Name and full postfix/binary/assignment/comma continuation. Sizeof strips only an unchanged outer parenthesis; operand postfix suffixes retain their parenthesized node. Cast and trait delimiter regions restore the angle environment. |
| `parser_expressions.cpp::parse_unary_expression/parse_postfix_expression` | The existing Rust path incorrectly allowed postfix calls, member access and subscripts on unparenthesized alignof/noexcept expressions. They now remain unary expressions; typeid retains its permitted postfix behavior. The broader generated sweep exposed 153 formerly accepted mismatches in this family, now rejected with exact diagnostics. |
| Optional cast/type delimiter and operand checks | A complete tentative type followed by an unsuitable delimiter/operand called fail and contaminated the furthest diagnostic. The reading now declines without manufacturing a grammar failure, fixing both decltype/typename missing-cast-operand rejection locations. Failures inside the actual grammar still retain their original diagnostics. |

This is syntax work in Rust, not semantic instantiation or later code-generation
work. The shared Name and nested TypeId/ExprId arguments remain in the existing
translation-unit slabs. A tentative type reading adds only specifiers and
declarator/type records after its checkpoint; rewind preserves the Name allocated
before it and removes discarded derivations. The owning specifier grammar handles
cv qualifiers, recognized vector/attribute syntax and typed operands for both
ordinary and shared-name type-ids; no duplicate miniature specifier parser remains
in the template module. Continuations consume typed expressions, never rendered
text. Ordinary aliases still retain their existing specialization records.

Output ordering is the same source traversal. Literal numeric facts remain the
PA2 owner; casts/traits do not evaluate target values. Existing function/class/
lambda body demand and bounded token retention are preserved. No unsafe, C++
fallback, dependency, fixture recognition, environment reads, mechanical file
split or lint allowance was added. Optimizer and register allocator policy is
unchanged. Two new invariants verify exactly nine names across three nested
trait/cast operands, no spurious alias records, and abandoned type derivations
leaving shared names/argument syntax live. Debug and release each pass **83
library plus eight tool tests**. Five new required cases and only their new
oracle expectations live in the executed `tests/general/300-shared-named-*.t`
lane. Existing fixtures/references and protected policy are unchanged.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-shared-operands/`.
`audit-metadata.json`, `start-results.json`, `pa1-results.json` through
`pa5-results.json`, `progress.json`, passing/failing ID lists, archived `cases/`
and `course/pa1/` through `course/pa5/` preserve commits, hashes and raw receipts.
The fixed manifest remains SHA-256
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
193 entry/final passes, zero lost identities, no fixed failures. Added cases and
probes do not redefine fixed-manifest progress or prove the remaining owners.

| Command | Result and artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` | PASS, 193/193; `stage.log` |
| required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS, 193/193, exit 0; `required-stage.log` |
| `make course-pa5` | PASS, 223/223, zero failed comparisons/no marker; `course.log`, archived course receipt |
| protected `clone_check.py prior pa5` | PASS; PA1–PA4 fixed 58/58, 26/26, 20/20, 106/106 and full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived receipts |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests and source audit; `quality.log` |
| `make audit-rust` | PASS, 82 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS, 193/193 preserved; `preflight.log`, `preserve.log`, independent `progress.json` |

`validate.py` and `checks.json` record exact commands, statuses and elapsed costs.
`probe.py`, `parameter-sources.json`, `operands-final.json`, `operands-final.log`
and raw `operands-final/` compare **704/704** exit/stdout/stderr/AST results.
They cover the previous 20 saturation cases, ordinary/nested/qualified names,
arrays/function/pointer/vector type syntax, functional/C-style casts, traits,
operand postfixes, angle delimiters, all four operators and malformed forms.
The initial broader sweep independently exposed 155 failures (153 postfix
acceptances and two diagnostics); its raw `operands.json`/`operands/` remain.
`new-regressions.json` retains all five strict comparisons. Copied prior probes
retain **806/806** additional full comparisons: 334 parameter-base, 147 attribute,
114 earlier audit, 135 lambda and 76 template cases in `prior-probes/`.

Resource command: `python3` on this root's `measure.py`. Metadata freezes tool
and source/output hashes, Linux/rustc 1.98.0, UTC/C locale and warm whole
invocations. Oracle A/A calibration then A/B/B/A blocks match all 54 complete
byte comparisons. An initial run overlapped gate execution and showed timing
variance; `resources-concurrent/`, `measurements-concurrent.json` and
`measure-concurrent.log` retain it. The table reports the repeat after the
validation process was confirmed terminal, with raw final observations under
`resources/` and `measurements.json`. Both runs passed byte comparison. These
include source I/O, preprocessing and clock query, and are observations on a
shared host, not cold-cache/hardware-counter or PA35 optimization proof.

| Workload | Functions | Rust mean ms | Rust max RSS KiB | C++ mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| named-trait-functions | 1000 | 46.25 | 8012 | 23.63 | 7884 |
| named-trait-functions | 4000 | 137.09 | 14200 | 69.92 | 15660 |
| named-trait-functions | 16000 | 453.22 | 49380 | 235.43 | 47336 |
| named-cast-functions | 1000 | 39.93 | 8024 | 15.97 | 7208 |
| named-cast-functions | 4000 | 102.01 | 12764 | 50.47 | 13620 |
| named-cast-functions | 16000 | 345.49 | 40228 | 194.96 | 40308 |
| ordinary-functions | 1000 | 25.01 | 8024 | 10.91 | 6480 |
| ordinary-functions | 4000 | 53.56 | 8532 | 24.57 | 8628 |
| ordinary-functions | 16000 | 163.85 | 24036 | 86.80 | 20600 |

Fourfold 4000 -> 16000 input growth takes about 3.1–3.4 times Rust wall time in
the final observations; syntax storage grows with introduced syntax. Checkout
and scratch occupy about 8.1 GiB of the shared 100 GiB budget.

The next dependency remains specialization category aggregation and dependent/
approximate prefix facts (`name_scopes.cpp`, `parser_names.cpp`), followed by
retained alternate statements/shared body ownership (`name_conflicts.h`,
`parser_statements.cpp::read_statement`). Rerun `pending-probe.py`: exact IDs
`specialization-namespace-type` (0 versus 1), `dependent-conflicting-category`
(both 0, differing AST), and `deep-parenthesized-expression` (0 versus 86)
still fail; raw `pending-probes.json` / `pending-probes/` prove those boundaries.
The two earlier capacity failures are resolved. Remaining source facts, class
search memo scaling and the safe host byte-stack policy still require owner
work. Hosted-only and later driver/backend responsibilities keep their original
stage boundaries. This entry does not certify full PA5 or complete Ralph's goal.

## Full-stage audit continuation: parameter-base lookup — 2026-10-12

Audit entry commit: `60515ce4ddfa053abc38a50075e4479825f99171` (clean).
Reviewed this entry against the stage base above and frozen oracle
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`. The implementation/audit commit
containing this entry is recorded in external `pa5-full-audit/handoff.json`.
The preceding goal turn is **progress**, proven by its committed lambda group
and archived 193/193 receipts. This continuation fixes an unfinished parser
responsibility beyond those fixtures. **Full PA5 acceptance remains unproven**;
the last accepted partial-audit identity above and Ralph stage are unchanged.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the full clone plan, full PA5
README/parsing guide, shared grammar, source map and historical ledger. Consulted
the actual read-only C++ name/scope, specifier, declaration, type/declarator,
statement/conflict and parser-region owners. No oracle or harness defect was
found. PA33 release-wide ownership/debug obligations and PA34 bootstrap are
future-stage gates; PA35 optimization targets are excluded.

### Findings resolved and responsibility review

| C++ owner | Finding and Rust owner |
| --- | --- |
| `parser_names.cpp::record_parameter_base/parameter_base_lookup/argument_class` | `parameter_bases.rs`, `classes.rs`, `scopes.rs`: type-parameter bases had no substitution lookup. `P<C>::word * p;` dumped multiplication even when C declares word as a type. Primary heads now record typed parameter positions and matching pack flags; concrete template-id prefixes search retained type arguments after ordinary members/bases. Packs search arguments in index order, duplicate bases are suppressed, lone-class argument restrictions and the oracle's depth > 8 approximation boundary are preserved. |
| `parser_names.cpp::typedef_record/specialization_record`, `parser_declarators.cpp::parse_type_id`, `parser_declarations.cpp::parse_using/declare_declarator` | `parameter_bases.rs`, `declarations.rs`, `templates.rs`, `parser.rs`, `ast.rs`: aliases and typedef chains discarded specialization arguments. Type-ids and plain typedef bindings now retain one-byte record identities pointing to immutable typed argument spans, with the frozen 255-record limit. Pointer/array/function/nested declarators do not gain class identity; alias templates omit the ordinary alias record. |
| `parser_names.cpp::advance_prefix/resolve_component` | `names.rs`, `scopes.rs`: known class prefixes now carry concrete argument handles or alias records to missing-member lookup, and pending forward classes remain dependent. Prefix queries after a template-id retain the class hidden by a value without changing the earlier ordinary template-id decision. |
| `parser_specifiers.cpp::parse_base_clause` | `classes.rs`: Rust incorrectly accepted `typename` at the start of a base name. It now rejects at that token with the frozen diagnostic bytes. |
| `parser_specifiers.cpp::parse_named_type_specifier` | `declarations.rs`: declining a known value as a type incorrectly recorded a grammar failure after the name. The optional reading now declines without updating the furthest diagnostic, fixing hidden-template and ordinary known-value rejection locations. |
| `parser_regions.h`, `parser_state.cpp` transactions and deferred changes | `parser.rs`, `scopes.rs`: template-head ownership, specialization record lengths and parameter-base changes participate in nested rewind. Deferred body detach/reattach reverses only the affected scope facts, preserving existing token and body lifetimes. |

The reviewed lookup uses compact ScopeId/NameId indices and scope-owned position
records; it does not scan a translation unit's templates or copy visible frames.
The active template head is saved/restored around nested definitions. Arguments
remain TypeId/ExprId facts in the existing syntax owner, not reparsed text.
Argument-class lookup reads structured name components through parser-category
indices, as the frozen syntactic approximation does; semantic instantiation is
not demanded. Type-id records borrow already parsed immutable argument spans.
Syntax/scope rollback removes their relationships before discarded IDs can escape;
the record table rewinds with its syntax lists. The limited recursive substitution
walk is separate from grammar nesting and never exceeds the frozen nine levels.

Unqualified lookup in a primary template does not substitute its parameter base.
Ordinary members and other bases precede this fallback; parameter positions and
pack argument order control its tie breaks. Source lists continue to control all
dump bytes. Literal values remain the PA2 typed numeric facts; no evaluation,
target arithmetic, optimizer, allocator or backend policy was changed. Production
execution remains entirely in Rust; oracle executions in the receipts only
compare independent outputs. No unsafe code, dependency, environment read,
fixture tailoring, blanket allowance or mechanical source split was introduced.

Two added invariants verify committed-inner/abandoned-outer rollback of heads,
bindings, argument records and scopes; source-position detach/reattach of base
facts; unqualified lookup isolation; alias sharing; matching pack metadata; and
the exact 255-record capacity. Debug and release pass **81 library plus eight
tool tests** each. Four required regressions and only their new expectation
sidecars live in the executed `tests/general/300-parameter-base-*.t` lane.
No existing fixture/reference, manifest, protected checker/policy, oracle source
or Ralph state was modified.

### Validation and resources

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-full-audit/`.
`audit-metadata.json`, `start-results.json`, `pa1-results.json` through
`pa5-results.json`, `progress.json`, exact passing/failing ID lists, archived
`cases/` and `course/pa1/` through `course/pa5/` retain review identities,
manifest/tool hashes and raw traces. The fixed manifest SHA-256 remains
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`.
The same 193 entry passes are retained, with zero losses; fixed failing IDs: none.

| Command | Result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` | PASS, 193/193; `stage.log` |
| required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS, 193/193, exit 0; `required-stage.log`, `pa5-results.json` |
| `make course-pa5` | PASS, 218/218, zero failed comparisons/no failure marker; `course.log`, archived `course/pa5/test/course-result.json` |
| protected `clone_check.py prior pa5` | PASS; fixed PA1–PA4 58/58, 26/26, 20/20, 106/106; full courses 58/58, 28/28, 20/20, 112/112; `prior.log`, archived stage/course receipts |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests, source audit; `quality.log` |
| `make audit-rust` | PASS, 82 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` / `preserve pa5` | PASS, preservation 193/193; `preflight.log`, `preserve.log`, independent `progress.json` |

`validate.py` / `checks.json` retain exact command/status/elapsed observations.
`probe.py` / `parameters-final.json` retain **334/334** full status/stdout/stderr/
AST comparisons, including scalar/pack positions, argument restrictions,
ordinary-base priority, aliases, nested substitution, depth boundaries, forward
completion, lambdas and rejecting clauses. Entry probes independently exposed
eleven mismatches among the first 62 cases. The broader sweep exposed 36
additional base-typename failures before that correction. `new-regressions.json`
retains full bytes for all four new cases. Copied prior probe scripts retain
**147/147 attributes, 114/114 earlier audit, 135/135 lambdas and 76/76 templates**
comparisons in `prior-probes/`. Added passing programs do not count toward fixed
manifest progress or prove ownership of the pending groups below.

`measure.py` / `resource-metadata.json` / `measurements.json` retain frozen tool,
source/output hashes, rustc 1.98.0, Linux, UTC/C locale and warm whole-invocation
conditions. Oracle A/A calibration plus A/B/B/A blocks match full bytes on all
54 invocations. Resource costs include reading, preprocessing, clock query and
dump I/O on a shared host; no cold-cache, hardware-counter or PA35 gate is claimed.

| Workload | Units | Rust mean ms | Rust max RSS KiB | C++ mean ms | C++ max RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| parameter-base-functions | 1000 | 57.32 | 8044 | 20.77 | 7216 |
| parameter-base-functions | 4000 | 128.93 | 11380 | 64.60 | 13552 |
| parameter-base-functions | 16000 | 417.23 | 33920 | 210.59 | 41012 |
| pack-base-functions | 1000 | 40.83 | 8020 | 18.50 | 7160 |
| pack-base-functions | 4000 | 124.22 | 12044 | 55.96 | 13564 |
| pack-base-functions | 16000 | 397.96 | 38136 | 216.76 | 39604 |
| alias-base-functions | 1000 | 34.10 | 8012 | 14.30 | 6524 |
| alias-base-functions | 4000 | 74.88 | 10676 | 38.55 | 10744 |
| alias-base-functions | 16000 | 246.85 | 32504 | 121.80 | 27636 |

Fourfold 4000 -> 16000 growth takes about 3.2–3.3 times Rust wall time in these
observations; storage grows with retained syntax. Alias inputs exercise record
saturation too. Checkout/scratch use about 7.6 GiB of the shared 100 GiB budget.

### Remaining acceptance evidence and next boundary

The green fixed/course checks do not certify full stage ownership. Fresh raw
counterexamples in `pending-probe.py`, `pending-probes.json` and `pending-probes/`
prove remaining due parser gaps with exact IDs:

- `specialization-namespace-type`: primary value versus specialization type;
  oracle accepts namespace `P<int>::word *p;`, Rust rejects (0 versus 1).
- `dependent-conflicting-category`: dependent `P<T>::word *p;` is an expression
  under conflicting member categories; both accept but Rust dumps a declaration.
- `deep-parenthesized-expression`: 300 parentheses; oracle accepts and Rust
  reports the unported nesting boundary (0 versus 86).
- `capacity-probes/14` and `capacity-probes/15`: 255/256 repeated nested
  `sizeof(P<P<C>>(1))` operands followed by an inherited alias member statement;
  both accept, but alias record saturation changes Rust's final declaration to
  multiplication. The independent 20-case `capacity-probes/parameters.json`
  sweep passes 18/20. Trait alternatives currently parse names again where C++
  shares a name outside its tentative type reading, changing record consumption.
  `parser_expressions.cpp::parse_sizeof_operand/parse_type_trait` and the current
  `expressions.rs` owner need that parse-once correction. Preserving abandoned
  record slots alone was probed and rejected because it increased mismatches;
  safe transactional record rollback is retained until the grammar owner is
  translated. This is an open parity finding, not a claimed exact capacity audit.

The next coherent dependency is `name_scopes.cpp` category aggregation and
`parser_names.cpp` dependent/approximate prefix facts, followed by
`name_conflicts.h` / `parser_statements.cpp::read_statement` retained alternate
readings and shared body ownership. C++ `AmbiguousStmt` retains several typed
readings even where PA5 prints only the first; the Rust statement model still
cannot represent that fact. Remaining source/specifier/parameter/initializer
metadata and the safe replacement for the frozen byte-stack budget need review
and implementation before full acceptance. Class search memo scaling is also
unfinished. Hosted-only syntax/prelude/statistics and later driver modes retain
their future owning-stage boundaries; no future semantic or backend stage is
pulled into PA5 to make the current receipts green. The full-stage goal remains
active and neither the assignment nor Ralph phase is marked complete.

## Lambda syntax and commit-time body demand checkpoint — 2026-10-12

Entry commit: `39528505e3b0f5955e215ecdbb5b35030e1479e4` (clean).
Implementation commit: the commit containing this entry; the external
`artifacts/pa5-lambdas/handoff.json` records its full ID. Stage base and
last-audited commit above remain unchanged. The preceding goal turn is
**progress**, established by the committed attribute group and 190/193 receipts.
This implements the next coherent group: all three remaining fixed failures
share `parser_expressions.cpp::parse_lambda` and stopped at status 86.
This is a partial production-owner handoff with green PA5 stage/course lanes;
Ralph owns checkpoint/final audits and advancement.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the full clone plan, PA5 README and
parsing guide, and this ledger. Consulted the actual read-only C++ implementation
at `/home/vishvananda/work/references/v5rust-cpp`; its HEAD remains the contract
commit `905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`. No oracle/harness defect was found.

| C++ owner/responsibility | Rust owner and completed group |
| --- | --- |
| `parser_expressions.cpp::parse_lambda`, `ast_expressions.h::LambdaExpr/LambdaCapture` | `lambdas.rs`, `ast.rs`: capture defaults, this/copy/reference captures and packs; optional parameter clauses, mutable, both exception forms, before/after-specifier attributes, trailing types and original clause/body token ordinals |
| `parser.cpp::parse_deferred_lambdas`, `parser_state.cpp::save/restore/commit/close` | `lambdas.rs`, `parser.rs`: tentative lambdas skip balanced bodies; outermost successful speculation demands each queued body once while its token window and scope log remain live; abandoned attempts discard queue targets and operands; body parse errors remain terminal |
| `name_scopes.cpp::detach/reattach` | `scopes.rs`, `lambdas.rs`: reversible change descriptors detach only later binding/relationship/completion changes; reattach in source order around body demand; newly allocated scope identities remain valid; the whole environment is never copied |
| `parser_regions.h::SpeculationBoundary`, `parser_specifiers.cpp` class-body boundary | `lambdas.rs`, `classes.rs`, `angles.rs`: speculation depth is distinct from retained transaction depth; class regions and committed lambda bodies isolate speculative lists, with nested bodies parsed through their own statement decisions |
| `parser_statements.cpp::statement_starts_declaration/parse_statement_reading` | `statements.rs`, `parser.rs`: typed declaration/expression head prediction; unambiguous declarations commit directly, while functional-cast/declaration ambiguity gets bounded transactional lookahead; read-only probes always restore names, syntax and cursor without inventing a failure diagnostic |
| `parser_state.cpp::skip_balanced`, `parser_declarators.cpp::parse_parameter_clause/parse_throw_types` | `parser.rs`, `declarators.rs`: one reusable iterative closer stack for skipped regions; existing typed parameter/default/throw-type parser is shared by lambdas |
| `ast_printer.cpp::lambda`, `ast_render.cpp::lambda/lambda_introducer` | `printer.rs`, `render.rs`: borrowed iterative exact introducer/declarator/body views, lambda-specific noexcept wrapper, throw spelling and payload-free trailing-return wrapper; recognized attributes stay typed but are omitted like C++ |

All lambda syntax lives in the existing translation-unit slabs and immutable
list spans. Expressions refer to compact LambdaId; capture kinds carry interned
NameId; declarators, exception operands, trailing types and attributes retain
ExprId/TypeId facts. Bodies explicitly transition from Deferred to Parsed(StmtId).
Original token ordinals are checked into u32, preserving future closure identity
without deriving it from rendered names. No target evaluation, template
instantiation, lowering or backend work is demanded by AST mode.

A deferred entry borrows the retained cursor region and records its lexical
ScopeId plus change-log position. After commit, later changes detach in reverse
and reattach forward between source-ordered bodies. Body-local changes remain
logged for any enclosing rollback. Speculation boundaries preserve the enclosing
transaction's retention lifetime while allowing inner statement attempts to
commit independently. The scratch change vector and delimiter stack retain
capacity between queries. Completed lambda bodies keep syntax, not token runs.
The existing class capture/replay owner and absolute token ordinals continue to
work for local classes and late members inside lambdas.

The first broader probe run found a diagnostic divergence for an unbalanced
body: the candidate unnecessarily speculated over an unambiguous auto declaration
and skipped ahead to a mismatched closer. Porting the C++ statement-head decision
restored the oracle's earlier direct-body diagnostic. The same owner also avoids
retaining complete ordinary declaration initializers for unnecessary speculation.
Balanced skipping is now iterative like C++, independent of the parser's remaining
host recursion policy. No unsafe, FFI/subprocess implementation, dependencies,
fixture recognition, optimization or new lint suppression was introduced. Code
generation, optimizer and register allocator sources are unchanged.

Five new Rust invariants cover typed capture/default/attribute/operand facts and
source ordinals; abandoned lambda attempts with no body demand or leaked scratch;
source-position bindings across two queued bodies and later shadowing; terminal
committed-body errors; and skipping 5000 nested delimiters without grammar demand
or recursion growth. Debug and release each pass **79 library and eight tool tests**.
Three new required cases and their own expectation sidecars are in the executed
PA5 `tests/general/300-lambda-*.t` lane, covering attributes/exceptions/traits,
ambiguity/nested bodies and a rejecting committed body. No existing fixture or
reference, frozen manifest, protected checker/policy, oracle source or Ralph state
changed. New passing cases do not contribute to fixed-manifest progress.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-lambdas/`.
`entry.json`, `start-results.json`, final `pa1-results.json` through
`pa5-results.json`, `progress.json`, passing/gained/failing identity lists and
`failure-index.json` retain exact identities, manifest/tool hashes and validation.
`start-failure-index.json` / `start-cases/<digest>/` preserve entry raw failure
traces/diagnostics/AST differences; final `cases/<digest>/trace.jsonl` and
`course/pa1/` through `course/pa5/` retain protected receipts. Raw comparisons
for broader probes, new regressions and resources remain in their own directories.
Fixed coverage stays **193**, with manifest SHA-256
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`:
**190 -> 193 passes; 3 -> 0 failures; three gains; zero lost identities**.

| Command | Final result and receipt |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` | PASS: 193/193; `stage.log`, `pa5-results.json` |
| required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | PASS: 193/193, checker exit 0; `required-stage.log` |
| `make course-pa5` | PASS: 214/214, zero failed comparisons and no failure marker; `course.log`, `course/pa5/test/counts.txt` |
| protected `clone_check.py prior pa5` | PASS: fixed PA1–PA4 58/58, 26/26, 20/20, 106/106; full course 58/58, 28/28, 20/20, 112/112; `prior.log` and archived stage/course receipts |
| `make lint` / protected `clone_quality.py` | PASS: rustfmt, strict Clippy, debug/release tests and companion audit; `quality.log` |
| `make audit-rust` | PASS: 81 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS; `preserve.log`; `progress.json` independently proves all 190 entry passes are retained |

`validate.py` / `checks.json` record exact commands, statuses and elapsed costs.
`probe.py`, `probes.json`, `probes-final.log` and `probes/` retain all **135/135**
full exit/stdout/stderr/AST comparisons: capture combinations, parameter packs,
mutable/noexcept/throw, attributes, nested lambdas in operands/defaults/traits,
angle regions, direct and speculative statements, source-position shadowing,
late class members, malformed clauses and bodies. The isolated preceding
attribute/template/audit probe scripts rerun **147/147**, **76/76** and **114/114**
full comparisons, with their raw sources/receipts and `prior-*.log` files retained.
All three new executed cases also match full bytes; `new-regressions.json` and
`new-regressions/` preserve evidence. These comparisons prove the translated
group's tested behavior, not every pending C++ implementation responsibility.

Resource command: `python3` on this root's `measure.py`. Frozen input/output/tool
hashes, rustc 1.98.0, Linux, UTC/C locale and warm whole-invocation conditions are
recorded in `resource-metadata.json`, `measurements.json` and `resources/`.
Oracle A/A calibration followed by A/B/B/A blocks match full status/stdout/stderr/
AST bytes on all 54 invocations. Measurements include reading, preprocessing,
the host clock helper and dump I/O on a shared host; no cold-cache or hardware-
counter proof, or PA35 generated-code improvement requirement, is claimed.

| Workload | Units | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| lambda-functions | 1000 | 41.22 | 8044 | 19.46 | 7220 |
| lambda-functions | 4000 | 112.03 | 14168 | 61.57 | 14260 |
| lambda-functions | 16000 | 419.95 | 46844 | 225.22 | 42072 |
| speculative-lambda-declarations | 1000 | 39.92 | 8012 | 18.07 | 7176 |
| speculative-lambda-declarations | 4000 | 125.94 | 12056 | 63.10 | 13580 |
| speculative-lambda-declarations | 16000 | 431.18 | 42356 | 217.18 | 39132 |
| ordinary-functions | 1000 | 26.22 | 8060 | 10.82 | 6524 |
| ordinary-functions | 4000 | 65.20 | 8560 | 25.89 | 8628 |
| ordinary-functions | 16000 | 193.33 | 21892 | 86.90 | 20596 |

Lambda functions exercise captures, mutable/noexcept and calls; speculative
lambda declarations exercise competing function/object readings and body-local
typedefs. Ordinary functions provide a control. Fourfold 4000 -> 16000 input
growth takes about 3.0–3.8 times Rust wall time in these observations, with syntax
storage growing with introduced syntax. Checkout/scratch occupy about 7.4 GiB
of the shared 100 GiB capacity (the filesystem's inaccessible lost+found is
excluded). These are measured supported costs, not new acceptance policy.

Exact newly passing fixed IDs:

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-capture-forms.t
tests/general/200-lambda-declarator.t
```

Exact remaining fixed IDs: **none**. PA5 fixed and course lanes are green.
The next production dependency boundary is retained specialization-category
conflicts and alternate statement readings in `name_conflicts.h`,
`name_scopes.cpp`, `parser_names.cpp` and `parser_statements.cpp`, including
sharing parsed lambda bodies across those retained readings. Dependent
prefix/substitution approximations, remaining parameter/name/initializer source
facts, hosted vocabulary/prelude/stats and the safe replacement for the oracle's
host byte-stack policy still require owner audits/translation. Hosted C++20
lambda template heads depend on that typed hosted grammar boundary. The current
real grammar nesting boundary still explicitly returns status 86 at 256;
iterative balanced skipping no longer consumes that budget. These are unfinished
production responsibilities, not certified clone rejections.

Architecture review should trace lambda body demand at outer commit, visibility
while later changes detach, preservation of enclosing rollback, nested class/run
lifetimes, typed source ordinals and iterative views. It should also assess the
remaining conflict/hosted/source-fact/stack owners against the C++ implementation,
rather than infer whole-PA5 completion solely from 193 passing cases. This entry
does not advance last-audited commit or Ralph's stage.

## Attribute collection, appertainment and transactional owners checkpoint — 2026-10-12

Entry commit: `a220356` (clean; full ID in `artifacts/pa5-attributes/entry.json`).
Implementation commit: the commit containing this entry; the external
`artifacts/pa5-attributes/handoff.json` records its full ID. Stage base and
last-audited commit above are unchanged. The preceding turn is **progress**:
its committed template group and authoritative receipts established 182/193.
This entry implements the next coherent group, reducing its eleven failures to
three. This is a partial assignment handoff; Ralph owns audits and advancement.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the full clone plan, PA5 README and
parsing guide, and this ledger. Consulted the actual read-only C++ source at
`/home/vishvananda/work/references/v5rust-cpp`; its HEAD matches contract commit
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`. No oracle/harness defect was found.

| C++ owner/responsibility | Rust owner and completed behavior |
| --- | --- |
| `parser_state.cpp::at_attribute/skip_attributes` | `attributes.rs`, shared `classes.rs::skip_balanced`: detect standard `[[...]]`, alignas and both GNU spellings; preserve the oracle's balanced skip grammar at pointer/base/trailing-parameter/abstract-function positions |
| `parser_specifiers.cpp::collect_attribute_specifiers/gnu_attribute/parse_alignment_operand` | `attributes.rs`, `ast.rs`: full recognized attribute vocabulary, balanced unknown groups, standard GNU namespaces/reserved spellings, noreturn's no-operand rule, typed alignment type/expression alternatives, string section/ABI-tag operands and continued tag grouping |
| `parser_specifiers.cpp::vector_specifiers/leading_vector_specifiers`, class/enum attribute ownership | `declarations.rs`, `classes.rs`, `attributes.rs`: vector specifiers and alias type extension; class-key/post-body GNU attributes, enum filtering, elaborated-class tag precedence and standalone declaration selection |
| `parser_declarations.cpp::parse_declaration/try_special_member/parse_namespace/parse_using/parse_simple_or_function` | `declarations.rs`, `classes.rs`: declaration-local leading/pending spans, special-member attributes, namespace/alias/using retention and explicit function-body environment isolation |
| `parser_declarators.cpp::parse_ptr_operator/parse_declarator_into/parse_function_qualifiers/parse_parameter` | `declarators.rs`, `attributes.rs`: pointer skipping among cv qualifiers, entity/type suffix appertainment, typed GNU asm-label literals, named versus abstract qualifier attributes and parameter environment isolation |
| `parser_statements.cpp::parse_statement/parse_condition/parse_for` | `statements.rs`: pass collected statement/for-init attributes to declarations; drop leading condition/range attributes like C++; nested declarations own their spans |
| `ast_types.h`, `ast_declarations.h`, `parser_regions.h` | `ast.rs`, `parser.rs`, `storage.rs`: located attributes with AttributeKind/Form/Site enums, ExprId/TypeId operands and compact immutable spans; complete rollback of storage, scratch and attribute environment handles |
| `ast_printer.cpp`, `ast_render.cpp` | `printer.rs`, `render.rs`: exact vector-attribute node and canonical compact vector spelling; ordinary attributes are omitted from these PA5 views as in C++ |

Attributes remain in the existing unit-owned flat syntax stores. Alignment
name operands are parsed once and shared by their tentative type and expression
readings; no rendered spelling is a semantic key or phase transport. Recognized
operands use the shared expression/type parser, with delimiter regions restoring
angle context. Unknown attribute tokens are consumed through bounded demand and
balanced skipping, without retaining a parallel source tree. Source traversal
and attribute argument order determine all views.

Nested owners save compact list handles and scratch boundaries. Parser attempts
rewind attribute operands, list data/scratch, leading/pending/class-tag handles,
names/scopes and existing deferred state together. Elaborated-class tags use a
separate immutable span until the declaration commits; they never splice into
an enclosing scratch prefix that an attempt would have to restore. Function
bodies and parameters isolate their declaration attribute environment. Nothing
adds semantic analysis, instantiation, optimizer, register allocator or native
emission demand to AST mode. No unsafe, dependencies, implementation fallback,
fixture recognition, new optimization or blanket lint suppression was added.
Two field-local non-test dead-code expectations retain class/declarator facts
for semantic consumers; invariant tests consume those fields directly.

Three new Rust invariants verify typed operands/owner order/appertainment and
function-body isolation; nested committed-then-abandoned declarations restoring
all attribute storage and bindings; and abandoned elaborated-class tags
preserving enclosing scratch. Debug and release each pass 74 library and eight
tool tests. Three new `.t` regressions and their own oracle expectation sidecars
live in the executed PA5 `tests/general/300-attribute-*.t` lane. No existing
fixture/reference, protected manifest/checker/policy, oracle source or Ralph
state changed.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-attributes/`.
`entry.json`, `start-results.json`, final `pa1-results.json` through
`pa5-results.json`, `progress.json`, passing/gained/failing case lists and
`failure-index.json` retain identities and hashes. Archived `cases/<digest>/`
contain protected traces and raw stdout/stderr/AST bytes; `course/pa1/` through
`course/pa5/` retain full course receipts. The fixed manifest SHA-256 remains
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`.
Coverage stays 193: **182 -> 190 passes; 11 -> 3 failures; eight gains; zero
losses**. Added passing regressions do not count toward fixed-manifest progress.

| Command | Result and receipt |
| --- | --- |
| `make build` | PASS; `build-final.log` |
| `make test-pa5` | FAIL: 190/193, make exit 2; `stage.log`, `pa5-results.json` |
| required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | FAIL: 190/193, checker exit 1; `required-stage.log`; all three remaining cases stop at the lambda implementation boundary |
| `make course-pa5` | FAIL: 208/211, including all three new regressions; three failed comparisons; `course.log`, `course/pa5/test/counts.txt`, raw trace and retained failure marker |
| protected `clone_check.py prior pa5` | PASS: fixed PA1–PA4 58/58, 26/26, 20/20, 106/106; full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived stage/course receipts |
| `make lint` / protected `clone_quality.py` | PASS: rustfmt, strict Clippy, debug/release tests and source audit; `quality-final.log` |
| `make audit-rust` | PASS: 80 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS: 182/182 accepted identities; `preserve.log`; `progress.json` independently proves unchanged coverage and zero losses |

`validate.py` records exact commands, exit statuses and elapsed seconds in
`checks.json`. `probe.py` compares full exit/stdout/stderr/AST on 147 generated
programs; all match. These cover every recognized attribute form, valid and
malformed typed operands, placement at declaration/specifier/class/parameter/
function sites, pointer skipping, class bases, aliases/vectors, GNU asm labels,
namespace tags, conditions/loops/catches and unknown balanced clauses. Raw
sources/bytes live in `probes/`, with `probes.json` and `probes-final.log`.
`prior-template/probe.py` and `audit-probe.py` rerun the preceding 76 and 114
programs; all match, including the formerly pending attribute probe. Their
sources/raw receipts and `prior-template.log` / `prior-audit.log` are archived
separately. All three new executed fixtures also match full exit/stdout/stderr/AST
bytes; `new-regressions.json` and `new-regressions/` retain these comparisons.
These comparisons support the translated group, not completion of
all remaining production owners.

Resource command: `python3` on this artifact root's `measure.py`. Frozen input
and final tool hashes, rustc 1.98.0, Linux, UTC/C locale and warm whole-invocation
conditions are recorded in `resource-metadata.json`, `measurements.json` and
`resources/`. Oracle A/A calibration then A/B/B/A blocks match full status,
stdout/stderr and AST bytes on all 54 invocations. Observations include source
reading, preprocessing, the host clock helper and dump I/O on a shared host;
no cold-cache, hardware-counter or PA35 performance gate is claimed.

| Workload | Units | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| attribute-declarations | 1000 | 32.00 | 8020 | 11.34 | 5812 |
| attribute-declarations | 4000 | 74.91 | 8036 | 29.18 | 8568 |
| attribute-declarations | 16000 | 212.89 | 19812 | 109.67 | 18420 |
| attributed-classes | 1000 | 30.96 | 8012 | 12.91 | 6540 |
| attributed-classes | 4000 | 90.65 | 8532 | 38.63 | 9972 |
| attributed-classes | 16000 | 296.26 | 24732 | 158.78 | 25948 |
| ordinary-functions | 1000 | 24.73 | 8004 | 10.00 | 6476 |
| ordinary-functions | 4000 | 62.06 | 8004 | 23.56 | 8628 |
| ordinary-functions | 16000 | 170.85 | 21924 | 83.55 | 20620 |

Attribute declarations combine typed alignas, specifier vectors and array
alignment; attributed classes combine class tags, function/parameter attributes
and noexcept. Ordinary functions are the control. Fourfold 4000 -> 16000 input
growth takes about 2.8–3.3 times Rust wall time here, with syntax storage growing
with introduced syntax. Checkout/scratch occupy about 7.0 GiB of the shared
100 GiB capacity. These are measured costs, not a new code-generation target.

Exact newly passing fixed IDs (all eight attribute-owner failures):

```text
tests/general/100-balanced-attributes.t
tests/general/100-class-alignas-after-class-key.t
tests/general/200-attributed-partial-specialization-current-class-constructor.t
tests/general/200-attributed-virtual-destructor-member.t
tests/general/200-forward-unknown-nested-template-in-ctor-body.t
tests/general/200-template-qualified-inline-attribute-constructor-definition.t
tests/general/200-trailing-parameter-carries-dependency-attribute.t
tests/general/200-trailing-parameter-vendor-attribute.t
```

Next dependency boundary: lambda capture/declarator/body syntax and deferred
speculation in `parser_expressions.cpp`, `parser.cpp` and `parser_regions.h`.
The exact remaining fixed IDs are:

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-capture-forms.t
tests/general/200-lambda-declarator.t
```

Those three cases still return status 86, not a valid matching C++ rejection.
The current full PA5 stage and course lanes remain failing. Remaining production
owners beyond these cases are the prior checkpoint's specialization-category
conflict aggregation, dependent prefix/substitution approximations, retained
ambiguous readings, remaining source facts, hosted vocabulary/prelude/stats and
host byte-stack policy (the safe nesting boundary still returns 86 at 256).
No PA5 completion or last-audited-commit advancement is claimed. Review should
trace attribute operand reuse, declaration/body/parameter isolation, class-tag
ordering at commit, complete scratch rollback and bounded token demand, rather
than infer ownership from passing fixture counts.

## Template syntax, parameter environments and angle regions checkpoint — 2026-10-12

Entry commit: `6704b7ae65e7a3f6ca7c8c1535f10ed2c33be80b` (clean). Implementation commit: the commit
containing this entry; `artifacts/pa5-templates/handoff.json` records its full ID.
Stage base and last-audited commit above are unchanged. The preceding goal turn
is **progress**, established by the accepted implementation/audit commits and
raw receipts. The supplied stageProgress result repeats the audited 114/193
baseline; entry inspection found no live compiler/check process to resume.
This is a coherent partial PA5 handoff. Ralph owns audits and advancement.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, the clone plan, PA5 README/parsing
guide and the current ledger. Consulted the actual frozen C++ owners at
`/home/vishvananda/work/references/v5rust-cpp`, contract commit
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`, including the responsibility paths
below and their AST/state headers. No oracle/harness defect was found.

| C++ responsibility | Rust owner and completed group |
| --- | --- |
| `parser_declarations.cpp::parse_template_declaration/parse_template_parameter_clause/parse_template_parameter/parse_explicit_instantiation` | `templates.rs`, `declarations.rs`, `ast.rs`: located structured heads and type/non-type/template-template parameters, separately scoped nested heads, packs/defaults, empty explicit-specialization heads and extern versus definition instantiation syntax |
| `parser_declarations.cpp::declare_declarator/parse_using`, `parser_specifiers.cpp` class publication | `parser.rs`, `declarations.rs`, `classes.rs`, `scopes.rs`: type-template/function-template categories, parameter replacement, aliases, injected names, declarations published through enclosing template frames, primary/specialization member reachability and ordinary-function/template redeclaration merge |
| `parser_names.cpp::parse_name_component/parse_template_arguments/parse_template_argument/is_type` | `names.rs`, `templates.rs`, `ast.rs`: identifier/operator/literal/destructor template ids, explicit template/typename, typed argument type/expression/pack alternatives, dependent prefixes and parameter mentions; each argument name is parsed once and shared by its type/expression alternatives |
| `parser_names.cpp::template_id_follows/member_template_arguments_plausible`, `name_scopes.cpp::declare` | `names.rs`, `angles.rs`, `scopes.rs`: member calls use marks from class/member templates or type templates, not unrelated namespace function templates; known values precede lexical hints; bare logical argument readings decline member-template speculation |
| `parser_state.cpp::closing_angle_ahead`, `parser_names.cpp` failed-list coordination, `parser_expressions.cpp` delimiter rules | `angles.rs`, `expressions.rs`, `declarators.rs`, `names.rs`: immutable delimiter fences, environment-specific failed-list queries and coordinated nested abandonment; linked greater halves close lists while parenthesized/call/subscript/braced/trait/decltype/conditional regions retain comparisons and shifts |
| `name_scopes.cpp::search_class_context/lookup/add_base_edge`, `parser_specifiers.cpp::parse_base_clause`, `parser_declarators.cpp` qualified clauses | `scopes.rs`, `classes.rs`, `declarators.rs`, `names.rs`, `declarations.rs`: definition-owned lexical frames and explicit member context overlays; class members precede enclosing definition frames without importing the primary's original parameters; dependent bases are skipped by unqualified lookup; nondependent edges remain searchable when the same target already has a dependent edge |
| `parser_expressions.cpp` pack nodes, `ast_printer.cpp`, `ast_render.cpp` | `expressions.rs`, `ast.rs`, `printer.rs`, `render.rs`: sizeof packs and expression expansions, exact template/argument/default rendering, lone unnamed int default token form, explicit-instantiation dump distinction and condition-declaration wrapper |

All new syntax belongs to the existing translation-unit slabs and immutable
list spans. Heads and parameters retain source offsets; argument alternatives
retain TypeId or ExprId, names retain located components and typed bindings,
and pack expansions reference their single pattern. The views borrow these
records and never feed text back to another phase. Templates retain parsed
bodies; only ordinary class completion captures and replays deferred member
runs. No template instantiation, semantic evaluation, lowering or backend work
is demanded by AST mode. The code generator, optimizer and allocator are unchanged.

Template scopes form parent-linked frames. Qualified declaration contexts add
indexed ScopeId member relationships instead of changing lexical parenthood to
the primary's parameter scope. Undo covers member relationships, parents,
bindings, edges, completion, all new syntax/list storage, current/function/class
state, pending-template/angle/parameter-mention state and retained body runs.
An outer failure still unwinds committed inner work. Reopening a forward and
changing a member context receive fresh parser-environment identities; rollback
restores the old identity. Failed-list keys contain absolute token position,
ScopeId, environment identity and angle context. Conservative category marks
survive rollback as hints, never as successful bindings. The cache does not
invalidate or scan unrelated semantic work.

Delimiter fences use an ordered interval index for predecessor queries, not
semantic equality or emission order. A new immutable run costs logarithmic
index work; no whole fence vector is sorted after each statement. Nested list
failures are remembered together and restore the first abandoned diagnostic.
Delimiter regions reuse one open-list vector with saved boundaries. Lookup uses
indexed name/scope entries and explicit edges; source-list traversal controls
all output ordering. No source reparsing by later phases, owning node graph,
unsafe/FFI/subprocess implementation, dependency or blanket lint suppression
was introduced. Two narrow source-location expectations retain facts for later
semantic diagnostics. The companion audit finds zero issues in 79 files.

Three new Rust invariants verify complete head/argument/environment/deferred-body
rollback, failed-angle cache separation after a changed binding environment,
and bounded speculation plus iterative output on a 4096-operand undeclared
relational chain. Debug/release each pass 71 library and eight tool tests.
Four new regression cases and expectation sidecars live in the executed PA5
`tests/general/300-template-*.t` lane: category/member marks and friend scope,
definition parameter rebinding, dependent/mixed bases and specialization edges,
and angle regions/pack/explicit-instantiation syntax. Only these newly added
expectations were obtained from the frozen C++ executable. Frozen fixtures,
references, manifest, checker, oracle sources, policy and Ralph state were untouched.

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-templates/`.
`entry.json`, `start-results.json`, `pa1-results.json` through `pa5-results.json`,
`progress.json`, sorted passing/newly-passing/failure case lists and
`failure-index.json` retain exact identities and tool hashes. The fixed manifest
SHA-256 remains `27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`.
Coverage remains 193 cases: **114 -> 182 passes; 79 -> 11 failures; 68 gains;
zero losses**. New passing cases do not count toward that gain. Archived
`cases/<digest>/` retain protected traces and raw diagnostic/AST mismatches;
`course/pa1/` through `course/pa5/` retain course traces and comparison counts.

| Command | Result and evidence |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` / required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` | FAIL: 182/193, checker exit 1/make exit 2; `stage.log`, final direct `required-stage.log`, `pa5-results.json`; progress is the fixed failure reduction |
| `make course-pa5` | FAIL: 197/208 including all four new regressions and all eleven earlier audit regressions; 11 remaining failures, retained failure marker; `course.log`, `course/pa5/test/counts.txt`, `course-result.json` and raw trace |
| protected `clone_check.py prior pa5` | PASS: fixed PA1–PA4 58/58, 26/26, 20/20, 106/106; full course 58/58, 28/28, 20/20, 112/112; `prior.log` and archived stage/course receipts |
| `make lint` / protected `clone_quality.py` | PASS: rustfmt, strict Clippy, debug/release tests and source audit; `quality-final.log` |
| `make audit-rust` | PASS: 79 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS: 114/114 accepted identities; `preserve.log`; `progress.json` independently proves unchanged coverage and 68 gains/zero losses |

`probe.py` compares full exit/stdout/stderr/AST bytes on 76 generated programs.
All 75 supported programs match, including malformed heads/argument lists,
nested heads, aliases, packs, casts/traits, operator names, default arguments,
member/namespace template marks, primary/specialization reachability and changed
parameter categories in out-of-class definitions. The separate attribute
probe reports the known pending boundary (oracle 0/candidate 86), not a pass.
`probes.json`, `probes-final.log`, `probes/` retain every source and raw receipt.
`audit-probe.py` reruns all 114 programs from the preceding accepted audit;
all still match, including friend publication, class forwards, namespace import
order/depth, typed casts, initializer rejection and qualified member scopes.
`prior-probes.log` and `prior-probes/` retain these raw comparisons.

Resource command: `python3` on this artifact root's `measure.py`. Frozen input
and tool hashes, Linux/rustc 1.98.0, C locale and UTC are recorded in
`resource-metadata.json` and `measurements.json`. Oracle A/A calibration then
A/B/B/A blocks match full stdout/stderr/exit/AST bytes on all 54 invocations.
These are warm whole invocations including preprocessing, the clock helper and
dump I/O on the shared host; they are recorded observations, not a cold-cache,
hardware-counter or PA35 gate. Raw RSS/stdout/stderr/AST files and source/output
hashes live under `resources/`.

| Workload | Units | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| template-heads | 1000 | 44.48 | 8016 | 19.74 | 7240 |
| template-heads | 4000 | 134.38 | 14188 | 67.80 | 15676 |
| template-heads | 16000 | 497.56 | 49468 | 248.83 | 47664 |
| relational-fences | 1000 | 31.80 | 8020 | 12.37 | 6524 |
| relational-fences | 4000 | 81.31 | 12780 | 38.00 | 10684 |
| relational-fences | 16000 | 269.97 | 43012 | 130.90 | 27600 |
| ordinary-functions | 1000 | 23.60 | 8044 | 9.49 | 6460 |
| ordinary-functions | 4000 | 52.62 | 8080 | 23.73 | 8544 |
| ordinary-functions | 16000 | 165.90 | 21904 | 84.01 | 20540 |

Template-head inputs define a class template with a deferred body and one typed
instantiation declaration per unit. Relational-fence inputs have ten unknown
operands per statement. Both grow storage with introduced syntax; fourfold
4000 -> 16000 input growth takes roughly 3.4–3.8 times Rust wall time in these
observations. Ordinary functions are a control. The isolated 4096-operand
invariant separately bounds speculation on one long ambiguous region. Checkout
and scratch occupy about 6.7 GiB of the shared 100 GiB capacity.

The next dependency boundary is attribute collection and retained attribute
facts (`parser_specifiers.cpp`, `parser_state.cpp`, AST type/declaration owners),
then lambda capture/declarator/body ownership and speculation
(`parser_expressions.cpp`, `parser.cpp`, `parser_regions.h`). The remaining
fixed failures are all status 86 at those owners, never matching C++ rejection.
No assignment completion or advancement is claimed from 182 fixtures. Remaining
production owners beyond these fixed cases include specialization category
conflict aggregation, dependent prefix guesses and substitution through parameter
bases/alias specialization records, retained ambiguous readings, remaining
parameter/initializer/specifier source facts, hosted vocabulary/prelude/stats,
and the oracle host byte-stack budget (the safe recursion boundary still returns
86 at depth 256). These remain due before their later consumers, and the source
map records the pending responsibilities. Review should trace the typed argument
sharing, member/lexical frame distinction, dependent edge deduplication, complete
undo/cache keys, class-body token lifetimes and iterative views, rather than
infer full phase ownership from fixture counts.

Exact remaining fixed case IDs, by earliest divergent owner:

### Attribute syntax and retained facts (8)

```text
tests/general/100-balanced-attributes.t
tests/general/100-class-alignas-after-class-key.t
tests/general/200-attributed-partial-specialization-current-class-constructor.t
tests/general/200-attributed-virtual-destructor-member.t
tests/general/200-forward-unknown-nested-template-in-ctor-body.t
tests/general/200-template-qualified-inline-attribute-constructor-definition.t
tests/general/200-trailing-parameter-carries-dependency-attribute.t
tests/general/200-trailing-parameter-vendor-attribute.t
```

### Lambda expressions and bodies (3)

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-capture-forms.t
tests/general/200-lambda-declarator.t
```

Exact newly passing fixed case IDs (68):

```text
tests/general/100-decltype-qualified-id-expression.t
tests/general/100-dependent-logical-template-argument.t
tests/general/100-function-type-template-argument-leading-named-parameter.t
tests/general/100-member-operator-name-call.t
tests/general/100-parameter-clause-trailing-ellipsis.t
tests/general/100-parameter-shadows-template-comparison.t
tests/general/100-parameter-shadows-template-strict-comparison.t
tests/general/100-template-argument-paren-greater-nested.t
tests/general/100-template-condition.t
tests/general/100-template-template-parameter-name-reuse.t
tests/general/100-unary-forms.t
tests/general/200-casts-typeid-construction.t
tests/general/200-decltype-less-partial-specialization.t
tests/general/200-dependent-member-template-call.t
tests/general/200-dependent-sizeof-pointer-type-id.t
tests/general/200-dependent-template-id-names.t
tests/general/200-dependent-template-keyword-nested-angle.t
tests/general/200-friend-function-template-declaration.t
tests/general/200-inline-namespace-template-visibility-base.t
tests/general/200-malformed-template-parameter-clause.t
tests/general/200-member-relational-ignores-using-template.t
tests/general/200-member-template-if-less-template-call.t
tests/general/200-member-template-parameter-value-vs-template-name.t
tests/general/200-mock-template-name-angle-forms.t
tests/general/200-namespace-alias-using-directive-imported-template-id-type.t
tests/general/200-nested-qualified-template-id-template-args.t
tests/general/200-nested-template-return-assignment-operator.t
tests/general/200-operator-less-followed-by-operator-greater.t
tests/general/200-pack-base-initializers.t
tests/general/200-parenthesized-qualified-function-template-call.t
tests/general/200-partial-specialization-current-class-constructor.t
tests/general/200-placement-new-identifier-led-initializer.t
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
tests/general/200-switch-case-qualified-template-id-call.t
tests/general/200-switch-case-template-id-call.t
tests/general/200-template-class-body.t
tests/general/200-template-conditional-simple-type-shift-return.t
tests/general/200-template-function-body.t
tests/general/200-template-function-type-alias-declaration.t
tests/general/200-template-function-type-alias-pack-declaration.t
tests/general/200-template-id-function-pointer-argument.t
tests/general/200-template-id-function-pointer-initializer.t
tests/general/200-template-id-return-type-inline.t
tests/general/200-template-id-value-if-condition.t
tests/general/200-template-member-definition-inherited-typedef-cast.t
tests/general/200-template-qualified-conversion-operator-dependent-result.t
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

## Accepted partial checkpoint audit — 2026-10-12

Audit entry commit: `1e826f5d29b8007f034e6780216071ae7b75982c` (clean).
`artifacts/pa5-checkpoint-audit/audit-metadata.json` retains the audit metadata.
Reviewed the full diff from
stage base `d182aa4bc3c8d2a2bfeffb7c615bea7a77a5fd43` through the three accepted
implementation groups and the fixes in this audit. Reviewed implementation commit: `0dc9f88edaafe9b64e59f440d8f9b6745dc090bc`.
The following ledger-only commit records this identity; no compiler or fixture
change follows the reviewed implementation. The earlier implementation turn is **progress**, established by
its committed class/body group, 71 -> 114 matches and retained raw receipts;
no compiler or gate process was live at audit entry.

This certifies the accepted foundation, exception/allocation/name, and
class/member/deferred-body groups as a **partial checkpoint**. It does not
certify full PA5, demand missing grammar, or advance Ralph's stage. Read the
repository guide, clone contract, specification, clone plan, PA5 README/parsing
guide and all checkpoint entries. Consulted the frozen owners directly under
`/home/vishvananda/work/references/v5rust-cpp` at contract revision
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

### Findings and fixes in completed behavior

| Frozen C++ responsibility | Finding and Rust correction |
| --- | --- |
| `syntax/name_scopes.cpp::add_edge_at/search` | Rust searched base/import edges in insertion order, while C++ prepends edges. `scopes.rs` now searches in reverse insertion order, including equal-depth directive heap behavior. Conflicting type/value categories produce the same syntax as the frozen owner. |
| `name_scopes.cpp::add_using_directive/search_level` | Every import was searched immediately in its declaring scope. Owned edges and nominated edges now carry separate typed facts; a directive is queued until lookup reaches its nearest common enclosing namespace. Inner variables correctly precede an outer nomination. |
| `name_scopes.cpp::Found`, `parser_names.cpp::advance_prefix`, `parser_specifiers.cpp` elaborated lookup | Ordinary value bindings incorrectly stopped lookup before `::` and in elaborated class references. Prefix queries now skip a value with no retained type scope, and still find a class hidden by a same-scope object. Terminal name lookup remains ordinary lookup. |
| `parser_state.cpp` initialization | The predeclared `nullptr_t` type and `__func__`/`__FUNCTION__`/`__PRETTY_FUNCTION__` value categories were absent. The parser interns and declares these once before a transaction can capture them, through the shared name owner. Local declarations still override the global categories. GNU registry/prelude work remains pending. |
| `parser_expressions.cpp::primary_from_name`, `ast_printer.cpp` functional casts | Global-qualified and explicit-typename casts were rejected or stored as plain calls. They now share the typed primary-name decision; lone decltype casts retain a Decltype specifier. Unknown parenthesized lexical hints remain calls. The dedicated borrowed dump view omits typename and spaces keyword sequences exactly as C++, while the syntax retains the keyword. |
| `parser_specifiers.cpp::parse_class_body`, `name_scopes.cpp::reopen` | A qualified definition failed to reuse a named inline namespace's forward scope; it also published new bindings where C++ only completes an existing forward. A class first mentioned inside another class retained that class as its parent after definition elsewhere. `classes.rs`/`scopes.rs` now preserve forward identity, select the owning inline namespace, distinguish unknown qualified definitions, and log/restore parent/depth when reopening. Enum scopes cannot be reused as class scopes. |
| `parser_declarators.cpp` function/result contexts | A trailing return type containing a function declarator replaced the defining function's parameter scope. Ordinary and special-member suffix parsing now preserves that function scope across result parsing. |
| `parser.cpp::parse_ctor_initializer`, `parser_declarations.cpp::parse_initializer/special_member_ahead` | Constructor mem-initializers accepted assignment syntax and could reach a printer failure instead of the owner's located syntax error. Default/delete initializers lacked the owner's delimiter condition, hiding a valid delete-expression initializer. Special-member prediction called fail on unrelated builtin declaration keywords, contaminating later deferred-body diagnostics. These boundaries now match the owner, including exact rejecting bytes. |
| `name_scopes.cpp` category marks and base-edge repeat detection | The edge vector's contains scan made a wide base clause quadratic; recursive graph search could exhaust the Rust stack. Indexed edge identities now deduplicate in expected constant time and participate in undo. Reusable iterative DFS and per-name declared/member hints avoid unnecessary class walks and detect cycles. Hints survive rollback like the C++ marks and can never supply a binding. |

All three original implementation groups were reviewed, including the AST
entry/source loop, shared clock extraction, token vocabulary and suffix identity,
cursor demand/capture, all node/list stores, declaration/specifier/declarator/name
grammar, precedence and statement/handler paths, class completion and both
iterative views. The existing per-group owner tables below remain the detailed
responsibility census; `source-map.tsv` was corrected for these fixes and stale
class/deferred coverage descriptions. No full-phase ownership claim was added.

### Ownership, failure, order and demand audit

Traced a derived class with inherited and later typedefs, an inline constructor,
bit-fields, a nested class and a local class through preprocessing, compact name
and scope identities, immutable member lists, BodyId capture and replay, and AST
printing. Each captured primary run is copied once; nested replay uses a range
of the same token slab. Each BodyId transitions from Deferred to Parsed once.
Outermost class completion drives its queue in source order and releases runs;
the AST holds only the parsed body. Print/render work stacks borrow the same
tree. No semantic, template-instantiation, optimizer, register-allocation,
lowering or backend work is demanded by this mode.

Traced nested alternatives through successful inner commits and outer failure:
nodes/list scratch, bindings, edge identities, completion, parent/depth, current
and function scopes, class depth, deferred queues and run lengths all unwind.
Stable source/name/literal owners survive retries. IDs from abandoned syntax do
not escape a successful root. Query heap/DFS/visited storage is reset on its
next query; category marks are conservative hints, not cached facts. Reopening
a forward only changes its own logged parent/depth. Lookup uses indexed
ScopeId/NameId bindings and explicit edges, never a global declaration scan.
Declaration, handler, member, constructor and dump ordering comes from source
lists, not randomized hash iteration. Production Rust names and external tool
spellings remain compatible.

No fixture recognition, oracle/C++ phase delegation, unsafe path, duplicate
parser, text phase transport, deep syntax clone, per-node owning heap tree,
mechanical file split, new dependency or new lint suppression was found. The
clock helper remains the previously audited host time query at the bin boundary.
Literal target bits remain the PA2 facts; syntax does not evaluate widths,
array bounds, arithmetic or template values. Earlier numeric lanes and both
debug/release literal invariants pass. The acknowledged host byte-stack policy,
templates, attributes, lambdas and later hosted/driver surfaces remain explicit
unimplemented boundaries, not certified behaviors.

Four added Rust invariants cover import undo/reinsertion, forward parent undo,
typed casts whose dump labels resemble calls, and a 10,000-edge category chain
with a cycle. Existing body/run-release, complete nested rollback, bounded
6,000-declaration pull-window and long-expression view tests still pass.
Debug/release each pass 68 library plus eight tool tests. The authoritative
source audit examines 77 files with zero findings.

Eleven required regressions and their new expectation sidecars live in the
executed PA5 `tests/general/300-audit-*.t` lane. Expectations were obtained from
the frozen C++ executable for only these new files; no existing reference,
fixture, protected manifest, policy, oracle source or Ralph state was changed.
They cover category order/depth/prefixes, predefined names, casts, inline and
elaborated forwards, result scope and initializer rejection/acceptance.

### Validation and artifacts

Artifact root: `/home/vishvananda/work/private/v5rust/artifacts/pa5-checkpoint-audit/`.
`start-results.json`, `pa1-results.json` through `pa5-results.json`,
`progress.json`, `passed-case-ids.txt`, `failure-case-ids.txt` and
`failure-index.json` retain entry/final case identities, exact failures, tool
hashes and unchanged fixed manifest SHA-256
`27cefa061900dab78611f38c128ded4809888024a76c349668e6cfdcc2ab22f1`.
`cases/<case-digest>/` and `course/pa1/` through `course/pa5/` archive raw
protected receipts. `new-regressions.json` retains the eleven strict course
comparisons. Exact fixed passing and failing sets are unchanged: 114 passes,
79 failures, zero gains and zero losses. The exact 79 failing case IDs listed
in the preceding class checkpoint remain the current set; the audit's sorted
failure-case-ids and per-case trace index are independent final evidence.

| Command | Result and receipt |
| --- | --- |
| `make build` | PASS; `build.log` |
| required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5` (`make test-pa5` route) | FAIL, exit 1, 114/193; `stage.log`, `pa5-results.json`; 79 future-group failures unchanged |
| `make course-pa5` | FAIL, make exit 2/checker exit 1; 125/204 including 11/11 new passes, 79 failures; `course.log`, `course/pa5/test/counts.txt` and raw trace |
| `make -C pa5 check TEST='tests/general/300-audit-*.t'` with protected Rust adapter, runner disabled and strict trace | PASS, 11/11; `regressions.log`, `new-regressions-trace.jsonl` |
| protected `clone_check.py prior pa5` | PASS; fixed PA1–PA4 58/58, 26/26, 20/20, 106/106; full course 58/58, 28/28, 20/20, 112/112; `prior.log`, archived stage/course receipts |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests and source audit; `quality.log` |
| `make audit-rust` | PASS, 77 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS, 114/114 accepted identities; `preserve.log`; `progress.json` separately proves 79 -> 79 failures with unchanged coverage |

`probe.py` compares complete exit/stdout/stderr/AST bytes for 114 audit programs,
including all sixteen four-namespace type/value combinations in global and
nested directive contexts, valid typed syntax and malformed inputs. All match;
`probes.json`, `probes.log`, `probes/` retain the evidence. The original 65-case
pre-fix run exposed 13 mismatches, archived separately in `probes-before.json`
and `probes-before/`. The prior 50 class and 32 exception/allocation scratch
programs were rerun in isolated artifact subdirectories and all still match;
their copied scripts, logs and raw receipts are retained. No oracle/harness
defect was found.

Resource commands: `python3` on this artifact root's `measure.py`. Warm whole
invocations include preprocessing, the clock helper and dump I/O. Frozen tool
and source/output hashes, Linux/rustc 1.98.0, C locale and UTC are recorded in
`resource-metadata.json` and `measurements.json`; every invocation retains raw
time/RSS/stdout/stderr/AST files. Oracle A/A calibration followed by A/B/B/A
blocks proves exact equality on 36 invocations. Wide-base inputs stress the
corrected edge repeat check and a lookup reaching the first base; ordinary
functions provide a control. These are recorded observations, not a PA35 gate.

| Workload | Units | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| wide bases | 1000 | 24.09 | 8104 | 8.30 | 5772 |
| wide bases | 4000 | 38.22 | 8104 | 15.48 | 7888 |
| wide bases | 16000 | 92.13 | 12740 | 52.54 | 16380 |
| functions | 1000 | 25.43 | 8024 | 9.85 | 6512 |
| functions | 4000 | 53.74 | 8032 | 27.47 | 8604 |
| functions | 16000 | 152.68 | 19836 | 83.11 | 20604 |

Checkout plus scratch occupied about 6.5 GiB of the shared 100 GiB allocation;
the measurement imposed no capacity pressure. The next dependency boundary is
unchanged: template parameter categories and typed template-id syntax, then
attributes/lambdas and their retained facts/ambiguities. No pending PA5 group,
future stage or PA34 proof is required for this partial checkpoint audit.

## Class/member and deferred-body checkpoint — 2026-10-12

Entry commit: `4e9dbf23a05003f706080a8ba1cafb0a589e9bff` (clean).
Implementation commit: the commit containing this checkpoint entry. Stage base
and last-audited commit above remain unchanged. The preceding implementation
turn is **progress**, evidenced by its committed exception/allocation group and
63 -> 71 exact matches. The supplied later stageProgress receipt repeated
71 -> 71; entry revalidation found no live compiler/check process to resume.
This entry ports the next coherent group. It is a partial PA5 handoff; Ralph
owns checkpoint/final audits and advancement.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `docs/rust-clone-plan.md`,
`pa5/README.md`, `pa5/parsing.md` and this stage ledger. Inspected the actual
frozen C++ implementation at `/home/vishvananda/work/references/v5rust-cpp`,
contract commit `905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

| C++ owner consulted | Rust owner and completed responsibility |
| --- | --- |
| `syntax/parser_specifiers.cpp`, `ast_declarations.h`, `ast_types.h` | `syntax/classes.rs`, `declarations.rs`, `ast.rs`: class/struct/union keys, forward and elaborated types, definitions, final, ordered bases/access/virtual/pack syntax and class-owned member lists |
| `syntax/name_scopes.cpp`, `parser_specifiers.cpp`, `parser_names.cpp` | `syntax/scopes.rs`, `classes.rs`, `names.rs`: class scope allocation/reopening/completion, injected names, known base edges, qualified nested definitions, friend elaborated declarations and value hiding that retains the class/enum scope |
| `syntax/parser_declarations.cpp`, `parser_declarators.cpp` | `syntax/classes.rs`, `declarations.rs`, `declarators.rs`: ordinary members/access labels, named/unnamed bit-fields, constructors/destructors/conversions, default/delete, definite special-member parameter clauses, member pointers and member-enum simple-declaration wrappers |
| `syntax/parser.cpp`, `parse_token_stream.*`, `parser_state.*`, `parser_regions.h` | `syntax/classes.rs`, `cursor.rs`, `parser.rs`, `ast.rs`: capture balanced member bodies, queue on outermost class completion, parse each body once into its BodyId, ordered nested replay and local-class completion, constructor mem-initializers and function-try bodies |
| `syntax/parser_names.cpp`, `parser_declarators.cpp`, `parser_declarations.cpp` | `syntax/names.rs`, `declarators.rs`, `declarations.rs`: retained prefix ScopeId; qualified member conversion-result, parameters, exception/result suffixes, bounds, initializers and body lookups |
| `syntax/parser_expressions.cpp`, `parser_declarations.cpp` | `syntax/expressions.rs`, `classes.rs`: named function-type alternatives in sizeof/typeid/alignof, silent decline of nonconversion special-member names and exact rejection location |
| `syntax/ast_printer.cpp`, `ast_render.cpp` | `syntax/printer.rs`, `render.rs`: exact iterative class/base/member/special-member/constructor/member-pointer dumps and compact typed declaration/statement/body/handler views |

The tree retains one owning representation, with compact ClassId/BodyId and
immutable list spans. Base and constructor initializer records retain locations
and typed names/operands. A captured member Body starts Deferred and can transition to Parsed
only once through a checked store API; printing rejects an unresolved state.
Qualified names retain their known prefix scope rather than reconstructing it
from spelling. Lookup remains indexed by ScopeId/NameId and explicit edges.
Declaration and body traversal use source lists, not hash iteration.

The pull cursor copies a primary body token run once. Local-class bodies already
inside a replay borrow subranges of that same slab. Runs retain original absolute
token ordinals, nest through resume frames and are released after the owning
outermost class has parsed its queue. Completed syntax bodies retain no token
runs. A failed speculation rewinds every new class/body/list slab, scope binding,
base edge, completion mutation, deferred queue, token-run length and parser class
context, including committed inner alternatives. No whole-unit pre-tokenization,
text phase transport, per-node heap tree or speculative semantic/backend demand
was introduced. The optimizer, register allocator and code generator are untouched.

Three new executed Rust invariant tests cover late member/base categories with
nested local-class replay, rollback of a completed class and all constructor/
bit-field/body/scope state, and 2000 successive classes with token release and a
bounded pull window. The latter retains fewer than 10 body tokens at once and a
window under 4120; the previous 6000-declaration/long-chain invariants still pass.
Debug/release pass all 64 library and eight tool tests. No dependencies or broad
lint suppressions were added. New dead-code expectations are confined to source
locations retained for later semantic diagnostics. The source audit reports
77 files and zero findings.

Artifacts: `/home/vishvananda/work/private/v5rust/artifacts/pa5-classes/`.
`entry.json`, `start-results.json`, `progress.json`, `pa1-results.json` through
`pa5-results.json`, `passed-case-ids.txt` and `failure-index.json` preserve the
entry, final tool hashes, unchanged manifest identity and exact passing/failing
case IDs. `cases/<case-digest>/trace.jsonl` archives protected traces and raw
mismatch diagnostics/ASTs; `course/pa1/` through `course/pa5/` retain complete
course receipts. No fixture, oracle, reference, checker, policy or Ralph state
was changed. No cases were added to or removed from the 193-case fixed manifest.

| Validation command | Final result / artifact |
| --- | --- |
| `make build` | PASS; `build.log` |
| `make test-pa5` (required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa5`) | 114/193 exact matches, 79 failures; checker exit 1 / make exit 2; `stage.log`, `pa5-results.json` and archived traces |
| `make course-pa5` | FAIL, 114/193, failure marker retained; checker exit 1 / make exit 2; `course.log`, `course/pa5/test/counts.txt`, `course-result.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa5` | PASS; PA1–PA4 fixed counts 58/58, 26/26, 20/20, 106/106 and full course counts 58/58, 28/28, 20/20, 112/112; `prior.log` and archived per-stage receipts |
| `make lint` / protected `clone_quality.py` | PASS; rustfmt, strict Clippy, debug/release tests and companion audit; `quality.log` |
| `make audit-rust` | PASS, 77 files, zero findings; `audit.log` |
| protected `clone_check.py preflight pa5` | PASS; `preflight.log` |
| protected `clone_check.py preserve pa5` | PASS, 71/71 prior identities; `preserve.log`; `progress.json` independently proves 122 -> 79 failures, 43 gains and zero losses on the same manifest |

PA5 differential and course checks remain failing; status 86 is still an explicit
missing implementation, never an accepted rejecting compiler. This checkpoint
satisfies the implementation phase's coherent-group partial handoff, not full
assignment completion or stage advancement. No oracle/harness defect was found.

Additional owner validation: run `python3` on the artifact directory's
`differential.py`. All 50 scratch programs match full exit/stdout/stderr/AST
bytes, including malformed class members/bases/bodies, nested deferred/local
classes, inherited/later typedefs, class-name hiding and forward reopening,
qualified nested/special/static members, member pointers, bit-fields and
constructor/function-try bodies. `differential.json`, `differential.log` and
`differential/` retain sources and raw receipts. These supplement the frozen
manifest and do not count as checkpoint progress.

Resource evidence: run the artifact directory's `measure.py`. Oracle A/A
calibration followed by A/B/B/A blocks compare exact stdout/stderr/exit/full
ASTs on all 54 invocations. Fixed sources contain 1000/4000/16000 ordinary
functions, class pairs with inheritance/constructor/bit-fields/later typedefs,
or methods within one class that defer until its final typedef. Binary hashes,
Linux/rustc 1.98.0, C locale and UTC are in `resource-metadata.json`; source/output
hashes, wall time and RSS observations are in `measurements.json`, with raw
`.time`, `.stdout`, `.stderr` and `.ast` files. Measurements are warm whole
invocations including preprocessing, the clock helper and dump I/O; they are
not cold-cache or hardware-counter proof and impose no PA35 target.

| Workload | Units | Rust mean ms | Rust peak RSS KiB | C++ mean ms | C++ peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| functions | 1000 | 25.24 | 8004 | 10.30 | 6476 |
| functions | 4000 | 54.19 | 8036 | 26.92 | 8584 |
| functions | 16000 | 170.67 | 20540 | 90.35 | 21244 |
| classes | 1000 | 54.89 | 8032 | 27.25 | 9924 |
| classes | 4000 | 175.50 | 17024 | 101.29 | 23308 |
| classes | 16000 | 661.60 | 60904 | 397.99 | 77884 |
| members | 1000 | 37.81 | 8020 | 14.63 | 7188 |
| members | 4000 | 100.02 | 13608 | 43.80 | 13540 |
| members | 16000 | 303.58 | 48808 | 142.48 | 39720 |

Storage grows with introduced syntax and the owning class's pending bodies.
Runs release at each class boundary; a single large class retains its bodies
until the required completion event. The 4000 -> 16000 measurements are
observations of that supported boundary, not permission for broader hot-path
work or changed code-generation policy.

Next dependency boundary: translate template declarations and parameter
categories (`parser_declarations.cpp`, `parser_names.cpp`, `parser_state.cpp`,
`name_scopes.cpp`), then typed template arguments/angle fences and dependent
qualified lookups. These are the earliest divergence in 69 remaining cases,
including template-member bodies now reaching that owner. Attribute collection
and retained facts account for three cases; lambda/deferred speculation for
three; unresolved template-id calls for three; mock template-id disambiguation
for one. Qualified template special-member definitions require the template
owner before using the completed ordinary class/body path.

Remaining production surfaces also include complete dependent/conflict/namespace
nomination rules, attributes and special-member explicit conditions, packs,
retained ambiguous readings, hosted extensions/stats/prelude/pack observations,
and the host byte-stack budget. The existing safe recursion limit still returns
86 at depth 256. These are documented unfinished owners, not clone rejections.
Before semantic integration, finish the earlier foundation's parameter/name/
initializer source metadata and ambiguity facts. No later phase should infer
missing facts from these dumps.

Architecture review should inspect typed prefix/member scope flow, class-forward
and value-hiding merge/undo, ordering and one-time completion of BodyId targets,
nested token-run alias lifetimes, complete rollback, and flat iterative views.
It should verify responsibility/size limits and not infer full PA5 completion
from 114 matches. This ledger does not advance the last-audited commit.

Exact newly passing fixed case IDs (43):

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
tests/general/200-member-pointer-const-function-declarator.t
tests/general/200-member-pointer-data-declarator.t
tests/general/200-member-pointer-function-declarator.t
tests/general/200-member-relational-logical.t
tests/general/200-nested-unary-call-arguments.t
tests/general/200-parenthesized-function-style-cast-mem-init.t
tests/general/200-qualified-base-and-ctor-init.t
tests/general/200-qualified-conversion-operator-definition.t
tests/general/200-qualified-nonconversion-operator-missing-return-type-bad.t
tests/general/200-qualified-result-parenthesized-member-pointer-declarator.t
tests/general/200-sizeof-elaborated-class-type-id.t
tests/general/200-sizeof-functional-cast-expressions.t
tests/general/200-sizeof-qualified-value-call-expression.t
tests/general/200-sizeof-zero-arg-functional-cast.t
tests/general/200-typeid-postfix-member-suffix.t
tests/general/200-zero-width-bit-field-declaration.t
tests/general/300-call-argument-prevents-function-declaration.t
tests/general/300-declaration-expression-intake.t
tests/general/300-function-object-ambiguity.t
tests/general/300-local-typedef-shadows-value-qualified-type.t
tests/spec/200-bit-field-declaration.t
tests/spec/200-class-bases-and-ctor-init.t
```

Current exact remaining fixed-manifest failures, clustered by earliest raw
trace divergence; `failure-index.json` links each to source and archived evidence:

### attributes and retained facts (3)

```text
tests/general/100-balanced-attributes.t
tests/general/200-trailing-parameter-carries-dependency-attribute.t
tests/general/200-trailing-parameter-vendor-attribute.t
```

### template declarations and parameter categories (69)

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
tests/general/200-casts-typeid-construction.t
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
tests/general/200-member-template-if-less-template-call.t
tests/general/200-member-template-parameter-value-vs-template-name.t
tests/general/200-namespace-alias-using-directive-imported-template-id-type.t
tests/general/200-nested-qualified-template-id-template-args.t
tests/general/200-nested-template-return-assignment-operator.t
tests/general/200-operator-less-followed-by-operator-greater.t
tests/general/200-pack-base-initializers.t
tests/general/200-parenthesized-qualified-function-template-call.t
tests/general/200-partial-specialization-current-class-constructor.t
tests/general/200-placement-new-identifier-led-initializer.t
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
tests/general/200-template-member-definition-inherited-typedef-cast.t
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

### lambda expressions and deferred bodies (3)

```text
tests/general/100-lambda-cast.t
tests/general/200-lambda-capture-forms.t
tests/general/200-lambda-declarator.t
```

### unresolved template-id calls (3)

```text
tests/general/100-template-condition.t
tests/general/200-switch-case-qualified-template-id-call.t
tests/general/200-switch-case-template-id-call.t
```

### template-id name disambiguation (1)

```text
tests/general/200-mock-template-name-angle-forms.t
```

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
