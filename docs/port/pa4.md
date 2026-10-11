# PA4 port ledger

Stage base commit: e953408ec36e27923018214d0c788e91d2756633
Last audited commit: `a56a656f20d03249cf4d6b4581ece696f0f1f512`

Implementation checkpoints:

- `52ff1e8344d359aaa501d740291fb105f6300df6`: macro group (2026-10-11).
- `5c06195f4c6f7960bcc768092a844871f1eaa2df`: conditional inclusion and controlling-expression rescanning (2026-10-11); entry HEAD `fd2481a` (documentation handoff after the macro implementation).
- `16f0a9779f89f5b82544b1f1085b53593df1f749`: presumed locations, file/line builtin replacement and macro-expanded line control (2026-10-11); entry HEAD `af7984f93d7ba4aedca00014cd7590441667d207`.
- `1fc98f61a249287476c98af449b1ca371e321285`: pragma directives/operators, packing and once registration (2026-10-11); entry HEAD `fbc24e1f77a920deb5b6889c2a62cc11a0216438`; independent review pending.
- File inclusion/source ownership checkpoint (2026-10-11), introduced by this entry's commit; entry HEAD `f37c6f89bec2cdf1a6fce366b4b0b060c460f2f1`; independent review pending.

The three accepted implementation checkpoints are independently audited below,
through the audit fixes in `4359650`, `637b946` and `a56a656`. Ralph retains stage advancement. The
location-control checkpoint reduced fixed failures from 14 to 8; this audit
preserves all 98 accepted passing identities and the same eight failing IDs.
Full PA4 remains incomplete; this certifies only the completed groups.
The subsequent unaudited pragma implementation reaches 100/106. The file
inclusion checkpoint below reaches 106/106; both leave the last-audited marker
unchanged and retain their independent review boundaries.

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

## Independent partial checkpoint audit, 2026-10-11

Audit entry: clean `bd013c2b02dc691d45ad2aa9c789a5131a3f0b63`. Reviewed the
complete diff from stage base `e953408ec36e27923018214d0c788e91d2756633`,
initially the last-audited boundary, through implementation
`a56a656f20d03249cf4d6b4581ece696f0f1f512`. The preceding goal turn was
progress: the accepted location implementation and its evidence are committed.
The turn-start fixed result was independently rerun and retained in
`entry-stage.json`: 98/106 passing and eight failures. The required full-stage
command was run without making unfinished inclusion/pragma behavior a condition
of partial-checkpoint acceptance.

Read AGENTS.md, CLONE_CONTRACT.md, spec.md, conversion plan, PA4 handout and
both detailed guides, stage ledger and source map. Independently consulted the
frozen C++ owners in the ownership table, including their headers, source-frame
token stabilization, definition-parser slot rollback, hide-set intern/index
operations, rescanner context/barrier handling, conditional skipping and
evaluation, line-control decoding/history, post-token writer and CLI handlers.
The oracle revision remains `905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

### Findings and fixes

1. Active `#include_next` incorrectly entered the unfinished inclusion group
   and returned 86. C++ `run_directive` rejects it before reading operands when
   hosted mode is false. Removed it from the active inclusion arm, so it now
   produces the exact located invalid-directive rejection, including after
   `#line`. Inactive `include_next` still requests header-name lexing and skips
   its operands as the C++ owner does. Regression:
   `tests/directives/171-include-next-course-bad.t`.
2. The CLI opened directories, ignored their read failure and accepted them as
   empty primary sources. C++ `SourceFiles::find_or_read` checks regular-file
   metadata before opening. The Rust CLI now does the same, preserving the
   exact cannot-open diagnostic and output already emitted for earlier primary
   sources. Checking before opening also rejects FIFOs without blocking.
   Regression: `tests/directives/172-directory-source-bad.t`, with a committed
   directory companion containing only a marker.
3. `std::env::args` panicked on valid non-UTF-8 source/output path arguments,
   while C++ processes their bytes. The entry point now collects `OsString`
   arguments and preserves bytes in `sof`, presumed names and path diagnostics.
   Regression: `tests/directives/173-source-path-bytes-bad.t`, whose companion
   filename ends with byte FF. The bin unit test additionally checks successful
   output and located failure for that path, plus directory rejection. No lossy
   conversion or exception-to-rejection wrapper was introduced.
4. The no-runner batch path decoded stdin as UTF-8 and stopped on a non-UTF-8
   request, whereas C++ getline consumes bytes and emits one sentinel per line.
   Replaced the string-line iterator with a byte-line iterator, retaining final
   unterminated lines, empty requests and ignored output failures. A bin unit
   regression and five direct CLI probes cover this interface. The sentinel
   remains a no-runner response, never a successful compiler invocation.
5. The Rust definition parser rejected 65,535 named parameters followed by
   ellipsis. C++ accepts that definition but stores its count and the variadic
   slot in uint16_t, wrapping them to zero. Preserve that explicit target
   truncation in arity, role assignment and redefinition comparisons, while
   retaining the full immutable declared parameter list. Argument substitution
   now checks its ranges, so malformed wrapped roles cannot panic. Regression:
   `tests/macros/930-parameter-count-wrap.t`, including redefinition with different
   declared names that compare equal under C++'s zero effective count.
6. Four source-map descriptions still called implemented NameId, retained
   spelling, file/line builtin and expanded-expression cursor facts deferred.
   Updated those responsibilities while retaining their remaining partial
   status. No pending hosted/file-store responsibility is marked complete.

The four new fixture families live in PA4's existing executed macro/directive
lanes.
Their new expected observations are checked against complete oracle-equal output
bytes, and their focused protected checker invocations compare status, stdout,
stderr and output bytes even for rejection. No frozen fixture/reference,
manifest, gate policy, dependency or Ralph state was changed.

### Completed-group completeness and ownership review

