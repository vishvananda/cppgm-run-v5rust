# PA2 port ledger

Stage base commit: `6f0c1b97b997e01289470a8220d658b9ef95bfc9`
Last audited commit: INITIAL
Text-group entry HEAD: `8ca05d6de479d2663b9a8f5437c1a79cfca6ff8a`

The PA2 implementation lane now passes its complete frozen manifest and original
course target: 26/26, up from 14/26 at this clean-checkout entry. The previous
turn was progress: commit `8ca05d6` implemented the shared numeric/simple group,
with twelve remaining status-86 text-literal failures. This increment implements
that entire shared text group. Ralph owns checkpoint/final audits and stage
advancement; this implementation does not advance the last-audited commit or
claim acceptance from fixture counts alone.

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
