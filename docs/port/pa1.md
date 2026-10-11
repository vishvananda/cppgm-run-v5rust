# PA1 port ledger

Stage base commit: `4250b77d11dbb3aaddf7a8a7a7bae806b4058a3d`
Last audited commit: `1093694671d27c96ae50619c86a33774127ec693`
Audit entry commit: `7a04be0fb104ae0887327a1f0955f2fc2163fad8`
Acceptance: **PA1 full-stage accepted**, after the independent review and fixes below.
Candidate implementation commit: `9459c35d3f6a20f570645b3117c006e0debcb923`.
The initial implementation evidence below is historical; the final audit section
records validation of the last audited commit. This ledger is its documentation-only descendant.

## Contract and ownership

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, `pa1/README.md` and
`docs/rust-clone-plan.md`. The implementation oracle is the read-only tree
`/home/vishvananda/work/references/v5rust-cpp`, commit
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

| C++ responsibility consulted | Rust owner and boundary |
| --- | --- |
| `dev/pptoken.cpp` and `dev/pptoken-scaffold.cpp` | `dev/bin/pptoken.rs`: stdin, argument handling, final flush, status and diagnostics |
| `source/source_buffer.{h,cpp}` | `dev/src/preprocess/source/mod.rs`: immutable bytes, BOM/final-newline rules, compact `SourceOffset`, monotonic `SourceLineCursor` |
| `source/translation_reader.{h,cpp}` | `dev/src/preprocess/source/translation.rs`: stateless logical/raw views, trigraphs, splices, Windows-1252 continuation-byte mapping |
| `lexer/char_classes.{h,cpp}` | `dev/src/preprocess/lexer/classes.rs`: exact Annex E ranges and lexical character classes |
| `lexer/pp_lexer.{h,cpp}` | `dev/src/preprocess/lexer/mod.rs`: token assembly, demand, comments, UCNs, identifiers, numbers and directive context |
| `lexer/pp_lexer_literals.cpp` | `dev/src/preprocess/lexer/literals.rs`: literal validation, suffixes, raw extents and header names |
| `lexer/pp_lexer_punctuators.cpp` | `dev/src/preprocess/lexer/punctuators.rs`: longest-first vocabulary and typed alternative-spelling identity |
| `tokens/pp_token.h`, `tokens/pp_token_cursor.h` | `PpTokenKind`, borrowed `PpToken`, `Punctuator`, `PpLexer::next_token`; later macro metadata remains pending |
| `tokens/pp_token_emitter.{h,cpp}`, `tokens/IPPTokenStream.h`, `tokens/DebugPPTokenStream.h` | `dev/src/driver/pptoken.rs`: debug output adapter consuming typed tokens; later consumers remain pending |
| `dev/src/support/unicode/utf8.{h,cpp}` | `dev/src/support/unicode.rs`: shared validating decoder and scalar encoder |

C++ rows in this table are relative to `dev/src/preprocess/` except where
explicitly prefixed. `docs/port/source-map.tsv` records every translated owner;
its frozen census is retained. The due candidate tool is `pptoken`, routed by
the protected adapters. The fixed PA1 manifest contains 58 existing cases.

## Coherent behavior groups

Completed implementation groups, validated together:

- Whole-file BOM, empty-file and implicit newline rules, including a final
  newline preceded by a physical backslash or trigraph backslash.
- UTF-8 shortest-form/scalar validation with exact failure precedence and
  diagnostics; the oracle's legacy continuation-byte mapping; trigraph
  replacement before splicing; physical byte locations for diagnostics.
- Maximal horizontal whitespace/comments, line comments across splices,
  block comments containing newlines, and unterminated-comment errors at
  the whitespace sequence's physical start.
- Exact Annex E identifier boundaries, valid/malformed UCN handling in
  identifiers, numbers and comments, identifier-like operators, pp-number
  exponent/sign and dot boundaries.
- Encoding prefixes, quoted escapes and UCN structure preservation,
  user-defined suffixes, empty character literals, raw delimiter validation
  and its 16-character bound, raw-view restoration, overlapping delimiter
  candidates and mapped bytes inside raw literals.