| Reviewed group | Behavioral and architecture conclusions |
| --- | --- |
| Definition grammar and publication | All course object/function/variadic forms, parameter diagnostics, whitespace requirements, reserved-name rejection, role assignment and exact redefinition checks correspond to the C++ branches. Local parameter-slot maps disappear on failure; they cannot pollute later definitions. A definition is published only after parse/role analysis and redefinition checks succeed. Immutable Rc definitions are shared by active contexts; undef/redefinition cannot invalidate live references. |
| Rescanning and substitution | Traced object/function invocation, lookahead popping, collected/nested argument links, arity checks, prescan demand, argument barriers and completion marks. Only plain parameter uses demand prescan, once per argument; raw/stringized operands do not. Hide-set additions, ordered unions, last-add removal, paint and origin/disabled/inherited facts match the course recursion rule. Source-following tokens can complete an invocation after a replacement context ends. Stringizing, placemarkers, adjacent paste operators, variadic comma paste and invalid-paste diagnostics match the C++ roles. No recursion/output cutoff or fixture shortcut exists. |
| Conditional inclusion | Active stacks and the flat inactive nesting stack preserve order checks, taken-section operand skipping, extra-token and unterminated diagnostics. `defined` and attribute probes consume raw operands inside controlling expressions, including during argument prescan; they query compact definition slots and preserve C++ diagnostic precedence. Macro output feeds the shared PA3 evaluator directly. Its generic error change leaves arithmetic and reduction algorithms untouched; earlier PA3 checks and supplemental arithmetic/error cases pass. |
| Single-file locations | Physical line starts are built lazily once, with compact byte offsets and binary search. Checked decimal line accumulation enforces 1..=2147483647 with the C++ error order. Ordinary filename strings use the shared phase-5 decoder. Builtins use invocation offsets and latest state; diagnostics use the historical state, with the same exclusive `until` boundary. Shared filename literals are invalidated only when a filename operand changes state. BOM columns, splices, binary/control filename bytes and delayed diagnostics are covered. |
| Typed boundaries and lifetimes | Immutable source bytes and shallow shared spellings feed Token -> Expander -> PostTokenInput/PpTokenCursor -> PA2/PA3 consumers. No serialized phase transport, second parser, whole-unit token vector or source-character record was added. Context tokens, argument tokens/links and expanded tokens are flat LIFO stores truncated at their owning context/invocation boundary; line tokens clear after success. Canonical names and immutable hide-set caches remain unit-owned. Failed input is terminal: the driver renders its error while the input owner lives, then drops the unit; no speculative state is reused across primary sources. Existing invariant tests prove deep-stack behavior and empty live buffers at EOF. |
| Determinism and implementation policy | NameId-indexed slots supply direct lookup. Hash maps are lookup indexes, never emission order; token/context/source order drives output. Clone calls copy ranges, small token facts or Rc references, not owning token trees/sets. No global registry scan, unsafe code, FFI, implementation subprocess, oracle delegation, new dependency or environment read under dev/src exists on the reviewed path. The sole new rescanner lint expectation is narrowly justified for its cohesive expansion inputs; the independent file audit passes. Responsibility modules are semantic owners, not mechanical file splits. Production tool names and Rust identifier conventions remain compatible. Backend, optimizer and register allocator are untouched. |

All candidate defects above are fixed. The C++ parameter-limit fault described
below is a reported frozen-baseline issue. Active file inclusion, pragma
execution, build-time values, counters and hosted probes retain their previously
documented unfinished status. Per-file conditional limits and inclusion location
identity remain requirements of the next file-store/frame boundary, not claims
of this single-file audit. PA34 source/object provenance checks are not applicable
to this PA4 checkpoint; no bootstrap or object substitution occurred.

### Fresh validation evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa4-audit/`.
Validated implementation: `a56a656f20d03249cf4d6b4581ece696f0f1f512`.
Release `preproc` SHA-256:
`209a6bb1a83bc6cf85ef48885843a362b976127127f6589f191bd5d5eebf3b39`.
Protected manifest digest remains
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.

| Actual command/check | Result and evidence relative to root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa4` | 98/106; exit 1 for the same eight unfinished cases; `stage.log`, `pa4-result.json` |
| `make course-pa4` | 102/110, including all four new fixture families; exit 2, failure marker retained; `course.log`, `pa4-course/{course-result.json,counts.txt,trace.jsonl}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: PA1 58/58, PA2 26/26, PA3 20/20 and all three full course targets; `prior.log`, `pa{1,2,3}-result.json`, `pa{1,2,3}-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS 98/98; `preserve.log` |
| Fresh entry/current identity comparison and raw-trace inspection | PASS: same 106 identities, same eight failures, all 98 passes have exact oracle status 0/1 and complete output equality; `entry-stage.json`, `summary.json` |
| `make lint` (protected `clone_quality.py`) | PASS: rustfmt, strict all-target/all-feature Clippy, debug/release tests including both CLI regressions and wrapped-parameter safety, independent audit; `quality.log` |
| `make audit-rust` | PASS: 55 files, zero findings; `audit.log` |
| Focused protected `execute_case` for the four added PA4 fixtures | PASS 4/4, one matching invocation each; `regressions.json` records raw trace/log paths |

`probe.py` retains seeded inputs and raw observations in `probe/`, covering
850 small macro graphs, 42 replacement/expression/line contexts, 16
active/inactive directive cases, four path/file probes and five batch probes. Before fixes,
`probe-before.log` records 906/911 matches: the five mismatches correspond to
the three defects above. The separate `batch-before.json` records the later observed byte-request
mismatch. After all fixes and adding FIFO/batch probes, `probe-after.log` and
`probe-results.json` record **917/917** identical status, stdout, stderr and
complete output bytes (batch probes have no output file).

The earlier location/conditional generators were copied into this audit's
artifact root to preserve historical evidence. `locations-differential/check.py`
passes **173/173** (`locations-differential.log`). The full conditional generator
reports **359/362** (`conditionals-differential.log` and its `results.json`).
Its only mismatches are `case-0246.cpp`, `case-0259.cpp`, `case-0298.cpp`: active
`#include <bad\q>` under respectively `#if 1`, `#ifdef X`, `#ifndef absent`
reaches the explicitly unimplemented inclusion group (86 versus C++'s missing
include rejection). Those inputs require the next dependency; the other 359
observations, including inactive include header lexing, match exactly. These
three results are recorded as failures, not silently dropped or called passes.

