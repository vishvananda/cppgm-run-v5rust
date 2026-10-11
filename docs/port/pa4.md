# PA4 port ledger

Stage base commit: e953408ec36e27923018214d0c788e91d2756633
Last audited commit: INITIAL

Implementation checkpoints:

- `52ff1e8344d359aaa501d740291fb105f6300df6`: macro group (2026-10-11).
- `5c06195f4c6f7960bcc768092a844871f1eaa2df`: conditional inclusion and controlling-expression rescanning (2026-10-11); entry HEAD `fd2481a` (documentation handoff after the macro implementation).
- `16f0a9779f89f5b82544b1f1085b53593df1f749`: presumed locations, file/line builtin replacement and macro-expanded line control (2026-10-11); entry HEAD `af7984f93d7ba4aedca00014cd7590441667d207`.

The current checkpoint is a coherent partial PA4 handoff. Ralph owns stage
advancement, checkpoint audits and the last-audited marker, which remains
`INITIAL`. The previous goal turn was progress: the conditional implementation
and its evidence were committed. This turn reduces the fixed failures from
14 to 8 while preserving 92/92 turn-start passing identities. Full PA4 is
still incomplete.

## Contract and ownership

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `docs/rust-clone-plan.md`,
`pa4/README.md`, `pa4/macros.md` and `pa4/directives.md`. The implementation
oracle is the frozen C++ checkout at
`/home/vishvananda/work/references/v5rust-cpp`, commit
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

The due tool is `preproc`; the unchanged fixed manifest executes 106 cases
(74 macro and 32 directive cases). No fixtures or acceptance policy changed.
Consulted C++ owners and translated Rust responsibilities:

| C++ owner under `dev/` | Rust owner | Responsibility |
| --- | --- | --- |
| `preproc.cpp`, `src/lex/post_token_writer.cpp` | `dev/bin/preproc.rs`, `dev/src/driver/preproc.rs`, shared `driver/posttoken.rs` | File CLI, primary-source reset, post-token view and failure boundaries |
| `src/lex/identifier_table.{h,cpp}` | `dev/src/lex/identifiers.rs` | Private spelling interning, compact `NameId` |
| `src/preprocess/source/source_stack.{h,cpp}` | `dev/src/preprocess/input.rs` | Single-file text/directive/expanded-line boundaries, newline location, inactive lexing and stable spellings |
| `src/preprocess/source/source_files.{h,cpp}`, `preprocessor.cpp` location queries | `dev/src/preprocess/locations.rs` | Lazy physical-line index, byte columns, presumed-name/delta history and cached escaped file literal |
| `src/preprocess/preprocessor_directives.cpp` line control | `dev/src/preprocess/line_control.rs`, shared `dev/src/lex/{strings.rs,text.rs}` | Expanded-line operand grammar, range/diagnostic precedence and shared ordinary-string decoding |
| `src/preprocess/macro/macro_definition.{h,cpp}` | `dev/src/preprocess/macros/definition.rs` | Definition grammar, immutable roles, expanded-argument demand and redefinition |
| `src/preprocess/macro/macro_expander.{h,cpp}` | `dev/src/preprocess/macros/expander.rs` | Iterative rescanning, arguments, substitution, paste/paint, raw defined and attribute probes |
| `src/preprocess/macro/hide_sets.{h,cpp}` | `dev/src/preprocess/macros/hide_sets.rs` | Ordered interned additions, skew-jump unions and persistent membership indexes |
| `src/preprocess/lexer/pp_token.h`, `src/lex/post_tokenizer.{h,cpp}` | `dev/src/preprocess/macros/token.rs`, shared `dev/src/lex/{mod.rs,pp_lexer/mod.rs}` | Stable spellings, typed punctuators, spans and name identity through phase 7 |
| `src/preprocess/preprocessor{,_directives}.cpp`, `preprocessor_names.cpp`, `preprocess_error.h` | `dev/src/preprocess/{unit.rs,conditionals.rs,error.rs}` | Unit composition, macro/conditional directives, skipped nesting/order, reserved names and structured errors |

`docs/port/source-map.tsv` records partial ownership where hosted syntax,
predefined environments, directives or telemetry remain unported.

## Completed behavior group and architecture review