- Contextual header names after `#`/`%:` and `include`/`include_next`, fallback
  to ordinary tokens for incomplete headers, and header UCN translation.
- Every punctuator spelling, alternative-spelling identity and the `<::`
  exception; exact dump lengths, spaces and emission order; error-prefix
  stdout, status and stderr; idempotent EOF and ignored unknown CLI arguments.
- The frozen lexer options: requested headers, token-text views, binary
  exponents and explicit wrapping addition of the 32-bit location base.
  The CLI reproduces the oracle's `--batch-stdin` sentinel protocol, including
  arbitrary byte lines; normal candidate tokenization is fully implemented.

Architecture review: one immutable source image owns the physical bytes.
The translation reader decodes on demand without per-character storage.
Clean spellings borrow source slices; the first transformation transfers the
prefix into one reusable lexer scratch vector. A token borrows either source
or scratch until the next demand. Four characters bound punctuator lookahead;
16 code points bound raw delimiter lookahead. Raw scans can revisit delimiter
candidates only within that fixed bound. Tokens carry compact physical offsets
and typed category/punctuator facts, with no owning token collection or text
round trip. Diagnostic line/column computation is lazy and monotonic.

The bare lexer retains borrowed identifier spellings, as the C++ lexer does;
the translation-unit preprocessor will own identifier interning. Macro flags,
hide-set IDs, expansion origins and placemarkers belong to PA4. Typed parser,
semantic and backend work remains with its future owning stages. This group
introduces no optimization or code-generation transformation. Shared UTF-8
algorithms live in support, lexical algorithms in preprocessing, output in
the driver, and environment/argument reads at the binary boundary.

## Evidence

Turn-start fixed-manifest failures: 58. Current fixed-manifest failures: 0.
Exact remaining failing case IDs: `[]`. Previously passing case IDs at entry:
`[]`; all 58 existing identities now pass, with no coverage reduction.

Manifest SHA-256:
`fddaac6242a26363161921d983e2609ddd5fba125cf1598df627521817b2991e`.
Candidate release `pptoken` SHA-256:
`64acfa57ab6846314cbd899467867abb281c84af8a38f65999f4d08201634eb0`.
Toolchain: `rustc 1.98.0 (88d9e12ae 2026-08-18)`, Linux x86-64.

The evidence root below is
`/home/vishvananda/work/private/v5rust/artifacts/pa1-port/`.
Generated evidence, scratch inputs and build targets remain uncommitted.
They were moved from `checkout/artifacts/pa1-port/` after validation; raw
invocation records retain their original absolute paths. The supplementary
scripts now write to the external evidence root when run from the checkout.