The fixed failing IDs remain exactly the eight listed in the preceding section:
`200-include.t`, `400-bad-include.t`, `400-header-guard-shapes.t`,
`400-header-guarded.t`, `500-pragma-ignore.t`, `501-pragma-op-ignore.t`,
`600-pragma-op.t`, `800-pragma-once.t` under `tests/directives/`.
No added pass compensates for a lost accepted identity or new fixed failure.

`parameter-limit/check.py` separately probes the target parameter-count boundary.
`parameter-limit/before.json` retains the original Rust rejection of the empty
wrapped definition. After the fix, all five defined oracle observations match:
65,534 parameters plus ellipsis, 65,535 ordinary parameters, the empty wrapped
variadic definition, rejection of 65,536 ordinary parameters, and expansion of
wrapped `__VA_ARGS__` as an ordinary identifier. The sixth probe invokes a
wrapped definition whose body uses `p0`: C++ crashes with SIGSEGV (-11), while
Rust returns a structured representation-limit error (1), with no panic.
`parameter-limit.log`, inputs, output bytes and stderr retain that evidence;
core dumps were disabled for the deliberate oracle fault probe. The C++ parser
wraps the count to zero, then indexes absent expanded-argument bounds for `p0`.
This is a frozen-baseline invalid-access defect, reported as CLONE_CONTRACT.md
requires; neither the C++ owner nor a reference/policy was patched, and the
oracle crash is not claimed as a matching rejection. The Rust-only invariant
test verifies this unsafe oracle path is handled through checked typed access.

### Audit resource observations and handoff

`python3 measure.py` records three runs per tool/workload, raw `/usr/bin/time`
observations, retained source/output bytes and matching output SHA-256 values
in `resources/`; `resources.log` and `resources/results.json` retain this run.
Median wall time includes process launch; RSS is the maximum of three runs.
All seven workloads have equal complete output hashes across both tools.

| Supported workload | Scale | Input bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Nested argument prescan | 1,024 | 4,115 | 3.906 | 2,908 |
| Same | 4,096 | 16,403 | 7.457 | 4,348 |
| Same | 16,384 | 65,555 | 23.057 | 11,344 |
| Nested inactive sections | 10,000 | 270,016 | 12.792 | 2,236 |
| Same | 50,000 | 1,350,016 | 54.286 | 3,604 |
| Repeated line state and builtin demand | 10,000 | 310,000 | 27.312 | 2,908 |
| Same | 50,000 | 1,550,000 | 125.959 | 7,132 |

The measurements are descriptive, not a calibrated performance claim. They
support the reviewed bounds: flat nesting storage, streamed output and time
following input/required expansion work. Source and scratch share the stated
100-GiB budget; observed filesystem use was about 5.2 GB with about 99.9 GB
available before these small audit artifacts.

The reviewed partial checkpoint is accepted. The last-audited marker names the
reviewed implementation/fix commit, followed only by this audit documentation.
Next work remains the file-store/source-frame/inclusion boundary described
above, followed by pragmas and other outstanding PA4 responsibilities. No claim
of complete PA4 behavior or stage advancement is made.

## Pragma implementation checkpoint (2026-10-11)

Validated implementation: `1fc98f61a249287476c98af449b1ca371e321285`.
Entry HEAD: `fbc24e1f77a920deb5b6889c2a62cc11a0216438`.
The stage base and last-audited commit at the top remain unchanged. This is an
implementation handoff for independent review, not a new audit or stage
advancement. Fixed-manifest progress is **98/106 -> 100/106**: eight failures
become six, with the same 106 case identities and all 98 entry passes retained.
The two gained IDs are `tests/directives/500-pragma-ignore.t` and
`tests/directives/501-pragma-op-ignore.t`. The complete course lane is 104/110;
its six failures and failure marker remain visible.

### Owners, behavior and architecture

Consulted the frozen C++ `dev/src/preprocess/preprocessor.cpp` functions `next`,
`run_pragma_operator`, `run_pragma` and `mark_once`,
`preprocessor_directives.cpp` functions `run_pragma_directive`,
`run_pack_pragma` and `apply_pack_pragma`, the `preprocessor.h` packing contract,
`source/source_files.cpp` function `GetFileIdentity`, and the shared token-text
lexer and literal-boundary owners. All paths refer to the recorded frozen
checkout, not candidate C++ code.

Rust responsibilities:

- `dev/src/preprocess/pragmas.rs`: raw directive dispatch; post-rescan operator
  grammar and removal; literal prefix/raw-body handling; destringizing only
  escaped quote/backslash; shared phase-3 token-text lexer; ignored unknown
  pragmas and the C++ lexical-error boundary; exact packing and once registration.
- `dev/src/preprocess/files.rs`: private device/inode identity values and a
  typed demand-only host query. `dev/src/driver/file_identity.rs` supplies the
  safe standard-library stat equivalent, following symlinks, accepting only
  regular files and respecting the frozen C-string NUL boundary.
- `unit.rs`: owns the pragma state and queries it at text demand; exposes the
  current packing value to future parser consumers. `input.rs` and
  `locations.rs` expose the latest presumed name without rendering/reparsing a
  file literal. `driver/preproc.rs` supplies the host identity service and
  creates fresh unit state for every primary file. In-memory construction has
  no host file identity; the file tool uses the explicit file-backed constructor.

Packing recognizes loose operand syntax, push/pop/show flags and the last
pp-number's decimal prefix, including explicit uint32 wrapping. Push and pop
happen in C++ order before invalid alignment is ignored; `show` changes nothing.
Directive operands are not expanded. Small typed operand facts are accumulated
locally and applied only after the complete operand run lexes successfully.
This preserves the C++ delayed state mutation without owning a second token
vector. Operator text uses `for_token_text` and disables phases 1/2, exactly as
C++; embedded newlines and raw bodies keep their original behavior. It uses the
shared literal splitter, not phase-5 escape decoding.

