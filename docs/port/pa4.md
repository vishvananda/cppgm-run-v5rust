# PA4 port ledger

Stage base commit: e953408ec36e27923018214d0c788e91d2756633
Last audited commit: INITIAL

Implementation checkpoint: 52ff1e8344d359aaa501d740291fb105f6300df6 (macro group, 2026-10-11). This is a
coherent partial PA4 handoff, not stage completion or advancement. Ralph owns
the checkpoint audit and the last-audited marker.

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
| `src/preprocess/source/source_stack.{h,cpp}` | `dev/src/preprocess/input.rs` | Single-file text/directive boundary and stable token spellings |
| `src/preprocess/macro/macro_definition.{h,cpp}` | `dev/src/preprocess/macros/definition.rs` | Definition grammar, immutable roles, expanded-argument demand and redefinition |
| `src/preprocess/macro/macro_expander.{h,cpp}` | `dev/src/preprocess/macros/expander.rs` | Iterative rescanning, argument collection/prescan, substitution, stringizing, pasting and painting |
| `src/preprocess/macro/hide_sets.{h,cpp}` | `dev/src/preprocess/macros/hide_sets.rs` | Ordered interned additions, skew-jump unions and persistent membership indexes |
| `src/preprocess/lexer/pp_token.h`, `src/lex/post_tokenizer.{h,cpp}` | `dev/src/preprocess/macros/token.rs`, shared `dev/src/lex/{mod.rs,pp_lexer/mod.rs}` | Stable spellings, typed punctuators, spans and name identity through phase 7 |
| `src/preprocess/preprocessor{,_directives}.cpp`, `preprocessor_names.cpp`, `preprocess_error.h` | `dev/src/preprocess/{unit.rs,error.rs}` | Unit composition, define/undef/error/null/non-directive rules, reserved names and structured errors |

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
  present. File/line/date/time definitions explicitly report missing
  implementation when demanded, including during argument prescan. An unused
  or stringized raw argument does not demand their values. Attribute probes
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

Invariant tests cover deep hide-set membership/unions and token history,
4,096 nested argument prescans with all live buffers empty at EOF, typed paste
spans, name identity through substitution/literal-operator splitting, and
argument demand/prescan-once behavior. These supplement the differential suite.

## Validation evidence

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

## Remaining exact case IDs and earliest divergent owners

All paths below are relative to `pa4/tests/directives/`.

- Conditional directive execution / macro-expanded PA3 input:
  `150-error.t`, `150-no-error.t`, `170-nondir1.t`, `170-nondir3.t`,
  `170-nondir4.t`, `170-nondir5.t`, `170-nondir6.t`,
  `200-conditional-exclusion.t`, `200-defined-identifier-like-operator.t`,
  `200-if.t`, `300-defined-operator-misuse.t`, `400-bad-include.t`,
  `400-multiple-source-files.t`, `400-predefined-macros.t`, `500-alt.t`,
  `500-predefined-macros.t`. Owner: `preprocessor_directives.cpp` conditionals
  and `macro_expander.cpp` controlling-expression operators.
- Source frames/inclusion: `200-include.t`, `400-header-guard-shapes.t`,
  `400-header-guarded.t`, `600-pragma-op.t`, `800-pragma-once.t`.
  Owners: `source_stack.cpp`, `source_files.cpp`, include directives.
- Line directive: `300-line-new-line.t`. Owner: `run_line`.
- Location-dependent builtin replacement: `600-line-macro.t`,
  `600-predefined-macro-argument-expansion.t`, `610-line-macro.t`,
  `660-line-directive.t`. Owners: builtin expansion and source inclusion
  presumed locations.
- Pragmas: `500-pragma-ignore.t`, `501-pragma-op-ignore.t`.
  Owners: directive pragma execution and `run_pragma_operator`.

Other remaining responsibilities include inactive-group lexing without name
interning, malformed conditional ordering, file identity/once and guard skips,
macro-expanded include/line operands, build-time capture in the CLI, attribute
probes in controlling expressions, exact statistics counters and file/location
limits. Hosted named variadic parameters and hosted feature probes belong to
the later extension group; no current-stage coverage was substituted for them.

## Resource observations

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

## Handoff

Next dependency boundary: conditional inclusion with a directive-line expansion
input, `defined`/attribute handling in the rescanner and a typed streaming
adapter to the existing PA3 evaluator. This owns the largest remaining earliest
failure cluster (16 cases), then unlocks include guards and active/inactive
file directives. Preserve per-file conditional depth once source frames arrive.

Independent audit questions: verify open-context nesting after lookahead pops,
raw versus expanded argument hide-set replacement, directive error/byte ordering,
and that nested collection retains linked ranges and bounded lifetimes. Check
predefined-location demand inside prescan when the environment group is ported.
No oracle or harness defect was identified. Full PA4 remains incomplete; the
implementation checkpoint does not advance the audited stage.
