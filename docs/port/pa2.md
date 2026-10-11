# PA2 port ledger

Stage base commit: `6f0c1b97b997e01289470a8220d658b9ef95bfc9`
Last audited commit: INITIAL

This is a coherent partial implementation handoff for PA2, not stage acceptance.
The implementation commit containing this ledger has the base above as its
parent. Ralph owns checkpoint/final audits and advancement; implementation does
not advance the last-audited commit. At entry the checkout was clean, `posttoken`
was a status-86 scaffold and the supplied fixed-stage result was 0/26. There
was no PA2 implementation progress to preserve from the interrupted entry;
porting the shared numeric owner was the next available action.

## Contract and ownership

Read `AGENTS.md`, `CLONE_CONTRACT.md`, `spec.md`, the complete `pa2/README.md`
and `docs/rust-clone-plan.md`. Consulted the frozen C++ snapshot
`905bd7e4412a2d77d88694fe6abc7f3e1c27bce1` directly.

| C++ responsibility | Rust owner and current scope |
| --- | --- |
| `dev/posttoken.cpp`, `lex/post_tokenizer.{h,cpp}`, `lex/post_token.h` | `dev/bin/posttoken.rs`, `dev/src/lex/mod.rs`: demand-driven bare-lexer simple/numeric conversion, borrowed spelling, compact physical spans, typed scalar facts |
| `lex/token_kind.{h,cpp}`, PA1 punctuator vocabulary | `dev/src/lex/vocabulary.rs`, `dev/src/lex/pp_lexer/punctuators.rs`: all PA2 keyword/operator lookup and term names; canonical new/delete keyword identities; hashes excluded from valid simple identities |
| `lex/number_literal.{h,cpp}`, `lex/ud_suffix.{h,cpp}` | `dev/src/lex/number.rs`: C++11 numeric grammar, suffix/type candidate ordering, checked 64-bit accumulation, underscore/Annex E UTF-8 suffixes and deferred user-defined numeric range |
| `lex/number_literal.cpp`, `lex/interchange_float.{h,cpp}` | `dev/src/lex/floating.rs`: decimal-to-binary32/binary64/x87 scanner, adapting the frozen limb algorithm for the standard target formats; no host float conversion or double rounding |
| `types/fundamental_type.{h,cpp}` | `dev/src/types/fundamental.rs`: distinct numeric types, target sizes/names and literal integer bounds |
| `lex/post_token_writer.{h,cpp}` | `dev/src/driver/posttoken.rs`: simple/numeric text views, uppercase little-endian hexdumps and user-defined prefixes |
| `preprocess/lexer/*`, `preprocess/source/*`, `dev/pptoken.cpp` | Existing PA1 algorithms moved to `dev/src/lex/pp_lexer/` and `dev/src/support/source/`; shared CLI stream policy in `dev/bin/source_tool/mod.rs` |

The authoritative audit forbids lex-to-preprocess dependencies. The relocation
puts phase 1–3 lexing below preprocessing and physical source storage below both;
`preprocess::{lexer,source}` retains public reexports for existing consumers.
The PA1 supplementary audit only changes its Rust source lookup paths and
predicate visibility. No checker policy, fixture or frozen source was changed.
The source map records the relocated owners and partial C++ APIs explicitly.

## Completed behavior group and invariants

All C++11 numeric forms share one classifier. Decimal versus octal/hexadecimal
candidate lists and unsigned/long/long-long suffix permutations match the C++
owner, including mixed-case long-long rejection, octal digit rejection and
64-bit overflow. A valid user-defined number retains a suffix boundary and
form, without a phase-7 value range check. Floating classification consumes an
exponent only when it has digits, preserving exponent-like integer suffixes.
Binary integers, hexadecimal floats and GNU/extended floating suffixes stay
invalid in the explicit C++11 dialect.

Floating scanning retains at most the format-dependent significant-digit bound
from the C++ interchange algorithm, plus one sticky digit. Decimal exponents
use its saturation rule and conservative magnitude bounds. Private little-endian
32-bit limbs form an exact numerator/denominator; division/remainder rounds
directly to 24, 53 or 64 significand bits, nearest with ties to even. Normal and
subnormal transitions, significand carry, overflow to invalid and underflow to
zero are explicit. x87 keeps its integer bit and initializes all sixteen storage
bytes, including six zero padding bytes. Two narrow cast expectations document
base-2^32 product/subtraction truncation; host-size conversions are checked.

