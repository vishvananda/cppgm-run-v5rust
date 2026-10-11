# v4 reference bug repairs

The report in `~/v4bugs.md` reproduced defects against source
`5ddcfe408e3d277825a6fc39a06f7f6bdeff7709`, student export `05cab5a6c`,
and bundle `cppgm-reference-binaries-linux-x86_64-5ddcfe408e3d.tar.gz`.
The fixes and regenerated outputs accompany this record. The next export's
`reference-binaries/manifest.tsv` identifies its source revision and checksums.

| Report | Repair and evidence |
| --- | --- |
| R1 | Namespace references with static storage duration materialize dynamic scalar and complete class temporaries in static storage. Semantic lifetime facts select the backing objects; construction guards ensure only the selected conditional temporary is destroyed. The PA12 control checks stack clobbering, converted scalars, member subobjects, both conditional arms and shutdown destruction. C++11 [class.temporary]/5 extends the complete object's lifetime. |
| R2 | Conditional class initialization retains each operand's value category when selecting its constructor. The PA12 control observes copies and moves on both arms of nested conditionals, while allowing permitted outer elision. See [expr.cond]/3–6 and [class.copy]/31. |
| R3 | Value-initialization zeros empty complete objects and members; placement `new T()` records the preliminary zero-initialization before the selected non-user-provided constructor. Empty bases and `[[no_unique_address]]` empty members retain their storage exceptions; injected anonymous-member aliases preserve the overlap fact. Controls cover complete/member storage, empty bases, padding, bit-fields, unions, user-provided constructors, `T()`, `T{}` and default initialization. See [dcl.init]/5,7 and [dcl.init.aggr]. |
| R4 | The scalar-reference fixture accounts explicitly for the global initializer's increment and each subsequent call. Its original expected arithmetic was wrong independently of R1. The corrected program returns zero. |
| R5 | Class/array subscript scaling converts the actual loaded index to i64 before i64 multiplication, including sign/zero extension. The existing PA11 fixture now covers negative signed and unsigned reference indices. Native conversion lowering reuses a dead source register and consumes the existing canonical-load normalization proof, avoiding redundant extensions at O0 through O3. The source-liveness control preserves a still-live narrow value. Existing generic IR-reader acceptance rules remain unchanged; broad arithmetic type enforcement needs a separate compatibility audit. |
| R6 | Explicit scalar initialization preserves destination volatility through aggregates, arrays, constructor helpers and declared volatile bit-fields. Preliminary class-storage zeroing remains unqualified. The policy is stated in `pa8/lowir.md`; PA11 fixtures distinguish volatile accesses from neighboring ordinary fields. Runtime exit equality cannot prove this marker property. |
| R7 | Defined TLS wrapper functions no longer share their symbol label with data. Declaration-only wrappers are callable, and standalone TLS addressing refers to the storage symbol. The native control executes both wrapper forms. |
| R8 | A cleanup clause synthesized for hosted exception tables no longer changes a generic native handler into a cleanup region. The cleanup/resume reducer now returns 14 without leaving an exception record on the stack. |
| R9 | The standalone backend supplies the canonical builtin strlen runtime when it has a declaration but no definition. It handles mutable, empty and long strings, indirect calls and existing definitions. Byte loads keep fallback scanning safe at page boundaries. Frontend object emission continues to use the hosted runtime. |

The Clang/libc++ 21 CI lane exposed an overlapping-member initialization regression. A reduced PA29 control covers direct, braced, temporary and conditional initialization plus anonymous template members. The original hosted map/unique_ptr program passes after preserving the destination member facts; initialization must not overwrite a neighboring live object.

The shutdown control also exposed missing standalone runtime metadata for
`__builtin_abort`; it now uses the existing termination runtime.

The R3 reducer returns zero with Clang and one with GCC in the investigated
host versions. This disagreement is retained: the repair follows the C++11
zero-initialization rule and the course's object-layout contract, rather than
compiler voting. Similarly, Clang's nonvolatile aggregate memset does not
settle R6; that repair makes the explicit LowIR initialization policy precise.

C++11 wording: [class.temporary](https://timsong-cpp.github.io/cppwp/n3337/class.temporary),
[expr.cond](https://timsong-cpp.github.io/cppwp/n3337/expr.cond),
[class.copy](https://timsong-cpp.github.io/cppwp/n3337/class.copy),
[dcl.init](https://timsong-cpp.github.io/cppwp/n3337/dcl.init),
[dcl.init.aggr](https://timsong-cpp.github.io/cppwp/n3337/dcl.init.aggr).
The IR-only cases follow [the LowIR contract](../../pa8/lowir.md).

## Validation

The reduced source cases and lifetime controls are checked at O0 through O3;
static-reference, conditional-transfer, value-initialization and strlen cases
also pass through hosted object emission and linking. The indexing runtime
benchmark produces byte-identical baseline/candidate executables at all three
measured levels (O0, O2 and O3).

Release gates are the full strict test report, debug-info suites, backend
variants, PA34 self-host through PA5, harness tests, architecture/placement
audits and CI's student-export validation. The pull request records the final
gate results and wall-time ABBA measurements, including A/A calibration.

Compiler timing must use the same allocator: the original local baseline
links glibc, while the rebuilt tool detects installed jemalloc. The controlled
campaign preloads the same jemalloc library for both immutable binaries.
Compiler time/RSS and generated runtime/code size are measured separately.
Frozen inputs, all observations, the superseded allocator-confounded campaign
and logs remain under `obj/v4bugs/`; no timing outliers are discarded.