Operators execute only after rescanning, never during argument prescan,
stringizing, unused argument handling, controlling expressions or line control.
Three expanded operands are demanded at a text boundary; malformed invocations
report the operator's location, while upstream lexical/macro errors preserve
precedence. Repeated substituted operators execute separately. The removed
operator tokens do not interrupt post-token string concatenation. The temporary
text buffer and lexer die at operator completion. Packing/once stores die with
the unit; names, macro tokens and definition ownership remain unchanged. Hash
iteration does not determine output. Identity lookup occurs only for executed
once pragmas. No new dependencies, unsafe code, environment reads in phases,
compiler subprocesses, fixture recognition, backend changes or optimizations
were introduced. The independent audit reports zero findings with no new lint
suppression.

Rust invariant tests cover observable packing at each token-demand boundary,
argument-prescan-once versus repeated operator execution, emptied expansion
storage, wrap/last-number/push/pop/show behavior, atomic lexical failure, latest
presumed-name registration, identity aliases/NUL handling/nonregular rejection
and unit lifetime. Frozen fixtures and protected policy were not changed.

### Validation and retained evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa4-pragmas/`.
Release `preproc` SHA-256:
`a73bcecebdadbbecc508a8eed2f2742cbc1c7635b6d2cef7dcd309088cb6d5af`.
Manifest digest remains
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.

| Command/check | Result and evidence relative to root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS; `preflight.log` |
| `make test-pa4` (required `clone_check.py stage pa4`) | 100/106; checker exit 1, make exit 2 for six inclusions; `stage.log`, `pa4-result.json`, copied raw `cases/` |
| `make course-pa4` | 104/110; make exit 2, failure marker retained; `course.log`, copied `pa4-course/{course-result.json,counts.txt,trace.jsonl,mismatches/}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: PA1 58/58, PA2 26/26, PA3 20/20, all full course targets; `prior.log`, copied `pa{1,2,3}-result.json` and `pa{1,2,3}-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS 98/98; `preserve.log` |
| Fresh entry/current identity and raw-trace comparison | PASS: unchanged 106 identities, 98 retained, two gained; all 100 passes match status 0/1 and complete bytes; `entry-stage.json`, `summary.json` |
| `make lint` (`clone_quality.py`) | PASS: rustfmt, strict all-target/all-feature Clippy, debug/release tests and audit; `quality.log` |
| `make audit-rust` | PASS: 58 files, zero findings; `audit.log` |
| `python3 probe.py` | 549/549 equal status/stdout/stderr/complete output; `probe.log`, `probe-results.json`, raw inputs and observations in `probe/` |
| `python3 pack-check.py` | 4,103/4,103 equal token-by-token packing observations; `pack-results.json`, `pack-sequence.cpp`, `pack-{cpp,rust}.{stdout,stderr}` |

The packing probe uses scratch views linked against the unchanged frozen C++
objects and the Rust library respectively. `pack-view.cpp`, `pack-view.rs` and
`pack-view-build-command.json` retain the views and C++ link inputs. These are
external validation tools, never implementations invoked by the compiler.
They exercise directive/operator/raw/macro forms, repeated arguments, loose
syntax, decimal suffixes/wrap, invalid alignment and lexical failure. The
549 complete-tool probes additionally cover literal prefixes, malformed
operators, macro-produced operands, inactive/controlling/stringized/unused
contexts, location diagnostics, comments and string concatenation. No new
oracle or harness defect was observed.

### Compile-time/RSS/scaling observations

`python3 measure.py` retains three observations per tool/workload, raw
`/usr/bin/time` data, source and complete output hashes in `resources/`;
`resources.log` and `resources/results.json` record the results. Every workload
has identical C++/Rust output hashes. Median wall milliseconds include launch;
RSS is the maximum of three runs. These describe the supported preprocessing
paths and impose no generated-code optimization target.

| Workload | Scale | Input bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| Ignored operators and emitted tokens | 10,000 | 270,000 | 17.838 | 2,228 |
| Same | 50,000 | 1,350,000 | 78.445 | 3,636 |
| Same | 100,000 | 2,700,000 | 147.699 | 4,340 |
| Pack push/pop operator pairs | 10,000 | 560,000 | 34.155 | 2,932 |
| Same | 50,000 | 2,800,000 | 162.083 | 5,044 |
| Same | 100,000 | 5,600,000 | 313.403 | 7,848 |
| Operands in one pack directive | 10,000 | 80,024 | 6.699 | 2,228 |
| Same | 50,000 | 400,024 | 21.860 | 2,224 |
| Same | 100,000 | 800,024 | 38.390 | 2,928 |
| Repeated once registration | 10,000 | 130,008 | 49.136 | 2,228 |
| Same | 50,000 | 650,008 | 221.803 | 2,932 |
| Same | 100,000 | 1,300,008 | 440.949 | 3,632 |

Storage follows retained source bytes, active expansion depth, pack stack depth
and distinct registered identities; pack operands retain constant-size facts.
Time scales with required tokens/operators/host queries. The shared filesystem
used 5.34 GB of its approximately 105.09 GB capacity after these small artifacts,
within the stated 100-GiB ceiling.

### Next dependency boundary and exact remaining IDs

The coherent pragma-execution group is implemented; file inclusion is still
explicitly missing. Once registration is present, but reentry skipping and
multi-file identity/location behavior require the file store and source-frame
stack. The exact fixed failures, all under `tests/directives/`, are:
`200-include.t`, `400-bad-include.t`, `400-header-guard-shapes.t`,
`400-header-guarded.t`, `600-pragma-op.t`, `800-pragma-once.t`.
All six raw traces show candidate status 86 at `file inclusion` versus oracle
status 0 (or status 1 for `400-bad-include.t`); none is called a matching rejection.

Next port `source_files.{h,cpp}` and `source_stack.{h,cpp}` ownership, compact
file/inclusion IDs and global location ranges, immutable cached source images,
include operand/search semantics, per-frame conditional depth, guard tracking,
once skips and inclusion-stack diagnostics. Reuse the typed host identity
service and pragma state. Build-time predefined values, telemetry and later
hosted environments/probes remain separately documented unfinished groups.
The implementation does not update the last-audited marker, certify full PA4,
or advance Ralph's stage.

