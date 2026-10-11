# PA3 port ledger

Stage base commit: `34c05a7e739705d8a477761b9787589f0c390ee0`
Last audited commit: `14002c70fc3d44722095edfb27242016a2a81330`

## Contract and ownership

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, pa3/README.md and
docs/rust-clone-plan.md. Consulted the following owners directly in the frozen
C++ tree `/home/vishvananda/work/references/v5rust-cpp`, revision
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`:

| C++ owner | Rust owner and responsibility |
| --- | --- |
| dev/ppexpr.cpp | dev/bin/ppexpr.rs, dev/bin/source_tool/mod.rs, dev/src/driver/ppexpr.rs: mock query, buffered results, EOF, CLI and diagnostic scopes |
| dev/src/preprocess/expr/controlling_expression.{h,cpp} | dev/src/preprocess/expression.rs: token conversion, grammar, defined, precedence and skipped scopes |
| dev/src/preprocess/expr/pp_arithmetic.{h,cpp} | dev/src/preprocess/arithmetic.rs: target values, conversions, arithmetic validity and intermediate bits |
| dev/src/preprocess/tokens/pp_token_cursor.h | dev/src/lex/pp_lexer/mod.rs: existing lexer plus borrowed PpTokenCursor trait |

The typed flow is SourceBuffer -> borrowed PpTokenCursor -> converted Token
enum -> private pending-operator/value stacks -> ExpressionResult -> decimal
view. PA1 owns source/token production; PA2 owns integer classification and
character decoding. PA3 calls those APIs directly. PpValue carries target bits
and signedness. MacroDefinitionQuery borrows spelling during defined's query;
PA3 tests its first UTF-8 byte's parity. Punctuator and fundamental-type enums
retain identities. No graph edges require new IDs: both stacks are private
flat LIFO storage with no cross-store indices or recursive ownership.

Due tool: ppexpr, fixed PA3 manifest and full original course test target.
The implementation checkpoint left the last audited commit at INITIAL; the
independent full-stage audit below now accepts its due PA3 responsibilities.
The predecessor's committed PA2 acceptance audit is authoritative progress.
PA3 entered clean with its CLI still returning status 86 in all
twenty cases, so their shared earliest divergence was the missing tool.

## Coherent behavior groups

The complete PA3 bare-tool group is implemented:

- Whitespace, empty logical lines and EOF; literal/identifier conversion;
  inherited keyword/alternative-operator distinctions; integral signedness;
  rejection of floating, array, user-defined and multicharacter literals.
- Both defined forms, PP identifier operands including keywords, query before
  checking the closing parenthesis, and malformed-operand recovery.
- Iterative precedence parsing, left-associative binary operators,
  right-associative conditionals, unary/group boundaries, and consumption
  through the logical line after its first expression error.
- Nested short-circuit scopes. Unevaluated operands are converted and typed;
  arithmetic failures alone are suppressed. Both conditional operands decide
  signedness, including failed division/shift/overflow intermediates. Frozen
  reduction order and intermediate bit patterns are preserved.
- Signed overflow checks, unsigned wrap, truncating division/remainder,
  INT64_MIN/-1 rejection, shift-count rejection, signed right-shift fill,
  discarding left shifts, usual conversions and signed bool results.
- Signed/unsigned decimal formatting, 64-KiB buffering, flushing prior results
  on source errors, EOF, initial unlocated versus later located source errors,
  ignored arguments and the no-runner batch sentinel. Shared CLI output failures
  remain sticky. Broken parser invariants return structured internal errors
  with status 70, distinct from expression errors and source rejection.

Each token is classified once at demand. No token sequence, copied line,
syntax tree or text transport is built. Stacks grow geometrically and reuse
capacity; nesting costs heap entries rather than recursive calls. No new
dependency, unsafe code, compiler subprocess, oracle answer, test recognition
or environment read under dev/src/ was added. Optimizer, allocator, code
generator and emission policy are unchanged. Future semantic/template facts
and per-function backend lifetimes remain with their owning stages.

Current failing PA3 IDs: `[]`. Next dependency boundary: PA4 macro expansion
and directives, followed later by GNU literal dialects. The C++ spelling
lookup for synthetic op tokens without canonical punctuator identity also
belongs to later preprocessor input; it cannot execute on PA3's bare lexer.
These partial source responsibilities remain explicit in source-map.tsv.

## Implementation checkpoint evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa3-implement/`.
Every `*-command.json` records command, status, elapsed time and log path.
`inputs.json` records base/oracle commits, toolchain and source hashes. The
implementation commit is the commit introducing this ledger; its exact hash
and clean-checkout status are recorded in external `completion.json`.