| Validation command | Result and evidence under the evidence root |
| --- | --- |
| `make build` | PASS; isolated clean release build also measured in `build.log`, `build-resources.txt` |
| `make test-pa1` (runs `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa1`) | PASS, 58/58; `stage.log` |
| `make course-pa1` | PASS, 58 matching invocations; `course.log` |
| `make lint` (runs `python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS: rustfmt, strict all/pedantic Clippy, 11 tests in both debug and release, file/architecture audit; `quality.log` |
| `make audit-rust` | PASS: 31 Rust files, zero findings; `audit.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa1` | PASS: frozen inputs, policy and oracle integrity; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa1` | PASS: no earlier assignments due; `prior.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa1` | PASS, 0/0 historic passing identities; `preserve.log` |
| `python3 /home/vishvananda/work/private/v5rust/artifacts/pa1-port/differential.py` | PASS, 639/639 additional exact exit/stdout/stderr comparisons; `differential.log`, `differential.json` |
| `python3 /home/vishvananda/work/private/v5rust/artifacts/pa1-port/measure.py` | PASS, paired output hash equality at every scale; `perf.log`, `perf/measurements.json` |
| `git diff --check` | PASS |

Protected primary result and raw invocation artifacts:

- `/home/vishvananda/work/private/v5rust/artifacts/clone/results/pa1.json`
  records all 58 exact case IDs, binary hashes and their trace/log paths.
- `/home/vishvananda/work/private/v5rust/artifacts/clone/cases/pa1/<case-digest>/trace.jsonl`
  and `course.log` retain per-case exit, stdout/stderr hashes and diagnostic
  evidence. The protected stage comparator checks raw bytes, not just course
  acceptance. For example, the invalid hex escape case digest is
  `f97f847a70dba643cbc8`; UTF-8 rejection is `d831ba66fe139a9f0aba`.
- `/home/vishvananda/work/private/v5rust/artifacts/clone/course/pa1/test/trace.jsonl`
  and `course.log` retain the complete owning course run.
- `inputs.json` in the evidence root records the base commit, exact oracle
  commit, host/toolchain, all Rust source hashes and all candidate binary hashes.
  The supplementary comparison records each reproducible input and option set.

Unit tests cover source borrowing and physical extents, alternative token
identities, repeated EOF, Unicode raw delimiters/legacy byte mapping, exact
output prefixes and physical diagnostics, literal UCN structure, explicit
lexer options, target offset wrap, UTF-8 scalar/error boundaries and monotonic
source cursor invariants. The additional differential cases include all 256
single physical bytes, UCN value/placement boundaries, raw delimiter lengths,
byte-oriented batch requests, unknown CLI arguments and deterministic mixed
lexical sequences. These supplement the fixed manifest without changing it.

## Resource observations

Supported hot path: tokenize the frozen C++ `pp_lexer.cpp` (9,526 bytes)
repeated 64/256/1024 times. This exercises ordinary source, directives,
comments, literals and punctuation. GNU time measures the whole tool,
including stdin and the token dump written to a regular file. Two Rust A/A
calibration invocations precede two ABBA blocks at each size; each paired
output has exactly the same SHA-256 for C++ and Rust. Inputs, commands, binary
hashes, individual timings and RSS are in `perf/measurements.json`.

| Source bytes | Rust median wall time | Rust maximum RSS | C++ median wall time | C++ maximum RSS |
| ---: | ---: | ---: | ---: | ---: |
| 609,664 | 40 ms | 2,948 KiB | 40 ms | 4,372 KiB |
| 2,438,656 | 140 ms | 4,356 KiB | 150 ms | 5,776 KiB |
| 9,754,624 | 565 ms | 11,380 KiB | 595 ms | 12,820 KiB |

An isolated clean release workspace build took 1.10 s with peak RSS
139,240 KiB (`/usr/bin/time make build CARGO_TARGET_DIR=.../build-target`).
The normal candidate build remains in `obj/rust`.

Observed time and storage scale approximately linearly on this supported
workload. GNU time's 10 ms reporting granularity and these warm local runs
limit performance conclusions. This is a lexer measurement, not a claim
about later source-to-object throughput or generated-code improvements.

## Implementation handoff (before independent audit)

PA1's complete lexical implementation group is ready for Ralph's independent
checkpoint/final audit. Implementation has not advanced the last-audited
commit or changed Ralph state, references, fixtures, manifests or gate policy.
The interrupted prior turn left no implementation change in the authoritative
checkout; this turn establishes implementation and validation progress.

Next dependency boundary: PA2's `posttoken` consumers in
`dev/src/lex/{post_tokenizer,number_literal,text_literal,string_concatenation,ud_suffix}`
and token-kind vocabulary. Consume this typed pull lexer directly and reuse
support UTF-8; validate target numeric widths, floating bits, encoding and
literal concatenation against those C++ owners. PA1 tokens remain phase 3
spellings, with phase 5/7 interpretation owned by PA2. Later source inclusion,
identifier interning and macro metadata remain with PA4.

Audit questions: verify every punctuator identity and Annex E bound against
the C++ tables; verify raw/logical transitions and lazy decoding errors retain
emission order; verify source/scratch lifetimes and private compact-offset
bounds; independently replay the fixed manifest and course lane. No oracle
or harness defect was found in the executed scope.

## Full-stage acceptance audit, 2026-10-11

Reviewed the stage base, the implementation at `9459c35`, the audit entry at
`7a04be0` and the complete resulting implementation at `1093694`. Read the full
owning handout, clone contract, specification, conversion plan, source map and
ledger. Consulted all C++ owners in the ownership table directly in the frozen
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1` tree, including the shared UTF-8
implementation, token interfaces and `lex/token_kind.h` punctuator identities.
This is a responsibility and control-flow review, supplemented by execution;
the 58 fixture results alone do not establish completeness.

Continuation assessment: no independent audit increment or live validation
process remained at entry after the interrupted turn. It was treated as no
verified audit progress; current source and all required checks were revalidated.

### Findings and resolutions

1. **CLI stream failures differed.** C++ iostreams have exceptions disabled:
   output failure sticks but does not stop tokenization or replace a lexical
   diagnostic. Before the fix, Rust returned an ENOSPC diagnostic for a valid
   source written to `/dev/full`, and also hid the source error in
   `ok\n"\\q"`. The CLI now owns a sticky output adapter; the shared renderer
   retains its typed, fallible `Write` API. The final regressions are
   supplemental IDs `10774`–`10778`, including a source error after 20,000
   bytes of preceding input and the documented batch sentinel mode.
2. **Seekable input and batch read failures differed.** Frozen
   `SourceBuffer::ReadFrom` probes the current/end offsets, rewinds and reserves
   the whole image plus one newline. A directory's Linux end offset exceeds
   the host string capacity and produces `ERROR: basic_string::_M_create`;
   Rust previously reported EISDIR. Batch `getline` ends silently on a read
   failure; Rust previously rejected it. A safe duplicate of stdin now
   preserves the shared current file position; checked reservation reproduces
   the frozen capacity failure, and read failures retain the available prefix.
   IDs `10779`–`10782` cover the invalid input and nonzero seek positions.
   Source ingestion remains at the binary boundary. The source map now
   records that owner alongside the immutable source store.
3. **Transformed spelling lengths lacked target truncation.** C++ narrows
   scratch length to `uint32_t` both for prefix classification and published
   token text. Windows-1252 mapping can expand a permitted source past that
   width. Rust previously exposed the full `usize` length. The lexer now
   deliberately truncates the text view in the target's 32-bit space while
   retaining the actual scratch storage and physical extent. The unit
   `expanded_spelling_length_wraps_in_the_targets_32_bit_space` checks the
   maximum, first wrapped length and expansion bound. A multi-gigabyte input
   was not allocated: this boundary is established by the C++ conversion,
   the matching Rust operation and the arithmetic test.

No stage-owned finding remains unresolved. Fixed-manifest failing IDs: `[]`.
Supplemental failing IDs in both profiles: `[]`. Typed option/span probe
failing IDs: `[]`. All 58 passing identities at audit entry remain passing;
the independent entry/result set comparison confirms this in addition to
the protected preservation command.

### Completeness and architecture reasoning

| Reviewed responsibility | Evidence and conclusion |
| --- | --- |
| Physical input and phase 1–2 translation | Same BOM/empty/final-newline decisions; trigraph-before-splice order; physical offsets; exact Windows-1252 table and UTF-8 failure precedence. Seekable and pipe CLI input now follow the frozen stream policy. |
| Character classes and numeric boundaries | Every Annex E1/E2 range is equal and sorted, verified from both source tables. ASCII/basic/control exclusions and scalar validity match. Eight-digit UCN accumulation fits `u32`; offset-base addition wraps explicitly; expanded text lengths now truncate explicitly. PA1 pp-numbers retain spelling, with numeric interpretation due in PA2. |
| Whitespace, comments, identifiers and pp-numbers | Same dispatch precedence, maximality, comment termination and diagnostic origin. Malformed UCN fallback and complete invalid UCN rejection match in start/continuation positions. Exponent/sign continuation follows the explicit binary-exponent option. |
| Quoted literals and suffixes | Same prefix sets, empty-character tokenization, simple/octal/hex escape consumption and failure positions. Basic/control UCNs retain lexical structure; extended scalar spellings translate. Suffixes use Annex E start/continuation rules, with no numeric interpretation here. |
| Raw strings | Same transition from logical opening quote to physical delimiter/body, 16-code-point limit, error precedence and closing-delimiter overlap scan. Legacy-byte mapping controls the verbatim fast path. Logical scanning resumes at the closing physical extent. |
| Header names | Same hash/digraph and include/include_next context transitions, whitespace preservation, newline reset, explicit requests, nonempty requirement, fallback and UCN behavior. Lookahead scans one line and commits only on success, with no token/source clone or escaping speculative state. |
| Punctuation and typed identity | Every spelling and identifier-like operator compared against C++ tables; all canonical enum associations reviewed against `TokenKind`. Longest-first candidates and the `<::` exception retain order and lazy decoding failures. Hash identities remain distinct from phase-7 tokens. |
| Output, failures and CLI modes | Exact byte counts, trailing spaces for empty tokens, newline/EOF output, prior output on source errors, diagnostic positions/text, statuses and ignored arguments. Repeated EOF is stable. The frozen no-runner batch sentinel is reproduced; actual harness batch requests are test-only adapter orchestration. |
| Shared interface/options | 1,280 direct C++/Rust pull-lexer comparisons cover all combinations of header-demand, token-text and binary-exponent options, source-buffer flavor, repeated header requests, four location bases including wraparound, physical begin/end spans, repeated EOF and error-prefix output. |

The due `pptoken` compiler work runs entirely in Rust. The protected original
course run records 58 invocations of `obj/rust/release/pptoken`; the adapter
invokes C++ only for comparison and test transport. The supplementary script
is also test tooling, never a production implementation. No production phase
executes a process, reads an environment setting, recognizes a fixture or
consumes an oracle answer. No dependency was introduced; Cargo.lock still
contains only the workspace package.

One immutable byte image owns source text. The pull lexer owns one reusable
scratch buffer and bounded punctuator/delimiter lookahead, with no complete
token collection or record per character. Token borrows prevent reuse of
scratch while a consumer retains its view. Private compact physical offsets,
typed token categories and punctuation facts pass directly to consumers.
Header scans visit their line at most twice; raw delimiter retries are bounded
by 16 characters, with one additional body pass only for mapped bytes.
Source locations are rendered lazily from physical bytes. Typed source and
output errors remain separate until the CLI applies its observable policy.

Identifier interning belongs to the PA4 translation-unit preprocessor, matching
the C++ bare lexer, whose identifier pointer is null. Macro flags, hide sets,
origins and placemarkers remain explicitly marked partial for PA4. Full
phase-7 token vocabulary is pending PA2; the source map now names the PA1
punctuator identity subset of `token_kind.h`. Parsing/rollback, semantic caches,
template demand and per-function backend ownership are not due in PA1; no
later-stage implementation is being certified here. The next dependency
boundary remains PA2's typed post-token/literal consumers. No optimizer,
allocator, generated-code policy or PA35 objective changed.

Strict formatting and all/pedantic Clippy pass without a new allowance or
expectation. Unsafe code remains forbidden. The authoritative companion audit
checks 31 Rust files with zero findings. `shape.json` reports a maximum of
405 code lines per PA1 implementation file, 83 per function and nesting five,
below the respective 3,000/240/eight hard limits (and six nesting warning
threshold). These metrics include the cohesive punctuator dispatch rather
than excluding it as vocabulary. No mechanical splits, macros hiding logic,
text transport or unrelated storage owners were introduced.

### Final validation and artifacts

Final audit evidence root:
`/home/vishvananda/work/private/v5rust/artifacts/pa1-audit/`.
`inputs.json` records the reviewed commit, frozen oracle commit, Rust source
hashes, toolchain, manifest and executable hashes. Release `pptoken` is
`8f20b051c68cda894488fc87a50efecf311f614967047fcbd713f7c92de9c5f8`;
debug is `27976386c14ac7dfb041bac2b62c541e68d5e475a46165f89fb618464db3469f`.
All final comparison and performance receipts match these current binaries.

| Command | Result; evidence relative to the audit root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `cargo build --manifest-path dev/Cargo.toml --workspace --locked` | PASS; `debug-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa1` | PASS, 58/58; `stage.log`, `stage-result.json` |
| `make course-pa1` | PASS, original complete `pa1 test` target, 58 exact Rust invocations; `course.log`, `course-output.log`, `course-trace.jsonl` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa1` | PASS, frozen source/tools/inputs and policy unchanged; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa1` | PASS; no earlier assignments due; `prior.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa1` | PASS, 58/58 historic identities; `preserve.log`, independent entry snapshot `entry.json` |
| `make lint` (`python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS, rustfmt, strict Clippy, 14 tests in each of debug/release, source/architecture audit; `quality.log` |
| `make audit-rust` | PASS, 31 files/zero findings; `audit.log`, detailed `shape.json` |
| `make test-harness` | PASS, all 17 protected checker tests; `harness.log` |
| `python3 scripts/audit_pa1.py --evidence /home/vishvananda/work/private/v5rust/artifacts/pa1-audit/supplemental --profile release` | PASS, 10,783/10,783 exact raw exit/stdout/stderr comparisons; `supplemental-release.log`, `supplemental/release.json` |
| Same command with `--profile debug` | PASS, 10,783/10,783; `supplemental-debug.log`, `supplemental/debug.json` |
| `python3 /home/vishvananda/work/private/v5rust/artifacts/pa1-audit/options/compare.py` | PASS, 1,280/1,280 typed lexer comparisons; `options.log`, `options/results.json`, probe sources/binaries under `options/` |
| `python3 /home/vishvananda/work/private/v5rust/artifacts/pa1-audit/measure.py` | PASS, output hash equality at every scale; `perf.log`, `perf/measurements.json` |
| `git diff --check` | PASS |

The typed C++ probe links the frozen `pp_lexer`, literal/punctuator, character
class, source-buffer/translation and UTF-8 owners using `g++ -std=gnu++11 -O2`
and the frozen `dev/src` include path. The Rust probe links the current
`obj/rust/release/libcppgm_rust.rlib` with `rustc --edition=2024 --forbid
unsafe_code`. Both are external test-only artifacts; neither changes frozen
sources or runs as a candidate tool. The option corpus compares typed
category/physical spans/spellings and source errors directly, rather than
reparsing the CLI dump into production facts.

The committed supplementary audit generator verifies source-table completeness
and records every input, arguments, stream mode and both raw outputs. It adds
4,900 adjacent-punctuator sequences, 2,409 Unicode boundaries, 1,280 physical
byte contexts, 512 basic UCNs, 280 lazy-lookahead failures, 210 raw extents,
80 header contexts and deterministic mixed/random input, plus CLI/stream
cases. The fixed manifest, course fixtures and reference sidecars are unchanged.
The new storage/arithmetic regressions execute in the workspace's Rust tests.

PA33 release/debug lanes and the implementation-wide completion census are
conditional future requirements, not due stage gates here. Both Rust profiles
were nevertheless checked for the PA1 implementation. Ralph state and its
existing goal were neither edited nor replaced.

### Resource observations after the fixes

Repeated the frozen 9,526-byte `pp_lexer.cpp` input at 64/256/1024 copies, with
two Rust A/A calibration invocations then two paired ABBA blocks per size.
The final measurements ran after the parallel validation work finished.
Input/binary hashes, individual commands, raw stderr, timings and RSS remain
under `perf/`; every paired dump has identical SHA-256. These measure lexer
tool throughput and storage, independently of generated-code equality.

| Source bytes | Rust median wall time | Rust maximum RSS | C++ median wall time | C++ maximum RSS |
| ---: | ---: | ---: | ---: | ---: |
| 609,664 | 30 ms | 2,948 KiB | 35 ms | 4,432 KiB |
| 2,438,656 | 140 ms | 4,340 KiB | 145 ms | 5,840 KiB |
| 9,754,624 | 560 ms | 11,380 KiB | 570 ms | 12,880 KiB |

Observed time and memory scale approximately linearly on this supported
workload. Ten-millisecond timing granularity and local warm-cache conditions
limit conclusions; no optimization target or cross-implementation resource
equality is asserted. The audit used about 35 MiB of additional retained
evidence, comfortably within the shared 100 GiB storage ceiling.

Handoff: PA1's due responsibilities, exact failure/byte behavior, original
course coverage, prior-case preservation and Rust architecture/quality have
been reviewed and validated at `1093694`. This ledger records acceptance and
the next PA2 dependency boundary; stage advancement remains Ralph's role.