## File inclusion, source frames and header reentry checkpoint (2026-10-11)

Entry HEAD: `f37c6f89bec2cdf1a6fce366b4b0b060c460f2f1`.
Stage base remains `e953408ec36e27923018214d0c788e91d2756633`; last audited
commit remains `a56a656f20d03249cf4d6b4581ece696f0f1f512`.
The implementation commit is the commit introducing this checkpoint entry.
Independent review and stage advancement belong to Ralph.

This coherent group reduces the fixed manifest from six failures to zero:
100/106 -> 106/106, preserving all 100 entry passing identities and all 106
case identities. The gained IDs, all in `tests/directives/`, are
`200-include.t`, `400-bad-include.t`, `400-header-guard-shapes.t`,
`400-header-guarded.t`, `600-pragma-op.t`, `800-pragma-once.t`.
There are no remaining failing fixed-manifest IDs. This is implementation
and validation evidence for the group, not certification of complete PA4:
build-time predefined values and the statistics interface are still missing.

### C++ owners and typed data flow

Read the frozen `preprocessor_directives.cpp::run_include`,
`read_replaced_include_name`, `find_include`, `include_file`, `track_guard`,
`run_conditional`, `innermost_conditional` and `skip_group_directive`;
`preprocessor.cpp::leaving_file`, `describe`, `presumed_line` and
`presumed_file_literal`; `source/source_files.{h,cpp}`,
`source/source_stack.{h,cpp}` and `source/source_buffer.cpp::ReadFrom`.
The surrounding handouts, contract, specification and conversion plan were
reread for this checkpoint.

| Responsibility | Rust owner and invariants |
| --- | --- |
| Physical file identity and storage | `preprocess/files.rs`: private compact `FileId`, dense immutable image store, path hit/miss cache, device/inode alias cache and optional guard `NameId`. One read per successfully opened identity; cache traversal never orders output. |
| Host file boundary | `driver/file_identity.rs`: regular-file stat, byte paths and C-string termination, bulk reading with checked reservation for an appended newline, retention of bytes delivered before a read failure. `FileServices` explicitly supplies identity/read functions; compiler phases perform no platform or environment queries. |
| Source image and physical index | `support/source/mod.rs`, `lex/pp_lexer/{mod,literals}.rs`: borrowed primary or shared immutable header image, one lazily built physical line index per image. Lexers own handles and use temporary translation-reader views, avoiding self-referential storage. Clean header spellings retain an image range; translated text retains the existing shared spelling. No per-token image allocation or image copy occurs. |
| Include operand grammar | `preprocess/inclusion.rs`: direct header-name bypasses replacement and requires line end; other tokens use the same line-mode rescanner, then ordinary-string decoding or the C++ angle-token spelling/whitespace combination. Empty/malformed/extra-token diagnostics and demand precedence are preserved. Transient operands are cleared after decoding. |
| File search and reentry | `preprocess/input.rs`: current presumed-name directory then working-directory search for both course delimiter forms. The found path becomes the included filename. Once identity and defined guard checks precede the exact depth limit (current depth > 200). No canonicalized path replaces the observable spelling. |
| Source frames and locations | `preprocess/input.rs`, `preprocess/locations.rs`: private compact `InclusionId`, checked 32-bit nonoverlapping location ranges, dense inclusion records with parent/include-directive edges and independent presumed-name/delta histories. Active lexer frames are bounded by inclusion depth and released on file return; retained inclusion facts support later diagnostics and macro tokens. |
| Conditional file boundaries and guards | `preprocess/conditionals.rs`, `preprocess/input.rs`, `preprocess/unit.rs`: each frame records entry depth and a copied optional open-conditional location from the owning conditional stack. A header cannot close a parent's section; an unclosed section fails before popping its frame. Start/open/closed/none guard states follow the C++ active and resumed-skipped directive transitions, and only a wholly closed guard is recorded on return. Undefinition permits reentry. |
| Builtins and diagnostics | `preprocess/macros/expander.rs`, `preprocess/input.rs`: file/line lookup follows the token's owning inclusion even after lookahead leaves that file. Source errors use the active physical image; directive errors use the global range. Diagnostic history and innermost-first `note: included from` lines preserve C++ names, lines, columns and ordering. |

Header EOF continues into its includer's text without a synthetic expansion
boundary; a directive remains a boundary. Macros defined by headers retain
stable source spellings after the lexer frame is released. Repeated headers
share their image and physical line index; repeated entries have distinct
locations and presumed histories. Source images, macro definitions, inclusion
records, names, pragmas and file caches all die with their translation unit;
no whole-unit token sequence is materialized. No backend, optimizer, target
code generation, dependency, unsafe code, fallback compiler, fixture,
reference or checker policy was changed.

Rust invariant tests exercise cached misses/alias reads/shared physical line
indices, header-definition survival, actual guard reentry skipping followed by
undefinition, macro invocation through header EOF, token-owned builtin
locations, conditional-file rejection diagnostics and emptied expansion
storage. Existing pragma/line/macro tests remain green. The file audit reports
zero findings and no new lint suppressions.

