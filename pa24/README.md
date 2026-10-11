## CPPGM Programming Assignment 24 (lowir2native)

### Overview

Write a C++ application called `lowir2native` that takes as input a set of LowIR source
files and writes a native executable program.

PA24 implements the native backend for the LowIR model introduced in PA8.
It lowers LowIR directly to native code and native program data. Earlier
behavioral exercises used the supplied native reference; this assignment
builds the student's backend.

The intent of this milestone is:

- keep LowIR as the long-term compiler backend boundary
- learn instruction encoding, address fixups, and executable layout here
- leave room for later optimization and additional native backends

### How PA24 Is Specified

This README separates three kinds of statement, and the course suite is
built the same way.

- The **contract** is what every implementation must do: the command line,
  the LowIR family it accepts (Assignment Boundary), the machine-IR dump
  format, the x86-64 ABI, and the behaviour of every generated program.
- The **quality bar** is what the generated code must reach on the course
  fixtures: the program behaves, and the machine IR stays inside each
  fixture's size envelope (`x.ref.expect`) or, for the fixtures whose
  shape is the contract, matches the canonical dump.  The Testing section
  states it.
- **One design** is a way to lower LowIR: the five-stage structure and
  compact shapes in the linked design notes.  A different lowering that meets
  the bar is a correct PA24; only the contract-shape fixtures compare your
  instruction selection with the course solution's.

Run `make test` from this assignment directory to check the course contract.
See [Testing and references](../TESTING_AND_REFERENCES.md) for selecting tests.

### Prerequisites

You should complete Programming Assignment 23 before starting this assignment.

You will want to reuse:

- the PA8 LowIR parser and LowIR specification
- any shared lowering or assembler abstractions that help you separate:
  - LowIR -> native instruction selection
  - native code/data emission
  - final executable image construction

Work through [native encoding and executable layout](native-encoding.md)
while implementing these pieces. It introduces a minimal ELF image, native
operands and instruction bytes, labels and fixups, and inspection tools.
No earlier assembler implementation is assumed.

PA24 tests execute generated native programs. Your development host therefore
needs an x86-64 Linux execution environment. With no `--target`, the tool should
emit a Linux executable. The target name used by the course is `linux`.

### Starter Kit

The starter kit contains:

- `pa24/README.md`, `pa24/Makefile`, and the test scripts in `pa24/scripts/`
- a student-editable `dev/lowir2native.cpp` starter scaffold
- the `pa24/lowir2native.cpp` symlink back to `../dev/lowir2native.cpp`
- shared support sources and headers under `dev/src/`
- the MIR vocabulary in `dev/src/mir_model.h` and
  `dev/src/native/mir/registers.h` (operations, operand kinds, registers,
  condition codes and the System V register sets), with the LowIR vocabulary
  in `dev/src/lowir_model.h` and `dev/src/ir_symbol_model.h`
- a local test suite under `pa24/tests/`
- the grammar for this assignment called `pa24.gram`
- the authoritative LowIR specification in `../pa8/lowir.md`
- an HTML grammar explorer of `pa24.gram` in the sub-directory `grammar/`
- checked-in golden result files under `tests/`
- `tests/strict/` for raw-MIR oracle tests
- `tests/structural/` for canonical-MIR oracle tests
- `tests/behavior/` for generated-program behavior tests without a machine-IR oracle
- `tests/controls/` for focused property checks that inspect only the
  documented MIR or native relationship and generated behavior, never a
  complete MIR dump or executable image
- `tests/regression/`, which pins the course solution's own MIR shapes and
  selection policies (`controls/`); it is
  outside your exit criteria, and `make test-regression` runs it on request

Students should implement the assignment in `dev/lowir2native.cpp` and any reusable
student-owned helpers they add under `dev/src/`. The assignment directory, grammar files,
test fixtures, comparison scripts, and checked-in reference outputs are support
files, not implementation files to edit for normal solutions. Reuse your earlier compiler infrastructure when implementing this milestone.

Use `lowir2native-ref` to inspect example output. Normal tests invoke the student's `lowir2native` and compare
with the checked-in `.ref` results; they never substitute the supplied backend.

### Driver Surface For This Assignment

Required in PA24:

- `--help` / `-h`
- `-o <outfile>`
- `-O0` (also the default)
- `--stats`
- `--dump-machine-ir <mirfile>`
- `--target <target>`

Not yet required here:

- separate compilation through `-c`
- link-map dumping
- the private exception/runtime ABI path

Those later pipeline surfaces are owned by the later `cppgm++` object,
compile/link, and host-EH assignments.

### Input / Command-Line Arguments

Behaviour is undefined unless the command-line arguments match one of:

    $ lowir2native --dump-machine-ir <mirfile> <srcfile1> <srcfile2> ... <srcfileN>
    $ lowir2native -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>
    $ lowir2native --dump-machine-ir <mirfile> -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>
    $ lowir2native --target <target> --dump-machine-ir <mirfile> <srcfile1> <srcfile2> ... <srcfileN>
    $ lowir2native --target <target> -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>
    $ lowir2native --target <target> --dump-machine-ir <mirfile> -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

where each `<srcfileK>` is a LowIR source file and `<target>` is `linux`.

With no `--target`, `lowir2native` should emit a native Linux executable.

### Output Format

If `-o <outfile>` is provided, `lowir2native` shall write a native executable
program to `<outfile>`.

If `--dump-machine-ir <mirfile>` is provided, `lowir2native` shall also write a
deterministic machine-IR dump to `<mirfile>`.

The machine-IR dump is the serialized form of the backend model used for native
emission. You may keep a typed MIR internally, with `dev/src/mir_model.h`
supplying its vocabulary, but the dump must describe the same program that
native emission consumes.

Integer immediates must preserve the complete input value, including values
that use the high half of an `i128`. Floating immediates must be interpreted as
their stated f32, f64, or f80 type so that globals, computations, and native
data contain the correct target value. The same values must appear in the
deterministic MIR dump and reach native encoding without loss.

The deterministic MIR dump must retain the required parameter names,
frame-slot display names, literal spellings, and debug source locations.

Frame metadata is part of that final MIR contract. In particular, the
callee-saved `preserve` list should name the callee-saved registers that the
final instruction body actually uses after local setup/copy cleanup, and the
stack size should match that final frame layout.

The frame section also records the final native layout policy.  It contains
`frame_pointer keep|omit` and `epilogues shared|duplicated` (one epilogue
that every return reaches, or one per return); these are the exact facts
consumed by prologue/epilogue and unwind emission, not optimization hints.
Which policy a level uses is yours; the course solution keeps the frame
pointer and shares epilogues at `-O0`.

`--stats` writes one diagnostic record to standard error after successful
lowering and encoding. Its counters are the course solution's own (for
example `scratch_carried_reloads`, the frame reloads its `-O0` selection
satisfied from a carried value); no course lane requires any of them, and
only the regression lane reads one.

That MIR dump path must work even for helper-only LowIR inputs that have no
entry function. In that case the dumped MIR should simply omit the optional
`startup` section.

For the native path, that means an ELF executable.

The exact binary encoding is not directly compared by the PA24 tests. Instead, the tests
compare:

- the compiler exit status
- the fixture’s machine-IR expectations for successful compilations
- the generated program exit status
- the generated program standard output

### Error Handling

If an error occurs while parsing LowIR, validating LowIR, lowering LowIR, or writing the
native output, `lowir2native` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of `lowir2native`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

From this assignment directory, run `make test`. From the repository root,
run `make test-report-through-pa24` before moving on.

The suite compiles each LowIR input, records the compiler exit status, and
runs successful programs to compare their standard output and exit status.
It also checks the machine-IR dump:

- With `x.ref.expect`, every stated outcome and size bound must pass. The
  adjacent `.ref.mir` is an example; your instruction sequence may differ.
- Without an expectation, `tests/strict/` compares `.ref.mir` after target
  header normalization, and `tests/structural/` compares `.ref.cmir` after
  canonicalization. These cases check ABI and other required relationships.
- `tests/behavior/` checks program results. Its reference MIR is informational.
- `tests/controls/` checks focused MIR, ABI and native execution properties,
  each named by its file: a shift, or an indexed address and a shift, for a
  multiply by a constant; a small copy's frame operands; a direct access of a
  locally bound global; a copy of 32, 48 or 56 aligned bytes made of direct
  chunks rather than a string operation; the `unreachable` terminator; the
  storage a by-address argument gets; and a 32-bit `and` for an
  upper-clearing mask. The course solution's other selection policies (its
  copy and zero thresholds, its `-O0` frame-pointer and epilogue choice, its
  switch compare chain, its carried reloads and parameter carriers) are
  checked only in `tests/regression`.