PA3 improves **0/20 -> 20/20**, with every entry identity retained. Entry
failing IDs, now passing, are the following under `tests/`:

```text
100-primary.t
100-unicode-character-literals.t
110-paren.t
120-defined-keyword-operand.t
120-defined-malformed-operands.t
120-defined.t
200-adjacent-subtraction.t
200-chained-multiplicative.t
200-chained-shifts.t
200-operator-precedence.t
200-ops-alts.t
200-ops.t
200-signed-unsigned-comparison.t
250-eval-order.t
260-cond-ret-type.t
300-incomplete-expression-bad.t
300-logical-line-error-isolation.t
300-triple.t
300-unconsumed-expression-tokens-bad.t
500-integer-overflow.t
```

`entry-stage.log` preserves the turn-start report; `entry.json` records its
identities. `pa3-result.json` retains the final fixed result and protected
trace paths under `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa3/`.
Those traces compare raw exit/stdout/stderr and record actual Rust routing.
The course trace/counts are copied into `pa3-course/`: 20/20, no failure marker,
twenty matching invocations. Earlier snapshots are `pa1-result.json` and
`pa2-result.json`; course traces remain under artifacts/clone/course/pa1/test
and pa2/test. The fixed manifest digest is unchanged:
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.

| Command | Result; artifact relative to root |
| --- | --- |
| make build | PASS; build.log |
| cargo build --manifest-path dev/Cargo.toml --workspace --locked --target-dir obj/rust | PASS; debug-build.log |
| make test-pa3 (python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa3) | PASS, 20/20; stage.log, pa3-result.json |
| make course-pa3 | PASS, full course 20/20; course.log, pa3-course/ |
| python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa3 | PASS, PA1 58/58 and PA2 26/26 plus both full course targets; prior.log |
| python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa3 | PASS; preflight.log |
| python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa3 | PASS, 0/0 previously passing at entry; preserve.log; all twenty entry identities independently retained |
| make lint | PASS, rustfmt, strict Clippy, both test profiles and companion audit; quality.log |
| make audit-rust | PASS, 42 files/zero findings; audit.log |
| python3 scripts/audit_pa3_expressions.py --evidence <root>/supplemental --profile release (and --profile debug) | PASS, 39/39 exact comparisons each; supplemental-{release,debug}.log, supplemental/{release,debug}/results.json |
| python3 <root>/measure.py | PASS, paired output equality at every scale; performance.log, performance/results.json and raw files |
| git diff --check | PASS; completion.json |

The supplemental tool executes 23,749 generated expressions per profile:
all unary/binary operators and alternative spellings, signed/unsigned
boundaries, precedence pairs, nested skipped failures, malformed token streams
and 65,536-deep grouping/unary/conditional stacks. Generated streams must emit
one line per input expression plus EOF, preventing early-stop false passes.
It also runs the unchanged course inputs and eleven source/CLI probes,
including errors after a skipped line and a 64-KiB flush, initial decoding,
seek offsets, empty input, ignored arguments and the no-runner batch protocol.
Raw inputs/results/diagnostics, exits and hashes are retained. Three added Rust
unit tests exercise typed skipped operands/query demand, 50,000-deep flat
storage/capacity reuse/error isolation, and source-error output prefixes.
Both profiles pass 31 library tests and six shared CLI tests.

Final ppexpr SHA-256: release
`c0bf014f821cfacbab1df5f5c3ecc487f4f812eea66f192718d8c83387615e83`;
debug `986adfaf38e7a0d285f3f2ee6802508c29b8cad8949499b9c9fbb77b8c808630`.

### Supported hot-path costs

Final release tools, two Rust warmups and two paired ABBA blocks per workload,
after other jobs finish. Every output matches byte for byte. All commands,
input/tool hashes, wall time, observed duration and peak RSS are retained in
performance/results.json. The streamed seed is `false ? 5/0u : -5 + 3 * 2`;
grouping nests parentheses around 1; the conditional chain repeats
`0 ? 1u :` then -5.