### Validation and retained artifacts

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa4-inclusion/`.
`entry-head.txt` and `entry-stage.json` preserve entry state; `summary.json`
records the final binary hashes, unchanged manifest digest, exact identity
comparison, gained cases and raw invocation verification.

| Command/check | Result; evidence relative to root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS; `preflight.log` |
| `make test-pa4` / required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa4` | PASS 106/106, exit 0; `stage.log`, `pa4-result.json`, copied raw `cases/` |
| `make course-pa4` | PASS 110/110, exit 0 and no failure marker; `course.log`, `pa4-course/{course-result.json,counts.txt,trace.jsonl,mismatches/}` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS PA1 58/58, PA2 26/26, PA3 20/20 and all full course targets; `prior.log`, copied `pa{1,2,3}-result.json` and `pa{1,2,3}-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS 100/100; `preserve.log` |
| Entry/current identities and every raw PA4 invocation | PASS: same fixed 106 identities/digest; all prior passes retained; six gained; every invocation has exact ordinary status and bytes; `summary.json` |
| `make lint` (`clone_quality.py`) | PASS rustfmt, strict Clippy, debug/release tests and file audit; `quality.log` |
| `make audit-rust` | PASS 59 files, zero findings; `audit.log` |
| `python3 probe.py` | PASS 202/202 equal status/stdout/stderr/complete output; `probe.log`, `probe-results.json`, source/header/alias inputs and raw observations in `probe/` |

Differential probes cover direct and replaced operands, string decoding,
whitespace in expanded angle names, invalid literals, extra tokens, NUL and
UTF-8 paths, directories/missing files, recursive depth failure, active/inactive
conditionals and lexical errors in headers, nested inclusion notes, guard
shape/invalidation/undefinition, hard/symbolic aliases, once pragmas through
line control, multiple primary-source reset, retained macro spellings,
macro argument collection across file return, and parent location restoration.
No new oracle/harness defect was observed. These scratch probes supplement
and do not change any frozen lane.

### Compile-time/RSS/scaling observations

`python3 measure.py` records three observations per tool/workload with raw
`/usr/bin/time` data and complete output hashes in `resources/`;
`resources.log` and `resources/results.json` retain the results.
Every corresponding C++/Rust output hash is equal. Wall time includes launch;
RSS is the maximum of three runs. Large guarded/once headers contain 1,000
output lines but are read/lexed only on their first entry. Unguarded headers
emit 16 literals plus file/line builtins per entry, exercising repeated location
queries against the shared physical index.

| Workload | Scale | Primary bytes | Header bytes | Rust ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| guarded | 10,000 | 200,000 | 67,035 | 23.088 | 2,188 |
| guarded | 50,000 | 1,000,000 | 67,035 | 63.451 | 2,892 |
| guarded | 100,000 | 2,000,000 | 67,035 | 125.498 | 4,304 |
| once | 10,000 | 200,000 | 67,013 | 18.128 | 2,192 |
| once | 50,000 | 1,000,000 | 67,013 | 57.251 | 2,904 |
| once | 100,000 | 2,000,000 | 67,013 | 101.636 | 4,308 |
| unguarded-locations | 1,000 | 20,000 | 67 | 10.933 | 2,188 |
| unguarded-locations | 5,000 | 100,000 | 67 | 49.174 | 2,936 |
| unguarded-locations | 10,000 | 200,000 | 67 | 81.240 | 3,600 |

The guarded and once paths retain one physical header and one header inclusion;
additional directives hit the path/identity and skip facts. Unguarded reentry
retains one image/line index and one small location-history record per entry;
active lexer frames follow nesting depth. Produced-token work remains linear
in required output. These measurements impose no PA35 generated-code target.
The shared filesystem remains within the 100-GiB storage ceiling; raw capacity
and usage are retained in `storage.txt`.

### Next dependency boundary

The course inclusion group, including guard and once reentry and exact
included-file diagnostics, is implemented with zero fixed-manifest failures.
Next handle the remaining PA4 tool/environment interfaces as an independently
reviewable group: typed once-per-invocation author/date/time inputs for builtin
replacement (`preprocessor.cpp::PredefinedText`, `preproc.cpp::SetBuildTime`)
and exact preprocessing/expansion counters and `--stats` rendering. These
currently return missing-implementation status 86 on demand; fixture coverage
does not certify them. Hosted search paths, include-next, feature probes and
host environment/dialect extensions remain later-stage responsibilities.
Independent pragma/inclusion architecture review and Ralph's checkpoint/final
audits decide acceptance and advancement; this implementation leaves the
last-audited marker unchanged.

## Full-stage acceptance audit and remaining comparator finding (2026-10-11)

Entry commit: `dfe3e9d`, the inclusion/source-ownership checkpoint. The
previous goal turn is classified as progress: that commit implemented the six
remaining inclusion cases and retained every earlier passing identity.
Stage base remains `e953408ec36e27923018214d0c788e91d2756633`.
This audit independently reviews the pragma checkpoint `1fc98f6`, inclusion
checkpoint `dfe3e9d`, the previously audited macro/conditional/location groups,
and the implementation changes introduced with this entry. Historical
incomplete-group statements above describe their respective checkpoints.

**Full stage acceptance remains open.** The fixed gate has zero failures,
and the original full course suite and earlier stages pass. The audit found
and repaired two required interfaces that those fixtures did not demand:
clock-valued predefined macros and statistics. An additional host-clock defect
found during the repair is also fixed. The protected differential adapter still
rejects a statistics invocation solely for an implementation-specific storage
measurement; its policy needs an external disposition before full acceptance.
No protected checker, oracle, existing reference, manifest or Ralph state was
changed. The existing goal remains active; this entry does not advance PA4.

### Findings and fixes beyond fixture coverage

1. `__DATE__` and `__TIME__` previously returned status 86 when demanded.
   `PredefinedValues` now supplies owned author/date/time bytes. The entry point
   captures the local time once for the entire invocation, and every primary
   source receives the same values while all unit-local state resets. The
   author remains exactly `Course Solution` for the standalone tool.
2. Predefined definitions were manually installed, so their source/directive
   work was absent. C++ reads `PredefinedText` as a synthetic source before
   the primary file. Rust now does the same through its ordinary directive
   parser: one immutable `<predefined>` image, an independent source range,
   and no parent inclusion note. Escaping, definition roles, errors, demand
   and reserved-name treatment use the existing phase owners. There is no
   rendered-token transport or second parse of compiled source.
3. `--stats` previously returned status 86. Unit-owned counters now observe
   stabilization, active and skipped directives, physical files/bytes, actual
   include entries and guard skips, and macro work at the same C++ transitions.
   Ordinary builtin replacements and `defined` do not count as macro
   invocations. Argument barriers count prescans; range-only nested collection
   does not copy/count argument tokens; placemarker paste does not count a
   paste. Peaks are sampled where C++ samples them, not at every intermediate
   push. Unsigned telemetry totals wrap explicitly. Snapshot/reporting occurs
   after successful work and after preprocessing/phase-7 failures; diagnostics
   precede statistics. Missing primary files have no initialized-unit report.
4. An initial host `date` query failed the POSIX `TZ=EST5EDT` probe: this host
   ships uutils date, which returned UTC while libc returned daylight Eastern
   time. The final clock boundary invokes the existing host Python interpreter's
   standard `time.localtime`/`time.asctime` interface in isolated, no-site mode.
   It supplies only environmental clock bytes, through the same substring
   rules as C++ `SetBuildTime`; all preprocessing and literal conversion run
   in Rust. No dependency was added to Cargo, no authored unsafe/FFI exists,
   and no C++ executable implements a candidate phase. The interpreter path
   and binary digest are recorded as host inputs. Six timezone pairs now
   produce identical complete output with both tools called in the same
   recorded wall-clock second. A separately retained pair crossing a second
   differs by exactly that environmental time input, with each invocation
   keeping its values constant across its two primary sources.

Required regressions supplement the executed PA4 directive lane:
`tests/directives/600-build-time-values.t` and `.t2` demand the clock values
through post-rescan pragmas without printing volatile values; and
`tests/directives/400-line-date-value.t` demands the date while rejecting an
invalid line operand. Their new fixed expectations were checked against the
unchanged oracle. An invariant test supplies fixed escaped author/date/time
values, drops the input options, verifies retained literal ownership and
lazy predefinition parsing, and checks the resulting work counts and empty
expansion storage. No existing fixture or expected output was modified.

### Responsibility and architecture review

The full handout (`README.md`, `macros.md`, `directives.md`), clone contract,
specification, conversion plan, source map and stage history were consulted.
The directly reviewed frozen C++ owners are `preproc.cpp`,
`preprocessor{,_directives}.cpp`, `preprocessor.h`,
`preprocessor_names.{h,cpp}`, `source/source_files.{h,cpp}`,
`source/source_stack.{h,cpp}`, `macro/macro_definition.{h,cpp}`,
`macro/macro_expander.{h,cpp}`, `macro/hide_sets.{h,cpp}` and shared
source/lexer/post-token/PA3 owners. The prior independent numeric audit is
preserved, rather than replacing target arithmetic with host arithmetic.

| Stage-owned responsibility | Review conclusion and Rust owner |
| --- | --- |
| CLI, writer, primary-source reset, non-runner batch protocol | `dev/bin/preproc.rs`, `driver/preproc.rs`, `driver/posttoken.rs`: exact file/path bytes, status and output failure behavior; documented non-runner batch response retained. Course routing selects the Rust executable for all due preprocessing. |
| Predefined constants, clock values, reserved names | `preprocess/options.rs`, `unit.rs`, `macros/definition.rs`: synthetic definitions parsed once on demand; reserved names permitted only in that private physical source; builtin identity and macro slots remain typed. |
| Definition/argument grammar, redefinition and parameter limits | `macros/definition.rs`: whitespace comparison and diagnostic order match; explicit uint16 parameter-count behavior retained. No shared mutable parameter scratch escapes a parse. |
| Rescanning, recursion, stringizing and pasting | `macros/expander.rs`, `token.rs`, `hide_sets.rs`: token-local paint/origin rules, argument demand and prescan-once facts retained; ordered hide-set additions and complete `(set, other)` cache keys. Trie recursion is bounded by identifier bits. |
| Conditional nesting, skipped demand and PA3 expressions | `conditionals.rs`, shared `expression.rs`/`arithmetic.rs`: flat stacks, exact nested ordering and short-circuit behavior, raw defined/attribute operators, per-file conditional boundaries and earliest failures. |
| Include grammar/search/cache and header reentry | `inclusion.rs`, `files.rs`, `input.rs`: direct-header versus expanded operands, presumed-path then working-directory order, regular stat identity, cached misses/aliases, once before guard before depth checks; only guard skips increment guarded-skips. |
| Locations, line control and diagnostics | `locations.rs`, `line_control.rs`, `input.rs`: lazy physical line index shared per image, per-inclusion history, token-owned invocation locations, checked global ranges, original byte columns and innermost-first notes. |
| Pragmas, `_Pragma` and pack state | `pragmas.rs`, `unit.rs`: operators execute after replacement; exact destringizing, ignored unknown/ill-formed pragma text, transactional pack updates, push/pop/show precedence, decimal-prefix uint32 wrap and presumed-name once identity. |
| Phase 5–7 and view integration | Shared `lex` and `PostTokenInput`: typed pull composition, string concatenation through removed operators/includes, exact literal/invalid-token records and errors. |
| Deterministic work and resource observations | `counters.rs` plus the owning phase stores; CLI reporting retains field names/order. Allocation storage is explicitly separate from work; the Rust implementation has no bump arena, so `arena_bytes=0`. RSS/scaling are measured independently below. |

Source frames and macro contexts retain bounded lookahead and active nesting,
not whole-unit token vectors. Each physical file owns one immutable image;
headers retain stable spellings after frame release. Compact name, hide-set,
file, inclusion, location and context identities stay within their private
owners. Definition roles and argument links are computed once. Maps never
supply observable order. Caches are translation-unit-owned and keyed by full
path, stat identity or ordered hide-set operands, with macro-defined state
checked again at inclusion. Owned option bytes cannot dangle or be reread from
the environment in a phase. Packing operands publish no partial update after a
lexical failure. Structured Results preserve ordinary errors, limits and
internal invariant failures distinctly. No new lint expectation was needed.
Parser rollback, semantic/template caches, per-function backend storage,
optimizer policies and PA33 implementation-wide census/debug requirements
belong to later stages and are not certified by this PA4 review. No backend or
optimizer code was changed, and no PA35 optimization objective was imposed.

### Current validation and raw evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa4-full-audit/`.
`entry-head.txt` records entry state; `validation.json` records every command
and exit status. `summary.json` records final binary/manifest/host-input
hashes, exact fixed case identities and inspection of all 106 PA4 invocation
traces. `paN-result.json`, copied `cases/`, and `paN-course/` retain raw
results rather than relying on aggregate fixture counts.