- Object/function definitions, parameter grammar including variadics, missing
  whitespace and malformed-parameter diagnostics; exact redefinition checks
  include internal whitespace separations and parameter identity.
- Replacement roles are analyzed once. Ordinary arguments are expanded once
  only when used; stringized/pasted operands retain raw tokens. Parentheses and
  comma links let nested invocations select argument ranges without rescanning
  or copying the enclosing argument.
- Stringizing preserves whitespace/escaping; paste uses the owning lexer in
  token-text mode and requires one token. Adjacent paste operators,
  placemarkers, pasted-helper rescanning and the oracle's variadic comma-paste
  behavior follow the C++ owner.
- Course recursion suppression preserves per-token paint, hide sets, origin,
  substituted/raw argument flags, open context lifetimes, inherited disabled
  names and parameter-substitution nesting boundaries. No recursion cutoff.
- Fixed predefined constants and the oracle's `__COUNTER__` replacement are
  present. File/line values now use invocation locations and the current presumed
  state. Date/time definitions still explicitly report missing implementation
  when demanded. An unused or stringized raw argument does not demand builtin
  values. Attribute probes
  remain identifiers outside controlling expressions, as in C++.
- Source bytes remain immutable. Clean nonidentifier spellings borrow the
  image; translated/generated text and interned names share immutable `Rc`
  storage. Tokens publish `NameId`, typed punctuators and source spans.
  Definitions and hide-set stores expose private typed APIs. The shared
  `PostTokenInput` consumes the preprocessor directly; there is no textual
  transport or whole-unit token materialization.
- Contexts, arguments and expanded arguments use explicit LIFO storage.
  Invocation contents are released at completion/context pop. Membership trie
  recursion is bounded by 32 identifier bits. Deep sets use lazy persistent
  indexes; ancestor unions retain the ordering needed by `without_last`.
- CLI environment/arguments stay in `dev/bin/`; there are no new dependencies,
  unsafe code, compiler subprocesses or fixture-dependent paths in phases.
  Backend, optimizer and emission policies were not changed.

The conditional group adds `#if`, `#ifdef`, `#ifndef`, `#elif`, `#else` and
`#endif`, exact active/inactive ordering checks, extra-operand diagnostics and
unterminated-section locations. An already taken section skips subsequent
`#elif` operands without expansion. Nested skipped sections check order but
ignore operands; skipped includes request header-name lexing. Ordinary skipped
text is never stabilized, interned or macro-expanded.

`SourceInput` switches explicit text/line/end modes with bounded lookahead.
The rescanner feeds a borrowed `PpTokenCursor` directly into the shared PA3
stacks. Cursor errors have an associated type, so preprocessing errors retain
their structured owner and earliest diagnostic. `defined` uses compact name
identity, reads raw operands and queries private macro slots in O(1), including
when produced by replacement or during argument prescan. Identifier-like
operators answer false. Standard/GNU attribute probes read raw names, preserve
the exact reporting vocabulary/namespace behavior, and remain identifiers
outside controlling expressions. No rendered-token transport or reparsing.

The reusable evaluator and conditional stacks belong to the translation unit;
line tokens are released on the next demand. Inactive nesting uses a flat
stack, with no recursive calls or depth cutoff. Tests exercise 50,000 skipped
sections, unchanged identifier allocation across skipped text, prescan/raw
operator behavior, empty expansion storage and diagnostic precedence.

Invariant tests cover deep hide-set membership/unions and token history,
4,096 nested argument prescans with all live buffers empty at EOF, typed paste
spans, name identity through substitution/literal-operator splitting, and
argument demand/prescan-once behavior. These supplement the differential suite.

## Macro checkpoint evidence (historical)

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/`.
The validated candidate implementation is commit `52ff1e8`; the subsequent
ledger commit changes documentation only. Release `preproc` SHA-256:
`1096b35a90afff431f936a7a54765fa317c6452323920dbf3dfdd22f06ed7e0c`.

| Command | Outcome | Log relative to evidence root |
| --- | --- | --- |
| `make build` | PASS | `pa4-macros-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS | `pa4-macros-preflight.log` |
| `make test-pa4` (runs required `clone_check.py stage pa4`) | 78/106 PASS; 28 FAIL | `pa4-macros-stage.log` |
| `make course-pa4` | FAIL on remaining directive groups | `pa4-macros-course.log`; `clone/course/pa4/test/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: 104 fixed PA1–PA3 cases and all three full course targets | `pa4-macros-prior.log`; `clone/course/pa{1,2,3}/test/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS: turn-start passing identity set was empty (0/0) | `pa4-macros-preserve.log` |
| `make lint` (`clone_quality.py`) | PASS: rustfmt, strict Clippy, debug/release tests, audit | `pa4-macros-quality.log` |
| `make audit-rust` | PASS: 52 files, zero findings | `pa4-macros-audit.log` |