Canonicalization absorbs interchangeable free registers and frame offsets;
it preserves instruction families, widths, ordering and operand classes.
The harness writes `.my.cmir` for diagnosis. Your backend emits only raw MIR.
Failed compilations are judged by exit status, not diagnostic wording.

A `.ref.expect` bounds quantities such as instructions, blocks, calls, memory
operands, pushes and frame size. Read the sidecar to see the bound for a
failing case; `scripts/expect_ir.pl` reports which predicate failed.

### Machine-IR text

For a successful compilation, the tested raw MIR dump is a plain-text file with this overall
shape:

```text
machine_ir x86_64 <target>

startup
    ...

global @name
  ...

function @name
  abi
    ...
  frame
    ...

  block ^label
    ...
```

The exact instruction inventory is target- and lowering-dependent, but the output format used
for testing is still this textual machine-IR form:

- one `machine_ir x86_64 <target>` header
- an optional `startup` section
- zero or more `global @...` definitions
- one or more `function @...` definitions
- per-function `abi`, `frame`, and ordered `block ^...` sections
- one instruction or metadata line per indented row beneath those sections

Machine operands use the following target-specific forms:

- `rax` through `r15` for general-purpose registers
- `xmm0` through `xmm15` for scalar floating-point registers
- an integer or floating literal for an immediate value
- `@name` or `^label` for a symbol or block label
- `[register]` or `[register+displacement]` for register-based memory
- `[base+index*scale+displacement]` for indexed memory, where `scale` is
  `1`, `2`, `4`, or `8`; `*1` and a zero displacement are omitted
- `[rbp+displacement]` for a frame location

A direct call names a symbol, while an indirect call prefixes its register or
memory target with `*`.  A call may also carry bracketed machine facts:

```text
call @consume [args=(rdi,rsi,xmm0), stack=16]
call *r10 [args=(rdi), variadic, unwind=no]
```

`args=(...)` lists the physical argument registers read by the call.  An empty
list is written as `args=()`.  `stack=N` records the bytes of caller-owned stack
arguments, `variadic` records a variadic call boundary, `unwind=no` records a
non-unwinding call, and `returns=noreturn` records a call that does not return.
These facts are part of MIR because register liveness, stack cleanup, exception
handling, and native emission all depend on them.

The sidecars' predicates count mnemonics as `dev/src/mir_model.h` spells
them, so the dump's vocabulary matters where a design could spell a thing
two ways: a memory access is a `load.<type>` or `store.<type>` instruction,
not a `mov` with a memory operand; the course exception model's regions are
`eh_push` and `eh_pop`; and the prologue and epilogue are implicit in the
frame facts (`stack_size`, `preserve`, `frame_pointer`, `epilogues`) rather
than dumped as `push`, `pop` and `ret` sequences, so a `preserve` list is
what the function saves.

### PA24 Syntax Spec

The authoritative input-language syntax for PA24 is `pa24.gram`.

As in the earlier assignments, that grammar defines accepted input syntax only. The native
program behaviour contract is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

PA24 does not add new LowIR syntax beyond PA8. It reuses the same LowIR language and adds
a new backend target for it.

That means the expected PA24 input surface is the current PA8 LowIR surface, not the older
pre-metadata subset. In particular, handwritten PA24 inputs may now use:

- explicit function role metadata such as `[role=entry]`, `[role=init]`, and `[role=fini]`
- top-level declaration forms such as `declare function` and `declare global`
- structured global data plus explicit global storage metadata where relevant
- call-boundary, parameter, and instruction-debug metadata described in
  [the LowIR format](../pa8/lowir.md)

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa24.gram` as the source of truth.

If this README and `pa24.gram` appear to disagree about LowIR syntax, treat `pa24.gram` as
authoritative. If this README and `../pa8/lowir.md` appear to disagree about the full LowIR
definition, treat `lowir.md` as authoritative. If they disagree about the required PA24
implementation subset, treat the `Assignment Boundary` and `Out Of Scope` sections below as
authoritative.

### Assignment Boundary

PA24 must support native lowering for the LowIR family already defined for PA8 and used by
PA10-PA23, including:

- scalar globals and structured global data
- functions, blocks, slots, temporaries, and runtime hooks
- direct and indirect calls
- control flow, integer operations, and pointer/index operations
- `phi` value merges as parallel transfers on their incoming control-flow
  edges, including loop backedges and critical edges
- floating scalar operations and comparisons over `f32`, `f64`, and `f80`
- explicit scalar conversions:
  - `sitofp`
  - `uitofp`
  - `fptosi`
  - `fptoui`
  - `fpext`
  - `fptrunc`
- atomic scalar operations and fences over `i1`, `i8`, `i16`, `i32`, `i64`, and `ptr`
- bulk memory operations:
  - `copyobj`
  - `zeroinit`
- object-lowered ABI forms emitted by source-to-LowIR assignments:
  - hidden destination-pointer returns
  - lowered object parameters carried as `ptr`
- direct one- and two-eightbyte object parameters and results in the supported
  x86-64 ABI, including padded homes for partial second eightbytes, with each
  eightbyte in a GPR or an XMM register (or nowhere) as the parameter's
  `classes` or the function's `result_classes` metadata states, and the whole
  object on the stack for `classes=memory` or when its registers run out
- `select`, as a conditional move or branches, and the `ctz` and `clz` bit scans
- supported variadic calls and `va_start` register-save state for GPR and XMM
  arguments, including the caller-provided vector-register count
- structured vtable/global table data emitted by source-to-LowIR lowering
- structured global alignment derived from typed data items; raw `zero` byte
  padding inside mixed data does not independently raise alignment

Within this milestone, PA24 should successfully compile the LowIR emitted by PA10-PA23 into
native executables.

PA24 must also expose a deterministic machine IR for successful compilations. That dump is
a view of the target-specific representation that native emission consumes.

The LowIR input path should parse the same LowIR text accepted by PA8 rather
than relying on a private object, semantic, or source-level backchannel. Any
backend fact needed below PA24 belongs either in LowIR text or in the
target-specific MIR produced from that LowIR.

Within the supported subset, PA24 should lower:

- direct function calls to direct machine-IR call sites
- block control flow to direct machine-IR conditional and unconditional branches
- startup and shutdown hooks to direct machine-IR call sites in the startup path
- external C `atexit` registration to process-lifetime callbacks: after the entry
  function returns, invoke callbacks in reverse registration order before the
  shutdown hook, including callbacks registered by another callback
- bulk object-memory operations to first-class machine-IR `copy_bytes` / `zero_bytes`
  instructions
- truly indirect LowIR calls to machine-IR indirect calls, rather than forcing all calls
  through the same lowered shape
- structured global data to machine-IR global data blocks
- atomic scalar LowIR to first-class machine behavior rather than silently dropping the
  atomic contract in the direct backend
- the LowIR arithmetic and conversion forms in the native contract, including:
  - signed/unsigned integer division, modulus, ordered comparisons, and right shift
  - integer/float conversion operations
  - float-width extension and truncation operations
  - `f32`/`f64`/`f80` arithmetic and comparison behavior

What the course lane requires of your backend is the behaviour of every
program, the ABI relationships the strict and structural fixtures check,
the size budgets of the sidecars (generated from the course solution's dumps
with 100% room) and the controls under `tests/controls/`. The goals below
describe the course solution's `-O0` lowering within that frame: a shape a
strict or structural fixture or a control checks is the contract; a
selection rule no control names (the copy and zero thresholds, carried
reloads, the frame-pointer and epilogue policy, the switch compare chain) is
the course solution's choice, pinned only in `tests/regression`, and a
backend may choose otherwise within the budgets.

To complete PA24, implement these goals:

1. Direct control-flow lowering.
   LowIR branches, first-class `switch` dispatch, and direct calls should become
   first-class machine-IR branches and direct calls.

   For a `switch` with at least sixteen case edges, an integer-literal case
   should remain an immediate operand of its machine comparison.  When that
   value fits the target comparison's immediate field, encode the comparison
   directly instead of first copying the value to a scratch register.  A
   nonliteral case value keeps the ordinary register-materialization path,
   and smaller switches may use that path for every case.  This is the
   course solution's selection, checked only in `tests/regression`; a jump
   table is an equally valid lowering.

2. Direct startup/runtime wiring.
   The startup path should call `@__cppgm_init`, `@main`, and `@__cppgm_fini` as direct
   machine-IR call sites where those hooks exist.

3. First-class bulk object-memory lowering.
   `copyobj <bytes>x<align>` and `zeroinit <bytes>x<align>` should survive as meaningful
   machine-IR operations such as `copy_bytes <bytes>x<align>` and
   `zero_bytes <bytes>x<align>`.

   The operands of `copy_bytes` and `zero_bytes` name their address registers
   directly, without preceding MIR copies into fixed registers.

4. Preserve the distinction between direct and indirect calls.
   The direct backend should still emit indirect machine-IR calls for truly indirect LowIR
   calls, such as virtual dispatch, instead of collapsing all calls into one lowered form.
   That includes pointer-valued global cells: if a call target comes from a scalar `ptr`
   global, PA24 should call through the pointer stored in that global, not through the
   address of the global storage itself.

5. Preserve richer LowIR data layout.
   Structured global data and later vtable-like globals should remain structured in the
   direct backend rather than being forced through a scalarized compatibility path.

6. Exercise backend-owned execution behavior directly.
   PA24 is the right home for LowIR-native execution tests that validate the basic machine
   semantics before the later source-driver/toolchain milestones. The important cases are arithmetic, signedness-sensitive integer behavior,
   scalar conversions, and floating execution. Those tests are expressed in LowIR.

   In particular, PA24 should already treat unsigned LowIR arithmetic/predicate forms such as
   `udiv`, `umod`, `ushr`, `ult`, `ule`, `ugt`, and `uge` as first-class backend behavior,
   not as optional later cleanups.

7. Preserve direct compare-fed branch lowering for ordinary scalar cases.
   When a compare result feeds exactly one branch, PA24 should lower that as a direct
   machine compare plus conditional branch rather than materializing a boolean temporary
   and branching on that temporary afterward.

8. Keep simple scalar and floating work on the appropriate machine path.
   Small leaf scalar expressions should normally stay in registers, and ordinary `f32` /
   `f64` operations should stay on the floating-register path. A conservative stack spill
   is acceptable when pressure or an ABI boundary requires it, as long as the generated
   program is correct and the checked structural MIR cases still match their oracles.
   Lowering operations with fixed scratch registers, including integer comparisons,
   division, and shifts, must preserve still-live frame addresses and incoming parameters
   before reusing those registers.

   A numeric immediate written without a decimal point still follows the declared LowIR
   type in a floating store or return. It must be materialized as the requested floating
   value rather than routed through an integer-only move path.

   An integer LowIR immediate stored to a frame, global, dereference, or indexed
   destination remains an immediate value operand in MIR.  This includes an
   `i64` value outside the sign-extended 32-bit encoding range; native emission
   may materialize that value in a scratch register when encoding the store.

   Multiplication by a positive power of two uses a shift in native encoding.
   Multiplication by 3, 5, or 9 times a power of two uses one indexed-address
   calculation followed by that shift. These substitutions retain full-width
   wrapping integer behavior. Focused native controls check only those
   instruction families inside a tiny entry function; they do not compare an
   executable image or prescribe physical registers, prologue layout, or
   instruction bytes.

   The right operand of integer `add`, `sub`, `and`, `or`, and `xor` remains a
   frame, global, dereference, or indexed memory operand when that location is
   already selected.  Two-operand `imul` follows the same rule at 16, 32, and
   64 bits, and an integer comparison may retain one memory operand.  The
   result of a binary operation remains register-resident.  Division,
   variable shifts, byte multiplication, floating operations, and a form that
   would overwrite an address register before reading it still materialize the
   required value.

   A shift with a constant count retains that count as an immediate operand in
   machine IR and uses the target's immediate-count instruction.  A variable
   shift instead places its count directly in the target-required count
   register.

   A scalar copy may keep a stable source location, including an intact
   incoming parameter register, when the copied result's complete interval
   crosses no clobber and its source and result have the same machine
   representation. This includes a bit-preserving `ptr`/`i64` copy; width- or
   sign-changing conversions remain explicit. Such a copy should not add a
   machine move.

   A compiler-created scalar that is live across one adjacent block edge
   should remain in its selected register when the source has only that
   successor, the destination has only that predecessor, and the register
   survives every intervening operation.  A value crossing a call may use a
   callee-saved register for this purpose.  Joins, backedges, exception edges,
   address-taking uses, and clobbered registers still require conservative
   placement.

   Instructions with fixed register effects must preserve an unrelated live
   value already carried by one of those registers.  In particular, widening
   a scalar to signed `i128` uses both halves of the target register pair, so a
   later scalar operand in the high-half register must first receive another
   stable location.

9. Implement call-boundary correctness before any allocation cleverness.
   PA24 must respect the native calling convention for direct calls, indirect calls,
   mixed GPR/XMM arguments, variadic register-save state, stack arguments, scalar and
   direct-object returned values, and values that remain live across calls. The tests
   intentionally check some high-pressure call cases by program behaviour only; those
   cases should compile and run correctly but do not require the exact spill/register
   strategy used by the reference implementation.

   An integer-only call still clobbers caller-saved XMM registers, so a live `f32` or
   `f64` value must survive that call even when no floating argument or result is present.

   Hidden indirect-result arguments can shift ordinary pointer and reference parameters
   into different ABI registers. Forwarding those parameters after earlier scratch-using
   operations must preserve their original values too.

   A `ptr [pass=by_address]` parameter denotes an addressable-storage
   boundary, including a source reference after source lowering. If its actual
   argument is a scalar temporary or register result rather than an existing
   pointer, native lowering must give the value temporary storage, pass that
   storage's address, and preserve the callee's observable load/store behavior.
   This requirement is structural and behavioral; it does not prescribe a
   physical register, frame offset, or complete MIR dump.

   Eliminating a scalar parameter's initial store to and later load from a local slot
   must preserve the parameter across every intervening instruction that clobbers its
   incoming register, including a call or bulk-memory operation. The same selected
   value must be used when an object or wide argument makes the call use extended ABI
   classification. This requirement also applies at six or more integer or pointer
   parameters, where all incoming argument registers are occupied.
   While such a parameter remains live, its incoming register must not be assigned
   to a temporary result merely because the function has a wide parameter boundary.
   The register becomes reusable only after the parameter's final selected use.

   When a frame-resident object or wide-integer chunk is assigned to a GPR
   argument, MIR should load that chunk directly from its frame location. It
   should not materialize the object's base address solely for the load.

   Copying a stack-passed object may use the target's copy registers while
   preparing a call. A scalar argument whose source occupies either copy
   register must retain its value until its register or stack argument is
   written, including when the scalar follows the object on the call stack.

   A small bulk copy with a frame-resident source or destination may encode
   its scalar chunks directly from that frame operand; it need not materialize
   a temporary base address. The encoder must still preserve both logical
   addresses and any live scalar values that overlap its scratch registers.
   When both bulk-copy addresses need setup, forming one address must not
   overwrite a parameter or deferred carrier still needed to form the other.
   The lowering may reverse the setup order or stage one address in reserved
   scratch; generated behavior must remain correct.

   A direct three-argument call to the canonical builtin `memcpy` may become
   a dynamic `copy_bytes` machine operation when its returned pointer is
   unused.  After ordinary ABI argument setup, that operation consumes the
   destination, source, and runtime byte count from their calling-convention
   carriers and copies exactly that many bytes.  A call whose result is used,
   an indirect call, an ordinary unmarked function, or a call with a different
   argument shape remains a call.  The course solution applies this rule at
   every level; `tests/regression` checks it, and the course lane does not.

   A fixed 16-byte zero may avoid string-operation setup with a cleared
   reserved vector scratch and one unaligned store.  This direct form must be
   smaller than the corresponding setup for every address-register choice,
   preserve the integer condition flags, and must not consume an XMM register
   available to ordinary value placement.  Other zero sizes retain the
   existing exact target-byte comparison between direct scalar stores and the
   compact string form.  These are the course solution's thresholds, checked
   only in `tests/regression`.

   For fixed copies through 32 bytes, direct chunks may also avoid the setup
   for a string operation.  The direct encoder uses its reserved vector
   scratch for each complete 16-byte chunk and scalar chunks for the tail.
   This form extends through 64 bytes when the operation declares at least
   eight-byte alignment.  Larger or more weakly aligned copies retain the
   compact string-operation form.  `tests/controls` requires the direct
   chunks for copies of 32, 48 and 56 aligned bytes; where the course
   solution switches to the string operation is its choice, checked only in
   `tests/regression`.  Both forms must preserve the source bytes,
   destination bytes, and declared scratch effects; vector chunks may not
   consume an XMM register available to ordinary value placement.

   Native emission may carry a compiler-created scalar temporary from its one
   defining frame store to later typed reloads in the same block when the
   complete bounded window is safe. A carried window must not cross a call,
   bulk-memory or EH operation, floating/XMM operation, symbol/global access,
   large-immediate scratch use, or any explicit or implicit definition of the
   chosen carry register. Overlapping windows require distinct carry registers
   or retain their frame traffic. The conservative frame form is always valid;
   the course solution carries such values at `-O0`, and `tests/regression`
   checks that it does.

   Direct object returns follow the same rule in both directions: returning a
   frame-resident object loads its chunks directly into the ABI result
   registers, and storing a direct object call result writes those registers
   directly to its frame destination. In the course solution a destination
   address created before the call remains frame-shaped through the later
   result copy rather than occupying a register across the call
   (`tests/regression` checks it).

   An immediately returned integer quotient or remainder may use the fixed
   division result register directly. If its dividend is frame-, global-, or
   dereference-resident, MIR must first issue a typed load into a register;
   a register-only `mov` must not carry a memory operand.

   Atomic operations are subject to the same pressure correctness requirement. Producing
   an atomic operation's returned old value in a loop must remain executable when its
   address and source values occupy the available general-purpose registers.

   The same correctness requirement applies through control-flow joins and loop
   backedges. Incoming parameters, values computed before a loop, and values recomputed
   on each iteration must retain their current value across calls without a later
   iteration overwriting an earlier spill home.

   A `phi ptr` edge transfer whose source is a retained frame address transfers
   the address value, not the scalar contents stored at that frame location.
   The generated machine code must materialize the address before placing it in
   the phi destination.

   A representation-preserving scalar copy or decay may share its source's
   physical location, but that location remains live until the final use of
   every value that shares it.

10. Keep mixed-width conversion and floating-bool materialization explicit.
   Mixed integer/float conversion chains should keep their conversion family and width
   visible in MIR, and floating compare results used as values may materialize booleans
   in registers without an unnecessary stack round-trip.

11. Preserve narrow integer width behavior in MIR.
   Ordinary `i8`/`u16` compare-fed branches should stay visibly narrow, and small signed
   or unsigned integer arithmetic should show the expected post-operation normalization
   instead of silently widening into an untyped 64-bit path.

   A typed integer load defines the complete logical register value: narrow
   signed loads sign-extend and narrow unsigned loads zero-extend as part of
   the load itself.  Do not add a separate normalization instruction after
   such a load.  If native layout folds an address-setup instruction into the
   load, the combined encoding must preserve the same signed or unsigned
   extension.  Narrow values returned across a call boundary still require
   explicit normalization before a wider comparison or `switch`; stale upper
   bits must not affect branch or case selection.  A call result used only by
   an immediate same-width store or return may remain in its ABI result carrier
   without that normalization because only its low result bits cross the
   boundary.  An immediately following explicit integer extension or
   truncation also performs the required normalization itself.

   The same rule applies when a narrow temporary is resident in memory and a
   wider comparison consumes it.  Load and extend the temporary according to
   its own logical type before the comparison; a direct compare-fed branch must
   not read the wider comparison width from the narrow frame home, because
   adjacent frame bytes are not part of the value.

   A canonical typed integer immediate, a materialized Boolean, or an
   identical preceding integer extension already establishes the complete
   narrow value and should not be followed by a duplicate normalization.

12. Keep the conservative `f80` path explicit rather than implicit.
   PA24 does not need to treat `f80` like ordinary XMM-resident `f32`/`f64`, but its
   conversions and truncation/extension path should still stay visible and testable in
   MIR.

13. Cover direct compare-fed branch lowering at 64-bit and 128-bit integer widths too.
   The direct compare/branch quality rule is not limited to `i32` and `u32`. PA24 should
   show the same direct branch shape for straightforward `i64` and `i128`
   comparisons. An `i128` ordering comparison decides unequal high words with
   the predicate's signed or unsigned ordering and compares the low words as
   unsigned only when the high words are equal. It should branch without first
   allocating a scalar Boolean result. When an `i128` comparison is instead
   used as a value, register pressure must not make lowering fail; the
   materialized Boolean may use a temporary frame home when no GPR is free.
   The core ordinary-width oracle is the `500-i64-direct-compare-branch`
   family, and the course suite supplies the wide structural and pressure
   cases.

14. Keep pointer/null comparisons on the direct machine compare/branch path.
   Ordinary pointer/null tests should remain visibly pointer-typed in MIR and branch
   directly rather than degrading into a less explicit scalarized path, including
   null values first introduced through `const ptr 0`.

15. Keep pointer/index address calculation visible as pointer arithmetic.
   Pointer indexing and pointer-difference behavior should stay structurally visible in MIR
   rather than being hidden behind an unrelated compatibility path. When a one-use
   `index` feeds the following scalar load or store, that memory operand should
   contain the base, index, scale, and displacement directly. A zero index
   should not introduce a register copy or `lea`, and an unused `index` should
   not emit an instruction. When an indexed address must remain as a value, the
   MIR should use one `lea` rather than separate copy, multiply, and add
   instructions. An address of frame storage used only by the following scalar
   load, store, or constant index should remain frame-shaped: the memory
   operation should use `[rbp+displacement]` directly, and the constant index
   should incorporate its displacement without first materializing the base.
   A constant derived address whose uses are all load, store, index, or bulk-
   memory address operands may remain a base/index/displacement operand across
   intervening instructions and control-flow edges when its register carriers
   remain valid for the complete interval. It must be materialized when the
   pointer value itself is observed or a required carrier is clobbered.

   A scalar load or store of a locally bound global may keep the global symbol
   as its MIR memory operand and encode a direct PC-relative access. A
   preemptible or imported symbol must retain the indirect address path
   required by the object ABI. Taking or otherwise observing the address still
   produces a pointer value.

16. Preserve mixed integer/floating call ABI classification.
   Calls that mix GPR and XMM arguments should keep that classification visible in MIR so
   students can tell whether the backend is respecting the native calling convention.

17. Keep ordinary `f80` arithmetic and comparison behavior executable and visible.
   Even though `f80` remains the conservative floating special case, simple `f80`
   arithmetic and `cmp` behavior should still run correctly and remain explicit in MIR.

18. Exercise non-64-bit atomic widths explicitly.
   The PA24 atomic contract is not only about `i64`; smaller-width atomic load/store
   behavior should survive through the direct native backend. An atomic load
   result that remains live across a call must also have a frame fallback when
   every preserved GPR is occupied.

The PA24 tests exercise the `LowIR -> machine IR -> native` path and its
required execution and structural properties.

### Out Of Scope

The following are explicitly out of scope for PA24:

- separate compilation and linking
- relocatable object-file output
- exception-aware native runtime metadata
- optimization passes
- non-x86 native instruction selection
- source-level end-to-end runtime programs that depend on the later `cppgm++ -c`
  driver and object/library flow

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stages are:

- PA25 `cppgm++` compile/link mode adds source-driven separate compilation
  and linking on top of the native backend.
- PA26 adds host-compatible exception metadata in relocatable objects.

So PA24 should leave behind:

- a stable `LowIR -> native` lowering boundary
- a stable `LowIR -> machine IR` boundary that later optimization passes can target:
  a machine IR whose instructions state the registers they define, use and clobber,
  and the placement of values that live across blocks decided apart from
  instruction selection, so that PA33 can replace PA24's placement with a
  liveness-based allocator, and add machine-IR passes, without rewriting the
  selector. The conservative placement PA24 uses is one policy over that IR
  ([the design notes](design-notes.md) describe these seams)
- reusable target-specific code/data emission layers
- a backend test corpus that already catches the basic execution-level
  arithmetic and conversion bugs before the source-driven toolchain stages

### Design example

[Design notes](design-notes.md) work through a possible lowering pipeline.
The assignment contract and fixture expectations remain the requirements.