| Command/check | Result and artifact relative to evidence root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa4` | PASS; `preflight.log`; unchanged protected inputs and oracle hashes |
| Required `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa4` | PASS 106/106; `stage.log`, `pa4-result.json`, `cases/`; every raw invocation has equal ordinary status and externally visible bytes |
| `make course-pa4` / `clone_check.py course pa4` | PASS 112/112; `course.log`, `pa4-course/{course-result.json,counts.txt,trace.jsonl}`; no failure marker or failed comparison |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa4` | PASS: PA1 58/58, PA2 26/26, PA3 20/20 and all full earlier course targets; `prior.log`, `pa{1,2,3}-result.json`, `pa{1,2,3}-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa4` | PASS 106/106 identities; `preserve.log` |
| `make lint` / protected `clone_quality.py` | PASS rustfmt, strict all-target/all-feature Clippy, debug/release unit tests and source audit; `quality.log` |
| `make audit-rust` | PASS 61 files, zero source/function/nesting/architecture findings; `audit.log` |
| Inclusion and pragma scratch probes, rerun against current Rust | PASS 202/202 and 549/549 exact status/stdout/stderr/complete output; `inclusion/` and `pragmas/` retain raw sources and observations |
| Packing state through the typed preprocessing API | PASS 4,103/4,103 token-by-token observations; `pragmas/pack-results.json`, `pack-{cpp,rust}.stdout`; Rust scratch view rebuilt against the updated library |
| Statistics on every course fixture | 112/112 equal work/status/diagnostics/output; `stats-fixtures-results.json`, `stats-fixtures/`; raw allocator storage retained and compared separately under the documented resource rule; this is supplemental evidence, not a protected-adapter pass |
| Focused stats/clock/error/demand/scaling probes | 25/25 equal work/status/stdout/output; `probe-results.json`, `probe/`; same storage caveat |
| Clock with UTC, fractional POSIX offset, implicit POSIX DST and named DST zones | 6/6 exact complete output pairs, each pair in one recorded epoch second; `clock-zones.json`, `clock-zones/`; initial host-date defect and differing-second observations retained separately |
| Protected adapter statistics reproduction | FAIL: only stderr allocator storage differs; `stats-policy/trace.jsonl` and `stats-policy/mismatches/`; required follow-up below |