Turn-start failures: **106/106**. Checkpoint failures: **28/106**. Every
original macro case passes (74/74), plus directive cases `120-invalid.t`,
`150-null-directive.t`, `170-nondir2.t` and
`600-repeated-argument-expansion.t`. Failure reduction
is 78 original identities, with the manifest still at 106 cases. The complete
case-ID/status/trace list is `pa4-macros-case-summary.json`; every passing
trace was checked for matching status 0/1 and zero byte differences. Example
valid rejection: `clone/cases/pa4/fd2095703c2ec1bb4a57/trace.jsonl` for
`tests/macros/300-badhash1-1.t`. All remaining traces show status 86 at a named
missing dependency; none is claimed as a matching C++ rejection.

## Conditional checkpoint validation

Evidence root remains `/home/vishvananda/work/private/v5rust/artifacts/`.
Validated implementation: `5c06195`; the next ledger/source-map commit changes
only documentation. Release `preproc` SHA-256:
`8f25547d91fd6ba4fb6ee6eb603129a2ee187625e84976a5066ed004b84a5555`.

| Command | Outcome | Log relative to evidence root |
| --- | --- | --- |
| `make build` | PASS | `pa4-conditionals-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS | `pa4-conditionals-preflight.log` |
| `make test-pa4` (required `clone_check.py stage pa4`) | 92/106 PASS; 14 FAIL; command exits 1, make exits 2 | `pa4-conditionals-stage.log` |
| `make course-pa4` | 92/106; FAIL, failure marker remains | `pa4-conditionals-course.log`; `clone/course/pa4/test/{course-result.json,counts.txt,trace.jsonl}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: 104 fixed PA1–PA3 cases and all three course targets | `pa4-conditionals-prior.log`; `clone/course/pa{1,2,3}/test/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS: 78/78 turn-start identities retained | `pa4-conditionals-preserve.log` |
| `make lint` (`clone_quality.py`) | PASS: rustfmt, strict Clippy, debug/release tests, audit | `pa4-conditionals-quality.log` |
| `make audit-rust` | PASS: 53 files, zero findings | `pa4-conditionals-audit.log` |

All 106 original case IDs still execute. The complete captured invocation
records in `pa4-conditionals-case-summary.json` verify matching status 0/1 and
zero byte differences for each of the 92 passing cases; the remaining 14
return explicit status 86 at their next dependency. Neither a new fixture nor
a smaller manifest supplies this reduction. Newly passing directive IDs:
`150-error.t`, `150-no-error.t`, `170-nondir1.t`, `170-nondir3.t`,
`170-nondir4.t`, `170-nondir5.t`, `170-nondir6.t`,
`200-conditional-exclusion.t`, `200-defined-identifier-like-operator.t`,
`200-if.t`, `300-defined-operator-misuse.t`, `400-multiple-source-files.t`,
`400-predefined-macros.t`, `500-alt.t`.

Supplemental generated differential evidence:
`pa4-conditionals-differential/{check.py,results.json,summary.json}` and
`pa4-conditionals-differential.log`. Of 362 combinations, **336 match** status,
stdout, stderr and complete output bytes. All 26 other combinations reach
explicit pending dependencies: 23 demand location builtins, three demand file
inclusion. Their exact generated IDs and raw source/output/diagnostic artifacts
are retained; they are not claimed as matching rejections. Supported cases
cover malformed operators and sections, diagnostic precedence, branch
selection, attributes, `defined` through prescan/paste and EOF without newline.
No oracle or harness defect was identified.

## Conditional checkpoint remaining case IDs (historical)

All paths below are relative to `pa4/tests/directives/`.

- Source frames/inclusion: `200-include.t`, `400-bad-include.t`,
  `400-header-guard-shapes.t`, `400-header-guarded.t`, `600-pragma-op.t`,
  `800-pragma-once.t`. Owners: `source_stack.cpp`, `source_files.cpp`,
  `preprocessor_directives.cpp` includes/guards and `preprocessor.cpp` once.
- Line directive: `300-line-new-line.t`. Owner: `run_line`.
- Location/build-time builtin replacement: `500-predefined-macros.t`,
  `600-line-macro.t`, `600-predefined-macro-argument-expansion.t`,
  `610-line-macro.t`, `660-line-directive.t`. Owners: `replace_builtin` and
  source inclusion presumed locations.
- Pragmas: `500-pragma-ignore.t`, `501-pragma-op-ignore.t`.
  Owners: directive pragma execution and `run_pragma_operator`.

Remaining production responsibilities include per-file conditional-depth
boundaries, file identity/once and guard skips, macro-expanded include/line
operands, build-time capture in the CLI, exact statistics counters and
file/location limits. Hosted named variadics and other hosted probes remain in
the later extension group. Backend/optimizer/emission policies are unchanged.

## Macro checkpoint resource observations (historical)

Reproducer and raw inputs/outputs/CSV/measurements:
`/home/vishvananda/work/private/v5rust/artifacts/pa4-macros-resources/`.
Run `python3` on its `measure.py`. Five observations per size/tool; all nine
workloads produce identical complete C++/Rust output hashes. Times below are
median wall milliseconds including process launch; RSS is the maximum of five
`/usr/bin/time` observations. These are descriptive measurements, not an
isolated comparative performance claim.

| Supported workload | Scale | Input bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Repeated argument expansion + identifier paste | 10,000 rows | 180,041 | 29.534 | 2,220 |
| Same | 50,000 rows | 900,041 | 129.630 | 2,928 |
| Same | 100,000 rows | 1,800,041 | 253.388 | 3,632 |
| Nested `ID` argument prescan | 1,024 deep | 4,115 | 4.592 | 2,916 |
| Same | 4,096 deep | 16,403 | 8.042 | 4,332 |
| Same | 16,384 deep | 65,555 | 23.696 | 11,352 |
| Distinct object-macro context chain | 1,024 deep | 18,283 | 5.975 | 2,924 |
| Same | 4,096 deep | 79,723 | 13.565 | 5,736 |
| Same | 16,384 deep | 338,253 | 50.515 | 16,992 |

Streaming retention follows source bytes rather than output length; deep
prescan storage follows invocation depth. The object chain exercises persistent
hide-set membership. No native-stack failure at 16,384 nested invocations.

## Conditional checkpoint resource observations

Reproducer and raw inputs/outputs/CSV/measurements:
`/home/vishvananda/work/private/v5rust/artifacts/pa4-conditionals-resources/`.
Run `python3 measure.py` there; `summary.txt` records this run. Five
observations per size/tool; all nine workloads have identical complete C++/Rust
output hashes. Median wall milliseconds include process launch; RSS is the
maximum of five `/usr/bin/time` observations. These measurements ran alongside
validation and are descriptive, not a calibrated speed comparison.

| Supported workload | Scale | Input bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Active `ID(defined FLAG)` groups; unused `#elif __LINE__` | 10,000 | 680,031 | 58.935 | 2,904 |
| Same | 50,000 | 3,400,031 | 216.594 | 5,008 |
| Same | 100,000 | 6,800,031 | 435.250 | 8,532 |
| Inactive distinct names and undemanded builtins | 10,000 | 328,912 | 11.238 | 2,228 |
| Same | 50,000 | 1,688,912 | 47.176 | 3,604 |
| Same | 100,000 | 3,388,912 | 89.764 | 5,012 |
| Nested skipped sections with ignored operands | 1,024 | 48,150 | 4.307 | 2,204 |
| Same | 4,096 | 192,534 | 10.672 | 2,196 |
| Same | 16,384 | 770,070 | 31.682 | 2,900 |