One immutable byte image and one lexer scratch buffer own source spellings.
Whitespace is skipped before lending the next token, avoiding copies or a token
collection. Each returned post-token borrows its spelling, carries compact
physical `SourceOffset`s and contains its decoded scalar facts directly. The
writer consumes those facts and never reparses text. The scanner reuses six
private limb buffers and clears their contents after every result, keeping
capacity only. Bare-lexer identifiers remain borrowed, matching the C++ null
identifier pointer; interned preprocessor input belongs to PA4. No graph store
or artificial IDs are needed in this streaming group.

The shared CLI retains prior tokens on lexical errors, flushes before the
source diagnostic, honors seekable stdin's current position and the frozen
failed-stream behavior, ignores unrelated arguments and reproduces the
no-runner batch sentinel. Unported text conversion returns status 86, never
an ordinary rejecting-compiler status. No dependency, unsafe code, FFI,
process-based compiler implementation, environment read under `dev/src/`,
fixture recognition or reference-answer dependency was introduced.

## Validation evidence

Evidence root: `/home/vishvananda/work/private/v5rust/artifacts/pa2-numeric/`.
`inputs.json` records the base, frozen oracle, toolchain, source hashes and final
release/debug executable hashes. `stage-result.json` records the unchanged
manifest hash and individual case outcomes; `stage-cases/` snapshots raw trace,
course and mismatch evidence before the course run. `course-evidence/` retains
the original complete course target's counts, log and actual invocations.
`commands.json` records final command statuses; make's status 2 for the two
partial PA2 lanes reflects their underlying protected check's failure.

| Command | Result and artifact relative to the evidence root |
| --- | --- |
| `make build` | PASS; `build.log` |
| `cargo build --manifest-path dev/Cargo.toml --workspace --locked` | PASS; `debug-build.log` |
| `make test-pa2` / `python3 /home/vishvananda/work/v5rust-control/clone_check.py stage pa2` | Partial, 14/26; `stage.log`, `stage-result.json`, `stage-cases/` |
| `make course-pa2` | Partial, original target 14/26; `course.log`, `course-evidence/` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py prior pa2` | PASS, PA1 58/58 and original course target; `prior.log`, `prior-result.json` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preflight pa2` | PASS; `preflight.log` |
| `python3 /home/vishvananda/work/v5rust-control/clone_check.py preserve pa2` | PASS, historic PA2 identity set empty (0/0); `preserve.log`; prior PA1 preservation is established separately above |
| `make lint` / `python3 /home/vishvananda/work/v5rust-control/clone_quality.py` | PASS, rustfmt, strict Clippy, 19 library and four CLI tests in each profile, companion audit; `quality.log` |
| `make audit-rust` | PASS, 37 files, zero findings; `audit.log`, `shape.json` |
| `python3 scripts/audit_pa2_numbers.py --evidence <root>/supplemental --profile release` and `--profile debug` | PASS in both profiles: 4,767 generated numeric/vocabulary inputs across six streams plus eight CLI/source probes, 14/14 exact exit/stdout/stderr comparisons per profile; `supplemental-{release,debug}.log`, `supplemental/{release,debug}/results.json` |
| `python3 scripts/audit_pa1.py --evidence <root>/pa1-supplemental --profile release` | PASS, 10,783/10,783 raw comparisons; `pa1-supplemental.log`, `pa1-supplemental/release.json` |
| External rebuilt typed lexer probe, `python3 <root>/options/compare.py` | PASS, 1,280/1,280 option/span/error comparisons; `options.log`, `options/results.json` |
| `python3 <root>/measure.py` | PASS, output equality at all supported workload sizes; `perf.log`, `perf.json`, `perf/` |
| `git diff --check` | PASS |