### Remaining acceptance finding and dependency boundary

The exact reproducer is an empty `primary.cpp` and the unchanged adapter:

```sh
V5RUST_TRACE=/absolute/scratch/trace.jsonl \
  /home/vishvananda/work/v5rust-control/bin/preproc \
  --stats -o tokens primary.cpp
```

Both compilers return status 0 and emit identical complete token files.
Every deterministic field matches: source_tokens=24, directives=8,
files=2, source_bytes=223, identifiers=31 and all expansion work/peaks zero.
C++ reports `arena_bytes=16384`; Rust reports `arena_bytes=0`, because its
storage uses owned typed stores rather than the C++ bump allocator. The
protected trace records `match: false`, `differences: ["stderr"]`, and empty
statistics arrays. `clone_adapter.py::is_stats_line` recognizes only
`cppgm++`, `lowiropt` and `lowir2native` statistics, and its excluded storage
field spellings also omit the standalone preprocessor's `arena_bytes`.
This contradicts the conversion plan's explicit measurement treatment for
implementation-specific allocations and leaves this tool's storage field
outside the clone contract's recognized-statistics exception in the current
frozen checker. Faking the C++ allocator's measured value or changing the
protected comparator is not an authorized repair.

There are **no failing fixed-manifest case IDs** and no remaining known PA4
semantic stub. The outstanding beyond-fixture ID is
`stats-policy/empty-primary-source`: the protected adapter rejects its raw
storage observation. Full acceptance is not inferred from the green gates.
An external baseline/policy disposition is the next dependency boundary;
Ralph retains stage advancement and its existing goal. Hosted search paths,
include-next, environment/feature probes and dialect extensions are still
later-stage owners, as the PA4 handout specifies.

### Current resource observations

`measure.py` and `measure-macros.py` retain three runs per tool/scale,
raw `/usr/bin/time` records, source identities, complete output hashes and
macro work counters in `resources/` and `macro-resources/`. All corresponding
output hashes match. These are compile-time/storage observations, not a
paired performance-regression claim or generated-code optimization target.
Measurements use the final Rust binary after the clock repair; clocks and
statistics cannot select alternative semantic work. Wall time includes
launch and the once-per-invocation host clock query. RSS includes the host
clock subprocess high-water mark; it is not front-end arena storage.

| Workload | Scale | Primary bytes | Rust median ms | Peak RSS KiB |
| --- | ---: | ---: | ---: | ---: |
| guarded | 10,000 | 200,000 | 39.812 | 8,756 |
| guarded | 50,000 | 1,000,000 | 85.305 | 8,752 |
| guarded | 100,000 | 2,000,000 | 114.420 | 8,836 |
| once | 10,000 | 200,000 | 40.045 | 8,748 |
| once | 50,000 | 1,000,000 | 72.029 | 8,752 |
| once | 100,000 | 2,000,000 | 108.235 | 8,752 |
| unguarded-locations | 1,000 | 20,000 | 26.988 | 8,836 |
| unguarded-locations | 5,000 | 100,000 | 53.307 | 8,756 |
| unguarded-locations | 10,000 | 200,000 | 81.965 | 8,744 |
| Nested argument expansion, four output literals per call | 10,000 | 90,017 | 47.046 | 8,744 |
| Nested argument expansion, four output literals per call | 50,000 | 450,017 | 140.494 | 8,752 |
| Nested argument expansion, four output literals per call | 100,000 | 900,017 | 238.921 | 8,744 |

Guard/once skips retain one physical header image and location index;
unguarded reentries add small inclusion records, not image copies. Nested
argument expansions retain active context/argument buffers and reuse their
capacity after each call; retained storage does not follow emitted-token
count. Counter instrumentation records 2 invocations and 6 produced expansion
tokens per nested call (4 final literals), without causing an extra prescan.
The audit scratch evidence uses approximately 161 MiB; the shared filesystem
uses approximately 5.62 GB of 105.09 GB, below the 100-GiB ceiling.