Time follows input work; retained memory follows the immutable source plus
nesting depth, rather than the volume of expanded output or skipped names.
The 50,000-level invariant test also passes in debug and release.

## Location-control checkpoint behavior and architecture

C++ owners read directly: `dev/preproc.cpp` error/build-time boundaries,
`preprocessor.cpp` (`FileLiteral`, `EarlierState`, `presumed_line`,
`presumed_file_literal`, `describe`), `preprocessor_directives.cpp` (`run_line`,
`DecodeOrdinaryString`), `source/source_files.{h,cpp}` line indexing and
inclusion state, and `macro/macro_expander.cpp::replace_builtin` plus
replacement/substitution location handling.

The group completes single-file `__LINE__`, `__FILE__` and `#line`, including
macro-expanded operands, comments/splices and the physical newline ending the
directive, decimal-only line numbers in 1..=2147483647, exact operand/lexical
error precedence, ordinary-string filename decoding, and file-literal escaping
for quotes, backslashes, control bytes and non-UTF-8 bytes. Filename literals
are cached until a filename operand changes their state. Source `NameId` and
`SourceOffset` identities survive replacement and argument prescan; unused,
stringized and pasted arguments retain the existing demand boundaries.

Typed data flow: the CLI passes the primary path's bytes into
`Preprocessor::with_name`; `SourceInput` owns one private `Locations` record
for its immutable source image. `Expander` queries the invocation token's
compact offset for a number or shared literal. A reusable directive-local
vector receives typed expanded tokens; the line parser consumes those tokens
and the shared phase-5 converter, producing a checked line number and optional
filename bytes. After applying the state change, token contents are cleared.
No text dump is used as phase transport, and no unit token sequence is built.