Fixed manifest failures reduced from 26 to 12, with coverage still 26. Passing
IDs (all under `pa2/tests/`) are `100-integer-zero.t`, `100-simple.t`,
`200-basic-floating.t`, `200-basic-integer-suffix.t`,
`200-exponent-like-integer-ud-suffix.t`, `200-octal-limits.t`,
`300-floating-suffix.t`, `300-hex-limits.t`,
`300-incomplete-floating-exponent-bad.t`, `300-integer-limits.t`,
`300-invalid-floating-literal-shapes.t`, `300-multiple-decimal-points-bad.t`,
`300-user-defined-literal-range.t`, and `500-plus-ud-suffix.t`.

Exact remaining failing IDs (all under `pa2/tests/`):

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

Every remaining raw trace reaches the explicit text-literal status-86 boundary.
There is no numeric or earlier-phase mismatch hidden by a successful course
comparison. The supplemental corpus covers integer limits/suffix permutations,
malformed numeric shapes, exact halfway values and neighbors at every format's
subnormal/normal/top-binade boundaries, both parity choices, random decimals,
exponent saturation and long mantissas/sticky digits. Rust unit tests additionally
check physical spans, typed facts, repeated EOF and lexical-error precedence.
These supplement, and do not alter, the frozen acceptance lanes.

The source audit reports maximum implementation file/function sizes of 421/83
code lines and maximum function nesting four. Current responsibilities are
lexical/numeric; parsing, semantic demand and per-function backend storage are
not certified by this handoff. Optimizer, register allocator and code generation
were not changed. Generated-object/tool equality remains the PA34 obligation;
PA35 improvement targets are outside this phase.

## Resource observations

Measured the frozen `200-basic-floating.t`, `200-basic-integer-suffix.t` and
`100-simple.t` concatenated with newlines, repeated 16/64/256 times. Also measured
16/64/256 independent x87 literals with a 12,000-zero fraction and a final
nonzero digit, exercising the bounded scan rather than unported text behavior.
Each size uses two Rust A/A calibration runs followed by two paired ABBA blocks.
`perf.json` retains every command, input/binary hash, wall time, RSS and output
hash; every paired output is identical. Final measurements run after the other
validation processes finish.

| Workload | Source bytes | Rust median ms / max RSS KiB | C++ median ms / max RSS KiB |
| --- | ---: | ---: | ---: |
| frozen-numeric | 71,456 | 0 / 2,228 | 0 / 3,640 |
| frozen-numeric | 285,824 | 10 / 2,188 | 5 / 4,356 |
| frozen-numeric | 1,143,296 | 50 / 2,932 | 30 / 5,064 |
| long-mantissa | 192,080 | 20 / 2,240 | 0 / 3,644 |
| long-mantissa | 768,320 | 100 / 2,932 | 5 / 4,352 |
| long-mantissa | 3,073,280 | 375 / 5,028 | 30 / 6,520 |

Both source-size ladders show approximately linear time and memory scaling.
Ten-millisecond `/usr/bin/time` resolution makes zero entries below-resolution
measurements, not zero work; these are local warm-cache observations. Exact
integer-limb x87 conversion is slower than the frozen C library scanner on the
long-mantissa workload. Its arithmetic storage remains bounded by the format,
while source storage grows with input size. This is a measured implementation
cost, not a generated-code optimization change or a resource-equality claim.

Retained evidence occupies approximately 140 MiB within the shared 100 GiB
storage ceiling. Generated inputs, dumps, binaries and logs remain outside the
committed checkout.

## Handoff and next dependency boundary

The next coherent owner is `lex/text_literal.{h,cpp}` together with
`lex/string_concatenation.{h,cpp}` and the text branches of
`lex/post_tokenizer.cpp` / `lex/post_token_writer.cpp`. Preserve typed decoded
characters (numeric code unit versus Unicode scalar), prefix/suffix unification,
multicharacter facts, raw bodies, sequence source joining, one-token lookahead,
and the `operator ""suffix` split. Those shared rules explain all twelve
remaining failures; numeric conversion can now be reused unchanged.

Beyond that, source-map partials explicitly retain primary/alternative spelling
queries, interned preprocessor input, user-integer value demand and GNU/interchange
floating APIs for their later consumers. No complete assignment claim is based
on the passing fixture count. No oracle instability or harness defect was found.
Independent audit should review direct rounding against C-library semantics,
long-input digit bounds and scratch reuse, canonical token identity, translated
physical spans, and the relocated dependency direction. The checkout is to be
committed cleanly; Ralph decides advancement after this partial handoff.
