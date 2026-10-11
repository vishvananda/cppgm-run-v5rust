# PA1 port ledger

Stage base commit: `4250b77d11dbb3aaddf7a8a7a7bae806b4058a3d`
Last audited commit: INITIAL (unchanged; Ralph owns the independent audit)
Candidate implementation commit: pending commit of this implementation group.

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
`/home/vishvananda/work/private/v5rust/checkout/artifacts/pa1-port/`.
Generated evidence, scratch inputs and build targets remain uncommitted.

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
| `python3 artifacts/pa1-port/differential.py` | PASS, 639/639 additional exact exit/stdout/stderr comparisons; `differential.log`, `differential.json` |
| `python3 artifacts/pa1-port/measure.py` | PASS, paired output hash equality at every scale; `perf.log`, `perf/measurements.json` |
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

## Handoff

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
