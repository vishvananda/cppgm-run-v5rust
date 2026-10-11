# PA2 port ledger

Stage base commit: `6f0c1b97b997e01289470a8220d658b9ef95bfc9`
Last audited commit: `ab3105fd2cfc05f9d39d147a513d4f0c08e3ee06`
Text-group entry HEAD: `8ca05d6de479d2663b9a8f5437c1a79cfca6ff8a`

The PA2 implementation checkpoint at `d71823c` passed its frozen manifest and
original course target: 26/26, up from 14/26 at that clean-checkout entry. The previous
turn was progress: commit `8ca05d6` implemented the shared numeric/simple group,
with twelve remaining status-86 text-literal failures. This increment implements
that entire shared text group. Ralph owns checkpoint/final audits and stage
advancement; this implementation does not advance the last-audited commit or
claim acceptance from fixture counts alone.

The independent full-stage audit below accepts the implementation at
`ab3105f`, after two compatibility fixes and correction of incomplete
supplemental execution. Current results are 26/26 frozen cases and 28/28 full
course cases, including two new regressions; there are no outstanding PA2
findings. The following implementation-checkpoint evidence is historical.

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, the complete `pa2/README.md`
and `docs/rust-clone-plan.md`. Consulted the frozen C++ snapshot
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1` directly, including the complete text,
sequence, post-tokenizer and writer owners, the existing shared numeric owners
and the identifier/operator vocabulary. The source map distinguishes the PA2
surface from APIs demanded by later assignments.

## C++ and Rust owners

| C++ responsibility | Rust owner and completed PA2 behavior |
| --- | --- |
| `dev/posttoken.cpp` | `dev/bin/posttoken.rs`, `dev/bin/source_tool/mod.rs`: bare-source CLI, stream policy and source diagnostics |
| `lex/post_tokenizer.{h,cpp}`, `lex/post_token.h` | `dev/src/lex/mod.rs`: typed conversion on demand, physical spans, borrowed spellings, one lexer lookahead, literal-operator split and typed scalar/array facts |
| `lex/text_literal.{h,cpp}` | `dev/src/lex/text.rs`: compact literal boundaries, encoding prefixes, scalar/numeric escape distinction, character evaluation and target multicharacter packing |
| `lex/string_concatenation.{h,cpp}` | `dev/src/lex/strings.rs`: private maximal-sequence store, prefix/suffix unification and target code-unit encoding |
| `lex/post_token_writer.{h,cpp}` | `dev/src/driver/posttoken.rs`: all five output forms, source joining, user-defined text suffixes, array bounds and uppercase little-endian hexdumps |
| `lex/token_kind.{h,cpp}`, PA1 punctuator vocabulary | `dev/src/lex/vocabulary.rs`, `dev/src/lex/pp_lexer/punctuators.rs`: canonical simple identities, including alternative operator words after a literal-operator split |
| `lex/number_literal.{h,cpp}`, `lex/ud_suffix.{h,cpp}`, `lex/interchange_float.{h,cpp}` | Existing `dev/src/lex/number.rs`, `dev/src/lex/floating.rs`: exact C++11 numeric classification and standard-format rounding, reused unchanged |
| `types/fundamental_type.{h,cpp}` | `dev/src/types/fundamental.rs`: thirteen distinct numeric/text identities and their PA2 target names/sizes |
| `preprocess/lexer/*`, `support/unicode/*` | `dev/src/lex/pp_lexer/`, `dev/src/support/unicode.rs`: existing phase-3 spelling and scalar decoding; a narrow scan/current-token interface permits lookahead without copying ordinary tokens |

## Completed behavior and production invariants

Character decoding preserves simple escapes, fixed-width UCNs, three-digit
octal escapes and arbitrary-length hexadecimal escapes. Hex values above
32 bits saturate at the C++ sentinel. `LiteralCharacter` distinguishes a
Unicode scalar from a numeric code-unit value. Character literals require one
scalar, ordinary ASCII has type `char`, other ordinary scalars have type `int`,
and `u` characters must fit one UTF-16 unit. Wide and UTF-32 identities remain
distinct. Unprefixed multicharacter literals retain the C++ compiler's `int`
packing facts, with explicit 32-bit wrapping shifts; the PA2 writer reports
these as invalid. A multicharacter literal with a user-defined suffix stays
invalid.

A maximal adjacent string sequence is collected through exactly its first
non-string token. One nonordinary prefix and one suffix identity may apply to
the entire sequence; differing prefixes or suffixes invalidate the complete
joined spelling. Numeric escapes produce exactly one unsigned code unit and
are range-checked against the final sequence width. Numeric surrogates are
permitted in UTF-16 strings; Unicode scalar sources become surrogate pairs
when needed. Raw bodies preserve their physical characters and never decode
escapes. UTF-8 runs are copied in bulk, UTF-16/UTF-32 units are serialized
explicitly little-endian, and exactly one null unit terminates a valid array.
Invalid sequences retain their complete source spelling.

Directly after `operator`, phase-3 `""suffix` splits into an empty string and
the independently classified identifier. The suffix's span starts at the C++
earliest possible offset (`begin.wrapping_add(2)`), including intervening
splices. This split does not pull another preprocessing token or concatenate a
following string. It uses the same keyword/alternative-operator lookup as the
other simple-token paths. Supplemental comparisons exposed and fixed the
`operator ""and` alternative-spelling case, and exercise all operator words.

The lexer retains one current token; lookahead stores only its kind while
its spelling remains in the lexer image/scratch. Ordinary numeric and simple
post-tokens still borrow that spelling directly. A string sequence owns one
space-joined spelling buffer, a flat list of compact piece offsets/lengths
and parsed 32-bit body/suffix boundaries, plus its encoded units. Boundaries
are parsed once per piece; encoding consumes those facts after the final
prefix is known. No rendered output is reparsed. The split identifier uses
one reusable suffix buffer. Contents are cleared after their last consumer,
retaining capacity only. Returned loans prevent advancing while a consumer
holds the current token. Host buffer sizes are checked; exposed spelling offsets, lengths and array
byte counts preserve C++ uint32 truncation. A suffix borrows its full owner
independently of that exposed source length. Language-invalid values remain
distinct from structured host-size limits and phase-1–3 source errors.

Source errors while finding the end of a string run precede that run's
conversion/output, just as in C++. The operator split emits its two tokens
before demanding later input. Unit tests cover these demand boundaries,
translated lookahead spellings, physical spans, scalar versus numeric-surrogate
units, suffix keyword identity, multicharacter target packing, EOF and sequence
reuse after failure. The writer consumes the typed facts directly. The companion shape evidence
reports maxima of 421 code lines per file, 94 per function and nesting four.

No dependency, unsafe code, FFI, subprocess compiler implementation, source
fixture recognition, reference-answer dependency or environment read under
`dev/src/` was introduced. All changes belong to lexing, literal types, text
views or their evidence tooling. Optimizer, allocator and code-generation
behavior were not changed. PA34 object/tool byte equality remains its own
obligation; PA35 improvement targets are outside this phase.

## Validation evidence

Current evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa2-text/`.
`inputs.json` records entry/base/oracle revisions, source and binary hashes,
toolchain and unchanged manifest digest. `commands.json` collects final command
statuses; each `*-command.json` records the exact invocation, duration and log.
`stage-result.json` and `stage-cases/` snapshot all protected PA2 results/traces.
`course-evidence/` retains the complete original course log, invocation trace,
26/26 counts and absence of failure markers. Raw candidate/oracle input, stdout
and stderr for every frozen PA2 input are additionally retained under
`supplemental/{release,debug}/frozen-*/`.

| Command | Result and evidence relative to the current root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `cargo build --manifest-path dev/Cargo.toml --workspace --locked --target-dir obj/rust` | PASS; `debug-build.log` |
| `make test-pa2`, then `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa2` on final code | PASS, 26/26; `stage.log`, `stage-result.json`, `stage-cases/` |
| `make course-pa2` | PASS, original target 26/26, no failure marker; `course.log`, `course-evidence/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa2` | PASS, PA1 58/58 plus original course target; `prior.log`, `prior-result.json`, `prior-course-evidence/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa2` | PASS; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa2` | PASS, all 14/14 historical PA2 identities; `preserve.log` |
| `make lint` (runs `python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS, rustfmt, strict Clippy, 26 library and four CLI tests in both profiles, companion audit; `quality.log` |
| `make audit-rust` | PASS, 39 files, zero findings; `audit.log`, `shape.json` |
| `python3 scripts/audit_pa2_text.py --evidence <root>/supplemental --profile release` and `--profile debug` | PASS in each profile: 6,004 generated cases across six streams, 26 boundary/CLI probes and 26 frozen inputs; 58/58 exact exit/stdout/stderr comparisons; `text-{release,debug}.log`, `supplemental/{release,debug}/results.json` |
| `python3 scripts/audit_pa2_numbers.py --evidence <root>/numeric --profile release` and `--profile debug` | PASS in each profile, 4,767 numeric/vocabulary inputs plus eight CLI probes, 14/14 comparisons; `numeric-{release,debug}.log`, `numeric/{release,debug}/results.json` |
| `python3 <root>/measure.py` | PASS, all paired output bytes match; `perf.log`, `perf.json`, `perf/` |
| `git diff --check` | PASS |

Fixed-manifest coverage remains 26, with the same manifest digest
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.
Failures reduced from twelve to zero. Exact newly passing case IDs, all under
`pa2/tests/`:

- `200-character-literal.t`
- `200-string-numeric-escape-code-units.t`
- `200-unicode-character-literals.t`
- `200-unicode-user-defined-literal-suffix.t`
- `250-string-literal.t`
- `250-ud-strchar.t`
- `300-invalid-character-string-boundary.t`
- `300-string-numeric-escape-out-of-range.t`
- `400-raw-string.t`
- `450-string-literal-concat.t`
- `700-hard-string-concat.t`
- `750-reserved-literal-operator-suffix.t`

Current failing case IDs: none. Previously passing identities all remain green.
No rejecting input passes through a status-86 text stub. No oracle/harness
defect was found. Supplemental inputs cover numeric escape width boundaries
and saturation, scalar boundaries, all encoding/suffix combinations, raw
delimiters and physical text, malformed lexical input, splices, deterministic
mixed runs, reserved suffixes and the split operator's demand ordering.
These supplement the frozen gate without changing its policy or fixtures.

## Resource observations

`measure.py` records supported text-path processing time, RSS, input and binary
hashes and all output hashes. Each workload has two Rust A/A warmup runs followed
by two paired ABBA blocks. Measurements ran after other validation processes
finished. The streamed workload repeats a short sequence containing UTF-16
raw/ordinary concatenation, numeric escapes, supplementary scalars, user-defined
suffixes, wide characters, multicharacter literals and integers, with token
boundaries between runs. The maximal-sequence workload repeats UTF-16-prefixed
and ordinary/raw string pieces without an intervening non-string token,
exercising the necessary one-sequence storage lifetime.

| Workload | Source bytes | Rust median ms / max RSS KiB | C++ median ms / max RSS KiB |
| --- | ---: | ---: | ---: |
| streamed-text | 174,080 | 10 / 2,204 | 0 / 3,640 |
| streamed-text | 696,320 | 20 / 2,940 | 20 / 4,376 |
| streamed-text | 2,785,280 | 110 / 5,052 | 85 / 6,460 |
| maximal-sequence | 122,880 | 0 / 2,936 | 0 / 4,344 |
| maximal-sequence | 491,520 | 10 / 3,612 | 10 / 6,456 |
| maximal-sequence | 1,966,080 | 50 / 9,272 | 40 / 15,356 |

The table uses `/usr/bin/time` wall samples, with ten-millisecond resolution;
zero is below resolution, and a median can fall between adjacent samples.
`perf.json` also retains end-to-end observed durations, including subprocess
wait overhead. These are local warm-cache observations, not a time-equality
requirement or a claim of improved generated code. Time and memory increase
with source size. Maximal sequences retain their necessary pieces and units;
streamed sequences release contents between runs. Piece boundaries are compact
and conversion does not rescan an expanding concatenation for each new piece.

Historical numeric checkpoint evidence remains under
`/home/vishvananda/work/private/v5rust/artifacts/pa2-numeric/`, described in
commit `8ca05d6`. Its largest frozen-numeric workload was 1,143,296 bytes at
50 ms / 2,932 KiB Rust versus 30 ms / 5,064 KiB C++; its largest bounded
long-mantissa workload was 3,073,280 bytes at 375 ms / 5,028 KiB Rust versus
30 ms / 6,520 KiB C++. The numeric scanner and its target rounding were not
changed by this increment. Current text evidence occupies approximately 238 MiB
within the shared 100 GiB ceiling; generated outputs remain outside the commit.

## Handoff and next dependency boundary

All due PA2 bare-tool behavior groups are implemented and validated. The next
assignment boundary is PA3 controlling expressions over these typed literals;
PA4 introduces preprocessor input and interned identifier identities. Do not
advance either stage from this implementation ledger: Ralph owns that decision.

Source-map partials retain later consumers' joined-spelling traversal helper,
preprocessor/interned-input APIs, GNU reserved-suffix/dialect controls, extended
floating/interchange formats, user-integer value demand and non-PA2 type/token
queries. They are not implemented through a fallback or forced into the PA2
C++11 dialect. The joined-spelling helper is unnecessary for this parse-once
sequence store; port its later source-faithfulness consumer when due.

Architecture review should check scalar/code-unit separation, prefix unification
before encoding, compact private piece storage, borrowed lookahead validity,
capacity reuse, error/output demand ordering and the alternative-operator
suffix lookup. Parsing, semantic demand and per-function backend lifetimes
remain future-stage obligations, not certified by this lexical handoff.

## Independent audit findings, 2026-10-11

Audit entry: `d71823ceb4e9ed239f147be49c61286296c3f5b4`. Direct typed
comparisons exposed a mismatch absent from the PA2 writer: the frozen bare
lexer overwrites the `first` PPToken reference while finding the end of a
string run. C++ therefore publishes the lookahead token's begin offset as the
string's begin, including when it exceeds the string's end. For
`"abc" "def" 7`, the frozen span is `(12, 11)`, not `(0, 11)`. Rust now
records the same deterministic frozen fact explicitly, without aliased mutable
storage. The literal-operator split still uses its original begin and never
pulls a lookahead. This baseline quirk is reported and preserved; the oracle
and references are unchanged. The owning Rust unit regression executes in
both workspace test profiles; a PA2 text fixture cannot expose these spans.

New test-only `scripts/audit_pa2_tokens.py` compares kinds, spans, spellings,
simple identities, forms, suffix/prefix slices, types, scalar/array bytes,
multicharacter packing, sequence sizes, repeated EOF and error ordering.
Entry evidence was 14/842 matching, with 828 span mismatches; the fix passes
842/842. Raw results and probe builds are under
`/home/vishvananda/work/private/v5rust/artifacts/pa2-audit/typed-{entry,release}/`.

The text audit's joined mixed stream stopped on a source error at line 29,
after only 18 output lines. Its advertised 1,200 generated mixed cases were
not all executed. The generator now runs each mixed sequence independently,
and requires every remaining grouped stream to reach EOF. Final supplemental
execution and the complete acceptance record follow in the final audit
increment. All protected gates already pass after the span fix: PA2 26/26,
original course target, PA1 58/58 plus course, preservation 26/26, preflight,
strict quality, independent file audit and 17 harness tests.

The span fix and independent typed audit are committed at `2d0c123`.
Further CLI probes exposed a second mismatch: a failure decoding the initial
logical character is caught by C++'s outer exception handler, with no source
location, because its `PPLexer` constructor precedes the inner source-error
handler. Rust previously rendered every source error with a location. The
post-token renderer now returns a distinct structured `SourceInitialization`
error; the CLI renders its message alone. Errors while consuming the stream
still carry locations, and PA1's constructor remains in its located handler.
`SourceError` implements standard `Display`/`Error` without changing its
stored physical offset or lazy location rendering.

New executed PA2 course cases are
`tests/300-initial-source-decode-error.t` (BOM/trigraph splice then invalid
UTF-8) and `tests/300-later-source-decode-error.t` (one emitted integer before
a located failure). Their new rejection-status sidecars come from the frozen
oracle; existing references and the 26-case manifest remain unchanged. The
owning renderer unit checks both typed error variants and retained output.
The supplemental text generator adds all 49 initial/late decoding combinations
of BOM, splices, whitespace, integers and strings with malformed UTF-8.
Stream/CLI evidence improves from 46/50 to 50/50 in both profiles combined.
All protected gates pass again, with the original full course target now
executing 28 cases including both added regressions. Final resource and
requirement-by-requirement acceptance evidence is recorded below.

## Full-stage acceptance audit

Reviewed the stage base `6f0c1b9`, numeric implementation `8ca05d6`, text
implementation/audit entry `d71823c`, span fix `2d0c123` and final implementation
`ab3105f`. The preceding implementation was progress: it committed the complete
text group and reduced twelve fixed-manifest failures to zero. This audit
revalidated the current checkout instead of relying on reused green results.
Read the full handout, contract, specification, conversion plan, stage ledger
and source map, and compared the C++ owners in the table above directly with
their Rust owners. The frozen oracle revision remains
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1`.

The two compiler findings and the supplemental-coverage finding are resolved
in the commits above. The deterministic C++ string-span overwrite is a
reported baseline quirk, reproduced safely; no reference correction or oracle
change was made. Fixed-manifest failing IDs: `[]`. Supplemental failing IDs
in each profile: `[]`. Every one of the 26 passing identities at entry is
present and passing in the final result; the independent set comparison is
retained alongside the protected 26/26 preservation result.

### Responsibility review and completeness

| Due responsibility | Review and evidence |
| --- | --- |
| CLI and phase 1–3 integration | Same input/BOM/newline/stream policy, ignored arguments and no-runner batch sentinel. Constructor failures use the outer message-only handler; later lexical failures use physical locations and retain exactly the preceding output. PA1's different handler scope is preserved. Stream probes include seek offsets, directory stdin and sticky `/dev/full` output. |
| Simple tokens and invalid pp-tokens | All C++ spelling/name associations, digraph identities, alternative operator words and reserved hash tokens reviewed. `true`, `false`, `nullptr`, `new` and `delete` remain simple identities. Unknown identifiers, non-whitespace characters and header names follow the same dispatch. |
| Integer classification and type order | Same strict C++11 grammar, malformed-exponent fallback, octal restrictions and same-case `ll`/`LL` suffix rule. Standard suffixes precede user-defined classification. Decimal/nondecimal candidate lists and equal-width long/long-long identities preserve C++ order. Checked accumulation rejects values beyond 64 bits; user-defined integer values remain unevaluated. |
| Floating classification and bits | Standard suffixes and underscore user-defined forms match. Overflow to infinity is invalid; underflow/subnormal transitions and halfway parity round to nearest, ties to even. Binary32/binary64 use their own precision; x87 retains 64 significand bits and six zero padding bytes. Limb bounds, kept digits/sticky tail, exponent saturation and carry rounding were reviewed against the frozen numeric/interchange owners and C-library scanner behavior, with generated exact dyadic boundaries for every due format. |
| Character literals | Encoding/body/suffix boundaries, simple/UCN/octal/hex escape consumption, 32-bit numeric-escape saturation, scalar validity and single-code-unit restrictions match. Ordinary ASCII is `char`; other ordinary scalars are `int`. Unprefixed multicharacter packing wraps explicitly in 32 bits and remains invalid in the PA2 view; prefixed and user-defined multicharacter forms reject. |
| Maximal string sequences | One prefix and one suffix identity apply to the complete run before encoding. Conflicts invalidate its complete space-joined spelling. Numeric escapes contribute one range-checked code unit; scalar sources encode as UTF-8/16/32, including surrogate pairs. Raw bodies retain physical characters. Exactly one zero unit terminates valid arrays; target lengths and element counts deliberately narrow to 32 bits. |
| Pull order and typed facts | One non-string lookahead terminates a run; lexical failure during that demand prevents output of the run. Literal-operator splitting emits two tokens before further demand, independently classifies every suffix keyword/operator and preserves splice-adjusted spans. Bare-run begin spans reproduce the frozen overwrite; source/array bytes still name the complete run. Typed probes cover valid/invalid runs, packing, scalar bits, suffixes, spans and three repeated EOF requests. |
| Writer | All five categories, form/type names, source joining, suffix/prefix extraction, array sizes, uppercase little-endian dumps, newline/EOF output and rejection prefixes match exact raw comparisons. It consumes typed facts; output is never reparsed into production state. |

These cover the handout's complete PA2 surface and every C++ responsibility
reachable through its strict bare-lexer tool, including behavior absent from
the original fixtures. `IdentifierTable`/arena interning is not used by the
C++ bare lexer; it belongs to PA4's preprocessor input. The linked GNU builtin
registry is consulted only by later type/builtin queries. GNU/reserved-suffix
dialects, binary/hexadecimal floating forms, extended interchange types,
user-integer value demand, joined-spelling traversal and later token/type
queries remain explicitly partial for their later consumers. They are not
required by this stage and do not execute through a fallback. PA33's global
ownership census and release/debug lanes are future conditional obligations.

### Architecture and Rust quality

The candidate's PA2 work executes entirely in Rust: the complete course trace
records 28 invocations of `obj/rust/release/posttoken`, all matching C++.
The adapter invokes C++ solely for evaluation/comparison and course transport.
External typed probes are test consumers built from the frozen sources and
current Rust library; they never run as candidate tools. No compiler process
fallback, fixture recognition, stored answer, new dependency or environment
read under `dev/src/` exists on the due path. Cargo.lock contains only the
workspace package, and unsafe code remains forbidden.

One immutable source image, one lexer scratch/current record, one lookahead
kind and one current string store own the lexical data. Loans prevent mutation
while consumers hold token slices. Compact private piece ranges and parsed
body/suffix facts are retained once; final encoding consumes them after
prefix unification. Scalar/type/form identities are enums, not rendered-text
keys. Invalid language values, source initialization, later source failures,
writer failures and host-size limits remain structured outcomes. Owned
buffers keep capacity and clear contents after their last consumer; floating
scratch is bounded by the target format independently of mantissa length.

No parser speculation, semantic cache, template demand or backend lifetime
exists yet on this path. Their transactional parsing, complete keys, event
scheduling and per-function ownership requirements remain with their owning
stages. The reviewed PA2 demand points are explicit, and no text transport,
whole-unit token collection, mechanical source split or hidden phase dependency
was introduced. Optimizer, register allocator and backend policies are unchanged;
PA35 optimization objectives were not imposed.

Rustfmt and strict all/pedantic Clippy pass with no new lint expectation or
allowance. Both profiles pass 28 library and four CLI tests, including the
new span and handler-scope regressions. The authoritative companion audit
checks 39 files with zero findings; `shape.json` records maxima of 436 code
lines per file, 94 per function and nesting four. These satisfy the source,
function and nesting limits, including the registered vocabulary roles.

### Final commands and artifacts

Current audit root:
`/home/vishvananda/work/private/v5rust/artifacts/pa2-audit/`.
`inputs.json` records the base/entry/reviewed/oracle commits, toolchain,
source hashes, manifest and binary hashes. Current `posttoken` SHA-256 is
`3f036cadea23f9ce87e9547e74a6abac24d83e21d074bc7bd59143e5d6f3fa86`
in release and
`cd8c1af9f50c6f58e520b68fed4bc31ccaff2b6efe7d76f82ec6d58b0cd651d4`
in debug. Final supplemental/performance receipts match these binaries.
`*-command.json` records each exact command, duration, status and log;
`commands.json` collects them. `completion.json` checks final counts, routes,
hashes and entry-case preservation. Entry mismatch receipts are intentionally
retained as reproduction evidence, not passing validation.

| Command | Final result; artifact relative to the audit root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `cargo build --manifest-path dev/Cargo.toml --workspace --locked --target-dir obj/rust` | PASS; `debug-build.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa2` | PASS, 26/26; `stage.log`, `pa2-result.json`, `pa2-cases/` |
| `make course-pa2` | PASS, original full `pa2 test` target plus new regressions, 28/28, no failure marker; `course.log`, `pa2-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa2` | PASS, PA1 58/58 and complete original course suite; `prior.log`, `pa1-result.json`, `pa1-cases/`, `pa1-course/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa2` | PASS, frozen policy/tools/sources/inputs unchanged; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa2` | PASS, 26/26; `preserve.log`, independent `entry-stage-result.json` comparison |
| `make lint` (`python3 /home/vishvananda/work/v5rust-control/clone_quality.py`) | PASS, rustfmt/strict Clippy/tests in both profiles/companion audit; `quality.log` |
| `make audit-rust` | PASS, 39 files/zero findings; `audit.log`, detailed `shape.json` |
| `make test-harness` | PASS, 17 protected checker tests; `harness.log` |
| `python3 scripts/check_fixture_lanes.py` | PASS, every input selected by an executed lane; `fixture-lanes.log` |
| `python3 scripts/audit_pa2_tokens.py --evidence <root>/typed-release` and `--evidence <root>/typed-debug --profile debug` | PASS, 842/842 direct typed comparisons per profile; `typed-{release,debug}.log`, `typed-{release,debug}/results.json`, probe sources/build commands/raw results |
| `python3 scripts/audit_pa2_text.py --evidence <root>/text --profile release` and `--profile debug` | PASS, 1,308/1,308 exact comparisons per profile: 6,004 generated sequences, 75 boundary/CLI probes and 28 frozen/new course inputs; `text-{release,debug}.log`, `text/{release,debug}/results.json` |
| `python3 scripts/audit_pa2_numbers.py --evidence <root>/numeric --profile release` and `--profile debug` | PASS, 4,767 vocabulary/numeric inputs and eight CLI probes in 14/14 comparisons per profile; `numeric-{release,debug}.log`, `numeric/{release,debug}/results.json` |
| `python3 <root>/streams.py` | PASS, 50/50 stream/CLI comparisons across both profiles; `streams.log`, `streams.json`, entry mismatch `streams-entry.json` |
| `python3 <root>/measure.py` and `python3 <root>/numeric-cost/measure.py` | PASS, paired output equality at every scale; `perf.log`, `perf.json`, `numeric-perf.log`, `numeric-cost/perf.json` and raw run files |
| `git diff --check` | PASS |

The protected manifest digest remains
`d3a98a505d9872f5a8b579b2f2b9937c606c0fcf6f372c9f7f2d1f5bfe9a3120`.
New rejection sidecars supplement the full course lane; they do not change
the fixed gate, protected files, acceptance policy or existing reference bytes.

### Resource observations on final binaries

Each workload uses two Rust A/A warmups followed by two paired ABBA blocks.
Measurement jobs ran individually after the main build/differential jobs
finished. `/usr/bin/time` reports wall time and RSS; all corresponding
outputs are byte-identical. Input/binary hashes and every command/sample are
retained in the performance receipts. The text workloads and sizes are the
same supported inputs described in the implementation checkpoint above.

| Workload | Source bytes | Rust median ms / max RSS KiB | C++ median ms / max RSS KiB |
| --- | ---: | ---: | ---: |
| streamed-text | 174,080 | 5 / 2,236 | 0 / 3,644 |
| streamed-text | 696,320 | 30 / 2,936 | 20 / 4,344 |
| streamed-text | 2,785,280 | 110 / 5,068 | 75 / 6,460 |
| maximal-sequence | 122,880 | 0 / 2,936 | 0 / 4,372 |
| maximal-sequence | 491,520 | 10 / 3,628 | 10 / 6,460 |
| maximal-sequence | 1,966,080 | 50 / 9,276 | 50 / 15,356 |
| frozen-numeric | 71,456 | 0 / 2,236 | 0 / 3,640 |
| frozen-numeric | 285,824 | 10 / 2,232 | 0 / 4,344 |
| frozen-numeric | 1,143,296 | 60 / 2,940 | 35 / 5,048 |
| long-mantissa | 192,080 | 30 / 2,200 | 0 / 3,640 |
| long-mantissa | 768,320 | 100 / 2,940 | 10 / 4,344 |
| long-mantissa | 3,073,280 | 425 / 5,052 | 30 / 6,480 |

The numeric seed joins the unchanged floating/integer/simple fixtures; the
long-mantissa seed is `1.` followed by 12,000 zeroes and `1L`, exercising
bounded scratch and sticky-digit scanning. Time/storage increase with source
size; maximal sequences retain their required complete run, while ordinary
streams reuse storage between tokens. Exact Rust long-double conversion is
slower than libc on this workload; no throughput equality or generated-code
optimization is claimed. Ten-millisecond sample granularity, warm caches and
local scheduling limit interpretation; zero means below reporting resolution.
Additional retained audit evidence is approximately 511 MiB, within the shared
100 GiB storage ceiling. Generated files are outside the commit.

Handoff: PA2's complete due behavior, exact diagnostics/bytes, original full
course coverage, earlier-stage preservation and Rust architecture/quality are
accepted at `ab3105f`. The next dependency boundary is PA3's controlling
expressions over these typed literals, followed by PA4's interned/preprocessed
input. The frozen bare-lexer span quirk must not be generalized to a future
input cursor with a different record lifetime without comparing that owner.
Ralph owns advancement; its state and existing goal were not replaced or edited.