| Workload | Scale | Source bytes | Rust median ms / max RSS KiB | C++ median ms / max RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| streamed lines | 4,096 | 106,496 | 0 / 2,240 | 0 / 3,688 |
| streamed lines | 16,384 | 425,984 | 20 / 2,224 | 10 / 4,352 |
| streamed lines | 65,536 | 1,703,936 | 105 / 3,648 | 50 / 5,048 |
| nested groups | 4,096 | 8,194 | 0 / 2,240 | 0 / 3,664 |
| nested groups | 16,384 | 32,770 | 0 / 2,224 | 0 / 3,656 |
| nested groups | 65,536 | 131,074 | 10 / 2,236 | 0 / 3,724 |
| conditional chain | 4,096 | 36,867 | 0 / 2,240 | 0 / 4,396 |
| conditional chain | 16,384 | 147,459 | 10 / 2,944 | 0 / 5,052 |
| conditional chain | 65,536 | 589,827 | 30 / 5,056 | 20 / 8,504 |

Time/storage grow with source and required stacks. Ten-millisecond reporting
granularity, warm caches and local scheduling limit interpretation; zero is
below resolution. Rust throughput is recorded separately from generated-code
performance. PA35 optimization targets were not imposed.

## Implementation checkpoint handoff

The due PA3 group is complete and validated. Independent audits and stage
advancement remain Ralph's responsibility. No remaining PA3 failure, oracle
instability or harness defect was observed. Frozen sources, fixtures,
references, checker policy and Ralph state were not edited.

PA4 next supplies MacroDefinitionQuery and PpTokenCursor from its macro table
and expansion/directive input, preserving defined operands and token demand.
Independent review should check reduction/error demand order, balancing of
skipped scopes, conditional signedness, buffer flush/handler scopes and compact
canonical token identities across macro expansion. Later GNU dialects and
noncanonical synthetic-token lookup remain due at their actual consumer.

## Independent full-stage acceptance audit, 2026-10-11

Reviewed the stage base `34c05a7`, the complete PA3 implementation and audit
entry `14002c7`, the full owning handout, AGENTS.md, CLONE_CONTRACT.md,
spec.md, conversion plan and source map. The preceding goal turn was progress:
it committed the complete controlling-expression group and reduced the fixed
manifest's failures from twenty to zero. This audit independently read the
frozen C++ owners above and revalidated the current checkout; reused checks
were not used as acceptance evidence.

No stage-owned compiler defect or unported due behavior was found. The evidence
gap was direct observation of cursor demand, macro queries, typed outcomes and
repeated EOF, which the decimal tool view does not expose. New test-only
`scripts/audit_pa3_demand.py` builds isolated consumers of the frozen C++ owners
and current Rust library, then compares every request/token/query/result and
source-error diagnostic byte. Both profiles pass 81/81 comparisons: 33 focused
boundary/recovery sequences, six generated groups, two empty/translation probes,
all twenty unchanged course inputs and twenty source-error probes. Successful
streams must reach three EOF results; failures must return exactly status 1.
The source-error probe places lexer construction inside its located handler on
both sides; separate CLI probes verify the real tool's outer constructor scope.

This supplements direct source review rather than replacing it. Current fixed,
full-course, typed/demand, supplemental and stream failing IDs are all `[]`.
Every one of the twenty passing identities at audit entry remains passing;
protected preservation reports 20/20 and the receipt independently compares
the entry log with the fresh fixed result.

### Responsibility review and completeness reasoning