The physical-line table is built once on first location demand, with one
compact offset per physical line, not a record per character/token. Location
lookup uses binary search. Presumed name/delta history is a flat vector with
one record per `#line`, sharing unchanged names. Builtin replacement uses the
latest state, exactly as C++ does; diagnostic lookup uses the state at its
original location. The driver obtains the input owner's diagnostic before
releasing it and writes bytes directly, preserving decoded filename bytes.
Unit/line storage lifetimes remain bounded by the source unit/directive.
Backend, optimizer and emission code is untouched; no dependencies, unsafe
code, process fallback, environment reads or reference answers were added.

Invariant tests prove lazy indexing and literal caching, original-location
history including its newline boundary, BOM byte columns, binary filename
escaping, line-operand prescan and complete release of directive/expansion
contents. They pass in debug and release with the existing ownership tests.
The file audit covers both new responsibility modules and finds no violations.

## Location-control checkpoint validation

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/`.
Validated implementation and source map: `16f0a9779f89f5b82544b1f1085b53593df1f749`.
The subsequent ledger commit changes documentation only. Release `preproc`
SHA-256: `f3ac4950678db1c64af9fde02e21620243f5e5a4da6e2d30ee7e08c61cb559fc`.

| Command | Outcome | Log relative to evidence root |
| --- | --- | --- |
| `make build` | PASS | `pa4-locations-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS | `pa4-locations-preflight.log` |
| `make test-pa4` (required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa4`) | 98/106 PASS; 8 FAIL; checker exits 1, make exits 2 | `pa4-locations-stage.log` |
| `make course-pa4` | 98/106; FAIL, failure marker remains | `pa4-locations-course.log`; `clone/course/pa4/test/{course-result.json,counts.txt,trace.jsonl}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: 104 fixed PA1–PA3 cases and all three course targets | `pa4-locations-prior.log`; `clone/course/pa{1,2,3}/test/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS: 92/92 turn-start identities retained | `pa4-locations-preserve.log` |
| `make lint` (`python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS: rustfmt, strict Clippy, debug/release tests, audit | `pa4-locations-quality.log` |
| `make audit-rust` | PASS: 55 files, zero findings | `pa4-locations-audit.log` |

Turn-start failures: **14/106**. Checkpoint failures: **8/106**. Coverage remains
106 original identities. `pa4-locations-case-summary.json` captures every raw
invocation record and confirms all 98 passing cases have matching status 0/1,
stdout/stderr and complete output bytes; all 92 prior passes remain passes.
Newly passing paths relative to `pa4/tests/directives/`: `300-line-new-line.t`,
`500-predefined-macros.t`, `600-line-macro.t`,
`600-predefined-macro-argument-expansion.t`, `610-line-macro.t`,
`660-line-directive.t`. Example raw successful location trace:
`clone/cases/pa4/51f30f389b5cb98b99a4/trace.jsonl`. Remaining traces still return
explicit status 86 for their named missing inclusion or pragma dependency;
none is claimed as a matching rejection.

Supplemental generated differential reproducer:
`pa4-locations-differential/{check.py,results.json}` and
`pa4-locations-differential.log`. **173/173** match status, stdout, stderr and
complete output bytes. Inputs and outputs are retained there. Coverage includes
malformed/range/extra operands, filename prefixes/raw/suffixed literals,
splices, BOMs, embedded control/non-UTF-8 bytes, nested/multiline arguments,
#line operands demanding builtins, lexer-error precedence and delayed
unterminated-conditional diagnostics using earlier filenames/line numbers.
No oracle or harness defect was identified. Fixtures, manifests and policy
remain unchanged.

## Location-control checkpoint resource observations

Reproducer and raw input/output/time/CSV observations:
`/home/vishvananda/work/private/v5rust/artifacts/pa4-locations-resources/`.
Run `python3 measure.py` there; `summary.txt` records this run. Five
observations per size/tool; all nine workloads have matching complete output
hashes. Median wall milliseconds include launch; RSS is the maximum of five
`/usr/bin/time` observations. These are descriptive measurements collected
alongside validation, not a calibrated C++/Rust performance comparison.

| Supported workload | Scale | Input bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Repeated `ID(__LINE__)` and stringized `__FILE__` | 10,000 rows | 310,058 | 59.573 | 2,200 |
| Same | 50,000 rows | 1,550,058 | 233.829 | 3,612 |
| Same | 100,000 rows | 3,100,058 | 487.387 | 5,716 |
| Repeated `#line` state plus file/line demand | 10,000 states | 458,894 | 29.737 | 3,612 |
| Same | 50,000 states | 2,338,894 | 148.798 | 8,540 |
| Same | 100,000 states | 4,688,895 | 297.112 | 14,872 |
| Nested `ID` builtin argument prescan | 1,024 deep | 4,121 | 4.146 | 2,908 |
| Same | 4,096 deep | 16,409 | 7.524 | 4,316 |
| Same | 16,384 deep | 65,561 | 21.671 | 11,376 |

Time scales with source/expansion work. Memory follows retained source bytes,
physical lines, required line history and invocation depth; produced output is
streamed. The cached file literal shares immutable spelling bytes across
repeated demand. No native-stack failure occurs at 16,384 nested prescans.

## Remaining exact case IDs and next dependency boundary

All paths below are relative to `pa4/tests/directives/`.

- Source frames/inclusion: `200-include.t`, `400-bad-include.t`,
  `400-header-guard-shapes.t`, `400-header-guarded.t`, `600-pragma-op.t`,
  `800-pragma-once.t`. Owners: `source_stack.cpp`, `source_files.cpp`,
  `preprocessor_directives.cpp` includes/guards, and `preprocessor.cpp` once.
- Pragma execution: `500-pragma-ignore.t`, `501-pragma-op-ignore.t`.
  Owners: `run_pragma_directive` and `run_pragma_operator`.

Next coherent boundary: a translation-unit source-file store with compact
file/inclusion IDs and location ranges, private path/host-identity caches,
source frames and per-file conditional depth. Transfer the single-inclusion
physical index/history into those owners, retaining lookup after a file ends.
Then port raw/expanded include-name combination, search relative to the current
presumed name, once/guard skips and their diagnostic include chains. Pragmas
require after-replacement text-sequence operator parsing, pragma destringizing
and relexing, once identity and pack state. All of these must preserve the
existing directive and macro demand boundaries.

Other production responsibilities still pending: one-invocation build-time
capture in `dev/bin/` with typed date/time options, complete deterministic
statistics, include/location-space limits and later hosted probes/variadics.
The date/time definitions explicitly return 86 if demanded; no fixed-manifest
case prints those environmental values. These gaps are not evidence of full
PA4 completion.

Independent audit focus: verify newline/history boundaries and latest versus
historical location queries, binary filename bytes at the driver boundary,
line-operand error precedence and argument-location retention. When frames
arrive, verify every token retains its inclusion range, conditional sections
cannot cross file boundaries and inclusion search uses the presumed name.
This handoff completes the location-control group without advancing the stage
or changing the last-audited commit.
