# PA3 port ledger

Stage base commit: `34c05a7e739705d8a477761b9787589f0c390ee0`
Last audited commit: INITIAL

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
Last audited commit remains INITIAL; implementation does not advance audit
state. The predecessor's committed PA2 acceptance audit is authoritative
progress. PA3 entered clean with its CLI still returning status 86 in all
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

## Evidence

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

## Handoff

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