| Due C++ responsibility | Reviewed Rust behavior and evidence |
| --- | --- |
| Token conversion | Borrowed PP tokens are classified once as demanded. All keywords except true/false/defined become signed zero when their PP kind is Identifier. The frozen lexer treats new/delete and alternative operator words as punctuators; their operator meaning or rejection is preserved. Hash/digraph-hash tokens reject. Strict Cxx11 integer types and character scalar facts come directly from the PA2 owners; floats, arrays, user-defined and multicharacter literals reject even in unselected operands. |
| Defined operands | Both forms require a PP Identifier, including keywords and decoded Unicode names. The mock tests the first UTF-8 byte, not a Unicode code point. The query executes before demanding the closing parenthesis and even in skipped operands; malformed operands and first-error line discard preserve query/request order. Direct traces include decoded UCNs, odd/even UTF-8 lead bytes, keyword operands and missing delimiters. |
| Grammar and reductions | Every C++ unary/binary operator and precedence level has a typed Rust counterpart. Equal-precedence binaries reduce leftward; conditional alternatives remain pending for right association. Parentheses and incomplete expressions have the same boundaries. Reduction loops continue through pending operators even after arithmetic failure, exactly as in C++; the parse loop then stops and consumes the rest of the logical line without further conversion or queries. |
| Skipped scopes and conditional types | Nested &&, || and ?: scopes suppress only arithmetic errors, preserving conversion and syntax errors. The logical operation is checked before its skipped scope is closed. Both alternatives determine unsignedness; invalid division/shift produces typed zero bits, while invalid signed arithmetic retains its wrapped intermediate bits, matching the frozen implementation. Traces and generated boundary cases cover nested skipped operations and scope reset after invalid lines. |
| Target arithmetic | All 23 operator identities, precedence ranks and result types match. Signed add/subtract use bitwise overflow tests; signed multiplication compares magnitudes against the sign-dependent limit. Unary minus and INT64_MIN/-1 division/remainder reject. Division truncates toward zero; remainder follows the dividend. Unsigned arithmetic wraps explicitly. Signed comparisons flip the sign bit; shifts retain the left type, reject negative/count >=64, discard left-shift bits and fill negative right shifts. No host signed-overflow or debug/release arithmetic policy defines target behavior. |
| Result and CLI bytes | Signed magnitudes, unsigned suffixes, error/empty lines, EOF and 64-KiB flushing match. Completed output survives later source failures; initial lexer failures remain unlocated, later failures located. Shared sticky output failure continues scanning. Ignored arguments, seek offsets, directory stdin and the documented no-runner batch sentinel match in both profiles. The batch sentinel is not used as successful course execution. |

The strict bare-source PA3 tool reaches every due branch described above.
The C++ spelling fallback for an operator token without punctuator identity
cannot execute with PA3's lexer, which always assigns that identity; its first
consumer is PA4 macro-produced input. GNU number dialects likewise cannot be
selected by ppexpr, which constructs the Cxx11 evaluator. These future shared
API responsibilities remain explicitly partial in the source map. Macro
expansion/directives/predefined inputs are excluded by the PA3 handout and
belong to PA4. PA33's implementation-wide census and debug release lanes, and
PA34's bootstrap proof, remain due at those stages.

### Architecture and Rust idioms

All due work runs through `dev/bin/ppexpr.rs` into shared Rust source, lexing,
literal, expression and arithmetic owners. Fresh course and fixed traces name
the actual Rust release binary; the oracle serves only comparison/evaluation.
The external C++ demand probe is a test consumer, never a candidate tool.
Cargo.lock contains only the workspace package. No dependency, unsafe code,
FFI, subprocess phase, fixture recognition, stored answer or environment read
under dev/src exists on the due path.

One immutable source image and one reused lexer scratch/current token feed
private flat operator/value stacks. A spelling borrow ends before another
request; typed enum identities and PpValue facts survive without reparsing or
copied token sequences. No graph edges need new IDs. Parsing is iterative,
without speculation, recursive ownership or abandoned trees. Each token and
operator is pushed/popped at most once; stacks grow geometrically, reset
contents per expression and retain capacity. The deep-stack unit and direct
probes exercise 50,000/65,536-level nesting and recovery. Invalid language
expressions, source failures and internal stack violations remain distinct
structured outcomes.

There are no semantic caches, template facts or machine functions in this
phase; complete cache keys, transactional speculation and per-function backend
lifetimes remain obligations of their actual owners. Current demand is one
token at a time, including defined's explicit operand/closing-token demands.
Optimizer, register allocator, code generation and emission policies were
preserved. PA35 optimization targets were not imposed.

Rustfmt and strict all/pedantic Clippy pass, with no new allowance or lint
expectation. Both profiles pass 31 library tests and six shared CLI tests.
The authoritative companion audit reports 42 files and zero findings.
`shape.json` records maxima of 471 code lines per file, 94 per function and
nesting four, below implementation/interface/model and function/nesting caps.
No mechanical splits, compressed code or hidden phase dependency were found.

### Fresh commands and retained artifacts

Audit root: `/home/vishvananda/work/private/v5rust/artifacts/pa3-audit/`.
`inputs.json` records base/reviewed/oracle commits, toolchain, source hashes,
manifest and binary hashes. `*-command.json` records exact commands, exits,
duration and log paths; `commands.json` collects them. `completion.json`
verifies routes, current hashes, expected counts, unchanged compiler sources,
entry-case preservation and the audit commit's clean-checkout handoff.
Rust binary hashes remain those recorded in the implementation checkpoint.

| Command | Fresh result; artifact relative to audit root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `cargo build --manifest-path dev/Cargo.toml --workspace --locked --target-dir obj/rust` | PASS; `debug-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa3` (`make test-pa3`) | PASS, 20/20; `stage.log`, `pa3-result.json`, `pa3-cases/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py course pa3` (`make course-pa3`) | PASS, original full course 20/20, no failure marker, twenty matching Rust invocations; `course.log`, `pa3-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa3` | PASS, PA1 58/58 and PA2 26/26 fixed cases, original full course targets 58/58 and 28/28; `prior.log`, `pa{1,2}-result.json`, `pa{1,2}-cases/`, `pa{1,2}-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa3` | PASS, frozen inputs/sources/tools/policy and Rust routing; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa3` | PASS, 20/20; `preserve.log`, `entry-stage.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_quality.py`, `make lint` | PASS, fmt/strict Clippy/both test profiles/companion audit; `quality.log`, `lint.log` |
| `make audit-rust` | PASS, 42 files/zero findings; `audit.log`, `shape.json` |
| `make test-harness` | PASS, seventeen protected checker tests; `harness.log` |
| `python3 scripts/check_fixture_lanes.py` | PASS, all assignment inputs selected by executed lanes; `fixture-lanes.log` |
| `python3 scripts/audit_pa3_expressions.py --evidence <root>/supplemental --profile release` and `--profile debug` | PASS, 39/39 exact comparisons per profile, 23,749 generated expressions, eleven source/CLI probes and twenty course inputs; `supplemental-{release,debug}.log`, `supplemental/{release,debug}/results.json` |
| `python3 scripts/audit_pa3_demand.py --evidence <root>/demand-release --profile release` and `--evidence <root>/demand-debug --profile debug` | PASS, 81/81 typed/demand comparisons per profile; `demand-{release,debug}.log`, `demand-{release,debug}/results.json`, probe sources/build commands/raw traces |
| `python3 <root>/streams.py` | PASS, 50/50 exact stream/CLI comparisons across both profiles; `streams.log`, `streams.json` |
| `python3 <root>/measure.py` | PASS, paired byte equality at every scale; `performance.log`, `performance/results.json` and raw observations |
| `git diff --check` | PASS; `completion.json` |

The protected manifest SHA-256 remains
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.
No original fixture/reference, frozen C++ source, protected acceptance policy
or Ralph state was edited. No production fix or new required language fixture
was necessary; the new evidence tool tests the shared phase boundary directly.

### Resource observations and handoff

Measured the unchanged supported streamed/grouped/conditional workloads at
4,096, 16,384 and 65,536 scale. The final measurement runs after validation
jobs finish, with two Rust warmups and two ABBA blocks; every paired output
matches. The earlier overlapping measurement is retained in
`performance-overlap/` and is not the final resource receipt. Raw time/RSS,
observed duration, commands, source hashes and tool hashes are retained.

| Workload | Scale | Source bytes | Rust median ms / max RSS KiB | C++ median ms / max RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| streamed lines | 4,096 | 106,496 | 5 / 2,220 | 0 / 3,652 |
| streamed lines | 16,384 | 425,984 | 20 / 2,224 | 10 / 4,344 |
| streamed lines | 65,536 | 1,703,936 | 100 / 3,632 | 45 / 5,064 |
| nested groups | 4,096 | 8,194 | 0 / 2,236 | 0 / 3,652 |
| nested groups | 16,384 | 32,770 | 0 / 2,224 | 0 / 3,736 |
| nested groups | 65,536 | 131,074 | 10 / 2,240 | 0 / 3,652 |
| conditional chain | 4,096 | 36,867 | 0 / 2,216 | 0 / 4,344 |
| conditional chain | 16,384 | 147,459 | 0 / 2,932 | 0 / 5,048 |
| conditional chain | 65,536 | 589,827 | 30 / 5,040 | 15 / 8,504 |

Ten-millisecond sample resolution, warm caches and local scheduling limit
interpretation; zero is below resolution and medians can fall between samples.
Storage/time increase with source bytes and necessary pending stacks. These are
Rust compilation observations, separate from generated-code equality; no new
code-generation or throughput target was introduced. Retained audit artifacts
occupy approximately 2 GiB within the shared 100 GiB ceiling and are untracked.

Acceptance is based on responsibility review, direct phase evidence, exact
public bytes, original full course coverage, earlier-stage preservation and
Rust quality together. PA3 is accepted at `14002c7`. The next dependency
boundary is PA4's macro table and expansion/directive cursor, then later GNU
dialects. Ralph retains ownership of stage advancement and its existing goal.
