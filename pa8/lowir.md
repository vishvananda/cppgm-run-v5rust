# PA8 LowIR Specification

This document defines the LowIR format introduced in PA8 and extended by
later assignments. The [PA8 handout](README.md) specifies the required
reader/writer subset and the three program-construction exercises.

LowIR is intended to be the long-term backend boundary for the later lowering-heavy
assignments. PA8 implements the introductory model and text interface; the
overall structure is defined early so later assignments extend the same IR.

LowIR is deliberately:

- lower-level than the PA7 semantic dump
- independent of machine registers and instruction encoding
- structured enough to support later native backends and optimization passes
- explicit enough to construct small programs before implementing C++ lowering

## Core Design

LowIR is:

- block-based
- text-based and deterministic
- explicit about globals, functions, stack slots, temporaries, and control flow
- not required to be in SSA form, while providing an explicit merge operation
  for values selected by incoming control-flow edges

LowIR text is the serialized form of the compiler-owned backend program model.
Implementations may use a typed in-memory representation. The starter support's
`dev/src/lowir_model.h` and `dev/src/ir_symbol_model.h` give its vocabulary:
the format's fixed words as enums, with their spellings and readers. The text
remains the durable boundary. A backend-visible fact that cannot be serialized
to LowIR and parsed back is not part of the LowIR contract.

Sentences below that say what *the C++ lowering* does (where it keeps a
local, how it writes a conditional value or a bit scan, how it numbers
handlers) are conventions, not descriptions. The source-to-LowIR
assignments (PA10 through PA23) compare an output with the course solution's
instruction for instruction, so an output that does otherwise differs from
the reference; [What The Comparison Absorbs And What It
Enforces](#what-the-comparison-absorbs-and-what-it-enforces) collects them,
and each assignment's handout states the ones its fixtures add. Sentences
about the optimizer and the backend are informative: what those stages may
do, not what they must.

The following naming conventions are used:

- globals: `@name`
- functions: `@name`
- blocks: `^label`
- stack slots: `$name`
- temporaries: `%name`

These are LowIR symbol names, not necessarily raw source-level identifiers. Later C++
lowering assignments may encode qualified or overloaded source names into deterministic
LowIR symbol spellings as long as they remain valid `@name` tokens.

## Types

The base LowIR type family begins with the scalar and pointer types:

- `void`
- `i1`
- `i8`
- `u8`
- `i16`
- `u16`
- `i32`
- `u32`
- `i64`
- `i128`
- `f32`
- `f64`
- `f80`
- `ptr`

PA8 also accepts a restricted object-aware direct call-boundary type:

- `obj<bytesxalign>`

where `bytes` is the storage size in bytes and `align` is the required alignment in
bytes.

This form is part of the semantic call boundary. In the current PA8 subset it is valid in
direct parameter positions, function return positions, explicit indirect-call signatures,
and addressable object slot declarations when later lowering stages need concrete local
storage. It is still not a general-purpose structured global-data item type.

## Program Structure

A LowIR program is a sequence of top-level declarations and definitions.
Canonical LowIR reference output and canonical dumps use this top-level
presentation order:

1. `declare global`
2. `declare function`
3. `global`
4. `function`
5. `alias object`

For canonical text, no later phase is followed by an earlier phase. For example,
all global definitions appear before all function definitions, and all
declarations appear before definitions.

Within canonical text, each phase's output order is defined for the same input
and command line. Source-owned top-level entries are ordered by command-line
translation-unit order and then source/declaration order within each
translation unit. Generated top-level entries, including helper globals, helper
functions, init/fini functions, thunks, and runtime-support entries, follow the
generated-definition rules below. If no more specific rule below applies, emit
generated entries in first semantic-demand order using command-line
translation-unit order and source/declaration order as the traversal order.
For reference presentation, generated support entries that are collected from
symbol sets rather than one semantic traversal point may use internal LowIR
symbol-name order as a tie-breaker. That fallback is a presentation detail, not
a semantic dependency: source-to-LowIR assignment comparison may canonicalize
top-level entries before comparison, and implementations should not make host
ABI mangling, `object=...` metadata, or backend symbol spelling the owner of
semantic ordering facts.

For C++ source-to-LowIR reference output, the course solution writes
generated definitions in the following order. The comparison canonicalizes
top-level order (What The Comparison Absorbs below), so an output in another
order compares equal; the list describes the reference's presentation:

- Source-owned non-mergeable function definitions appear in command-line
  translation-unit order and then source/declaration order.
- Demand-emitted definitions, such as inline, template-instantiated, weak, or
  synthesized helper definitions, appear after the source-owned definitions that
  make them required. If one emitted helper requires another helper, the newly
  required helper appears later in the same `function` phase.
- For one class, generated copy forms precede generated move forms within the
  same special-member family: copy constructor before move constructor, and copy
  assignment before move assignment.
- Constructor ABI entry points for the same constructor appear as base entry
  before complete entry. Destructor ABI entry points for the same destructor
  appear as base entry, deleting entry, then complete entry.
- Vtable-related global definitions appear in the `global` phase. For one
  polymorphic class, the primary vtable appears before secondary/view vtables,
  secondary/view vtables follow complete-object subobject layout order, and
  any VTT appears after the vtables it references.
- Vtable slots follow virtual-slot order. Inherited primary-base slots precede
  slots newly introduced by the class, newly introduced slots follow source
  declaration order, and an override keeps the slot of the function it
  overrides. A virtual destructor occupies two adjacent vtable slots: complete
  destructor entry first, then deleting destructor entry. The base destructor
  entry is not a vtable slot. This vtable slot order is intentionally different
  from the function-definition order above.
- Generated construction helpers for base and member subobjects follow C++
  lifetime order. Generated destruction helpers follow the corresponding reverse
  lifetime order.
- Global initialization helpers appear before global finalization helpers.
  Initialization actions follow declaration order; finalization actions follow
  reverse declaration order.

Top-level definitions and declarations may refer to symbols that are emitted
later in the same LowIR program. The canonical top-level order is a stable
presentation convention, not a dependency sort. The source-to-LowIR
assignment harnesses canonicalize top-level entries before comparison, and
they still preserve and check order-sensitive LowIR regions: instruction order
inside blocks, item order inside structured globals, vtable slot order, and
action order inside generated initialization, finalization, constructor,
destructor, and cleanup bodies.

The declaration/definition split is explicit:

```text
declare global @name
declare global @name : <type>
declare function @name(%a : i64, %b : ptr) -> i64
declare function @printf(%fmt : ptr) -> i32 [arity=variadic]
```

Top-level declarations and definitions may also carry explicit symbol metadata:

```text
declare global @shared_state : ptr [binding=weak]
declare global @__external_rtti__int : ptr [binding=strong, object=_ZTIi]
declare global @ro_table : ptr [storage=readonly]
declare global @tls_state : i64 [storage=thread_local]
declare global @placed_state : i64 [section=.cppgm_data]
declare function @user_entry() -> i64 [role=entry]
declare function @puts(%fmt : ptr) -> i32 [arity=variadic, linkage=c]
global @lookup_table : ptr [storage=readonly, section=cppgm_ro] = zero
global @tls_counter : i64 [storage=thread_local] = 7
function @main() -> i64 [role=entry, binding=strong, keep_alias=yes] {
  ...
}
function @helper() -> i64 [binding=strong, object=_ZL6helperv, prefer_local=yes] {
  ...
}
function @small_required_helper(%x : i64) -> i64 [force_inline=yes] {
  ...
}
function @boot() -> void [role=init, binding=strong] {
  ...
}
```

The currently defined top-level metadata keys are `role`, `linkage`, `binding`, `object`,
`tls_for` (functions only), `keep_alias`, `prefer_local`, `object_root`, `force_inline`
(functions only), `inline_hint` (functions only), `no_inline` (functions only), and
`storage` and `section` (globals only).

The currently defined global `storage` values are:

- `readonly`
- `thread_local`

`storage=readonly` means that no code writes the object once it is statically
initialized; a program that writes it has undefined behavior. A back end may
place it in read-only memory (`.rodata`, or `.data.rel.ro` when it holds
addresses), and a consumer may fold a load from a global that is not
`replaceable` to the value of its initializer. The C++ lowering states it by the
rule in Facts The C++ Lowering States below.

`section=<name>` places a declared or defined global in a named ELF section.
The first public form is intentionally token-safe: `<name>` must be nonempty
and contain only ASCII letters, digits, `_`, and `.`. It does not accept a
quoted name, whitespace, commas, hyphens, target segment syntax, or function
placement. A declaration and definition of the same source object must agree
on the section name before LowIR is produced.

`storage=thread_local` is semantic TLS storage intent. Backend-specific descriptor
sections and relocation forms are lower-layer object-format details. When a thread-local
object needs a named ABI wrapper function surface, that surface appears as an ordinary
LowIR function declaration or definition with `tls_for=<global>` naming the thread-local
global it wraps, for example
`declare function @x__tls_wrapper() -> ptr [object=_ZTW..., tls_for=@x]`.

The currently defined role values are:

- function roles:
  - `entry`
  - `init`
  - `fini`
  - `eh_allocate_exception`
  - `eh_begin_catch`
  - `eh_end_catch`
  - `eh_rethrow`
  - `eh_throw`
  - `eh_personality`
  - `eh_resume`
  - `allocate_memory`
  - `free_memory`
  - `terminate`
  - `pure_virtual`
  - `dynamic_cast`
  - `bad_cast`
  - `bad_typeid`
  - `bad_array_new_length`
- global roles:
  - `rtti_class`
  - `rtti_si`
  - `rtti_vmi`
  - `rtti_pointer`
  - `rtti_data`

Only one symbol may own each singleton role inside a LowIR program. All roles
listed above are singleton roles except `rtti_data`, which may describe each
distinct runtime type-information object or category helper used by the
program. A role must also appear on the correct top-level kind: function roles
on functions and global roles on globals.

The `rtti_pointer` role identifies the vtable of the runtime's pointer
type-information class. A pointer exception is caught by value: a standalone
backend recognizes pointer types through it when it matches handlers
(qualification, base and `void*` conversions, 15.3p3), so that
`eh_begin_catch` yields the converted pointer rather than the address of the
exception object.

The `terminate` role identifies the C++ termination runtime used at a
non-throwing function boundary. A standalone backend may provide this runtime
directly, while an object backend may retain the declared object symbol for the
host C++ runtime.

The `bad_array_new_length` role identifies the runtime an array new-expression
calls, instead of its allocation function, when its count is invalid; it does
not return. It throws `std::bad_array_new_length`: an object backend retains
the host symbol (`__cxa_throw_bad_array_new_length`), and the standalone
backend's runtime throws the same exception, as it throws `std::bad_cast` and
`std::bad_typeid` for those roles.

The currently defined linkage values are:

- `c`

Omission means ordinary C++ language linkage and has no explicit spelling. `c` makes an
otherwise ordinary symbol explicitly C-linked in the printed LowIR. The current emit-LowIR
path uses `linkage=c` where the semantic frontend already knows the symbol has C linkage,
such as `extern "C"` imports.

The currently defined binding values are:

- `internal`
- `strong`
- `weak`

`binding` makes the intended top-level symbol binding explicit in printed LowIR. This is the
language/backend-neutral ownership boundary that later object emission can interpret as local,
ordinary external, or weak/coalescable.

The `replaceable=yes` metadata flag, valid only with `binding=weak`, marks a GNU weak
definition (`__attribute__((weak))` on a function or variable): a strong definition in
another object replaces it at link time. A `binding=weak` definition without it has vague
linkage (an inline function, a template instantiation), and every copy of it is equivalent.
Nothing may inline, specialize or discard a replaceable definition, or read a replaceable
variable's initializer as its value; object emission gives it a weak symbol. Omission means
false; there is no `replaceable=no` spelling.

The `object` metadata key carries an explicit backend/object symbol spelling for the top-level
symbol. This is the textual carrier for cases where later object emission must use a concrete
host/object name such as `puts`, `_ZTIi`, or a local mangled helper name.

The `keep_alias=yes` metadata flag requests that later object emission keep the LowIR-internal
symbol spelling available as an alias even when `object=...` spells a different
exported/backend name. Omission means false; there is no `keep_alias=no` spelling.

The `prefer_local=yes` metadata flag records that later object emission should prefer a
local/private binding when that is legal, even if an explicit `object=...` spelling is also
present. Omission means false; there is no `prefer_local=no` spelling.

The `object_root=yes` metadata flag records that the definition is a language-required
object-emission root even when no LowIR instruction refers to it, as for a member emitted by
an explicit template-instantiation definition. Omission means false; there is no
`object_root=no` spelling.

The `force_inline=yes` metadata flag on top-level function declarations and definitions
admits an eligible direct call of the function for inlining at `-O1` and above without
counting its size against the caller's budget. A unit-wide growth budget still applies, as do
the safety checks (recursion, `no_inline`, `replaceable`, exception regions). At `-O0` it has
no effect, and `lowiropt -O0` preserves it. The C++ lowering never states it. Omission means
false; there is no `force_inline=no` spelling.

The `inline_hint=yes` metadata flag on top-level function declarations and definitions
records that the function is an inline function (7.1.2), the source language's preference
for inlining its eligible direct calls. It may increase a bounded profitability limit, but
does not require substitution, change linkage, make the definition an object root, or
override `no_inline=yes`. Omission means false; there is no `inline_hint=no` spelling.

The `no_inline=yes` metadata flag on top-level function declarations and definitions prevents
optional inlining of calls to that function. It does not make the function an object-emission
root, change its symbol binding, or suppress other semantics-preserving transforms. Omission
means false; there is no `no_inline=no` spelling.

The C++ lowering states `inline_hint` and `no_inline` by the rules in Facts The C++ Lowering
States below.

`alias object <object-symbol> = @target` records an additional object-file symbol spelling
that must resolve to the same emitted top-level LowIR function or global as `@target`.
Multiple alias lines may target the same LowIR symbol, but each alias object symbol may be
defined only once:

```text
function @Box__Box(%this : ptr) -> void [binding=weak, object=_ZN3BoxC1Ev] {
  ...
}
alias object _ZN3BoxC2Ev = @Box__Box
```

Use an object alias only when the extra symbol is another name for the same emitted code or
data. If the extra ABI surface is separate generated code, such as a thread-local access
wrapper, represent it as a normal LowIR function declaration or definition with its own
`object=...` spelling.

Canonical `cppgm++ --emit-lowir` output is expected to carry all backend-relevant symbol/export
and runtime-support information needed by later object preparation. Later object-path steps such
as debug stripping or LowIR optimization may transform that canonical program, but they should
not depend on hidden semantic side data that is absent from textual LowIR. An in-process
`cppgm++ -c` implementation may skip writing a temporary `.lowir` file for speed, but it should
operate on facts that could have been obtained by serializing the LowIR program and parsing it
again.

Functions may also carry explicit call-boundary metadata after the return type:

```text
declare function @printf(%fmt : ptr) -> i32 [arity=variadic, effects=readonly]
function @sum(%count : i64) -> i64 [arity=variadic] {
  ...
}
declare function @trap() -> void [effects=readnone, unwind=no, return=noreturn]
```

The currently defined function metadata keys are:

- `arity`
- `effects`
- `unwind`
- `return`

The currently defined arity values are:

- `variadic`

Omission means fixed arity and has no explicit spelling. `variadic` means the function accepts
at least its declared parameter prefix plus any later variadic arguments. LowIR does not reserve
an additional arity mode for a hypothetical source boundary; a new mode requires a real
producer, consumer, and property test.

The currently defined call-effect values are:

- `readnone`
- `readonly`

Omission is the ordinary conservative read/write default and has no explicit spelling.
`readnone` means the boundary does not read or write memory. `readonly` means the boundary
may read memory but does not write it.

The currently defined unwind value is `no`. Omission means the call may unwind and has no
explicit spelling.

The currently defined return value is `noreturn`. Omission means the call returns normally
and has no explicit spelling.

The C++ lowering states `effects`, `unwind` and `return` by the rules in Facts The C++
Lowering States below.

Call signatures only accept call-boundary metadata such as `arity=...`, `effects=...`,
`unwind=...`, and `return=...`. Top-level symbol metadata such as `role=...`, `linkage=...`,
`binding=...`, `replaceable=...`, `object=...`, `keep_alias=...`, `prefer_local=...`, `object_root=...`, and
`force_inline=...`, `inline_hint=...`, and
`no_inline=...` are not valid on `as (...) -> ...`
call signatures.

Later backend lowering may also introduce internal compiler builtin helper symbols for
operations that are still first-class in LowIR but are not emitted directly on the
current target. Those helpers should stay in the `cppgm_builtin_*` namespace inside the
compiler pipeline; later object emission may remap them to the host runtime's concrete
symbol spellings.

Definitions keep the existing `global` and `function` forms.

Example:

```text
declare function @helper(%x : i64) -> void
declare global @ext_state : ptr

global @g : i64 = zero

function @user_entry() -> i64 [role=entry] {
  slot $x : i64

  block ^entry:
    %0 = const i64 0
    store i64 %0, $x
    %1 = load i64 $x
    return i64 %1
}
```

## Globals

Declaration-only globals use:

```text
declare global @name
declare global @name : <type>
```

The typed form should be used when the storage has a stable scalar/pointer-like
LowIR type. The untyped form is used for opaque imported storage whose concrete
layout is not part of the current IR unit.

The PA8 subset requires:

```text
global @name : <type> = zero
global @name : <type> = <scalar-literal>
global @name : ptr = addr @other
global @name : <type> [storage=readonly] = <scalar-literal>
global @name : <type> [storage=thread_local] = <scalar-literal>
```

Address initializers may name either another global or a function.

The PA8 milestone must also support the structured global-data form below. Later milestones
may use it more heavily without changing the surrounding global definition structure.

### Structured Global Data Initializers

PA8 must support a structured global-data form for read-only tables, string-like byte data,
aggregate constants, and vtable storage:

```text
global @name = {
  <data-item>
  <data-item>
  ...
}

global @name [storage=readonly] = {
  <data-item>
  <data-item>
  ...
}

global @name [storage=thread_local] = {
  <data-item>
  <data-item>
  ...
}
```

A structured global may also specify an object storage type before its metadata,
for example `global @aligned : obj<16x16> = { i32 7 }`. The type supplies
the minimum alignment and storage span. Data items retain their own layout;
remaining storage bytes are zero-filled. This permits an over-aligned scalar's
storage to differ from its source-language size and canonical scalar type.

The C++ lowering states the storage type of a structured global exactly when
the object's alignment is greater than the largest alignment among its data
items other than `zero`: an `alignas` or `__attribute__((aligned))` on the
variable, its type or its class, and an object whose items are all `zero`.
Without it the global would be aligned only as its items require.

Where each `<data-item>` is one of:

```text
<type> <scalar-literal>
ptr addr @symbol
zero <bytes>
```

Examples:

```text
global @vtable [storage=readonly] = {
  ptr addr @YD__f
  ptr addr @YD__g
}

global @msg = {
  i8 72
  i8 105
  i8 0
}

global @obj = {
  i64 1
  zero 8
  ptr addr @helper
}

global @coeffs = {
  f32 1.5f
  f64 2.25
}
```

A reader also accepts the bare `readonly` and `thread_local` keywords on global
declarations and definitions, as the `[storage=...]` forms; a writer prints the
bracketed form.

This form is intended to describe final data layout directly. It does not imply any
source-language construction semantics by itself. When translated through PA8, multi-byte
items use their LowIR data width and alignment, while `zero <bytes>` contributes raw
padding/data bytes exactly as written.

## Functions

Declaration-only functions use:

```text
declare function @name(%a : i64, %b : ptr) -> i64
```

A declaration names the imported/direct-call surface of the current IR unit
without providing a body.

A function has:

- a name
- a parameter list
- a return type
- zero or more stack slots
- one or more basic blocks

Function header syntax:

```text
function @name(%a : i64, %b : ptr) -> i64 {
  ...
}
```

Optional top-level function metadata appears after the return type:

```text
function @user_entry() -> i64 [role=entry] {
  ...
}
```

Multiple function metadata items may appear in one bracket or separate brackets:

```text
function @sum(%count : i64) -> i64 [arity=variadic] {
  ...
}

function @user_entry() -> i64 [role=entry] {
  ...
}
```

Function definitions may also carry an optional trailing debug-location suffix
after the function metadata:

```text
function @user_entry() -> i64 [role=entry] !dbg(main.cpp, 1, 1) {
  ...
}
```

Parameters may also carry explicit passing metadata after the parameter type:

```text
function @helper(%ret : ptr [pass=indirect_result, object_bytes=24],
                 %obj : ptr [pass=by_address, object_bytes=24],
                 %ref : ptr [pass=by_address, object_bytes=8],
                 %arr : ptr,
                 %buf : ptr [alias=noalias],
                 %x : i64) -> void {
  ...
}
```

The currently defined parameter metadata keys are `pass`, `alias`, and
`object_bytes`.

The currently defined pass values are:

- `indirect_result`
- `by_address`

Omission means ordinary direct-value passing and has no explicit spelling.

The currently defined alias values are:

- `noalias`

The intended meaning is semantic, not host-ABI-specific:

- `indirect_result`: this pointer names caller-owned result storage
- `by_address`: this parameter is an indirect object, value, or source-reference
  boundary whose argument denotes addressable storage
- `noalias`: this incoming pointer is disjoint from every other pointer
  parameter on the same boundary that also carries `alias=noalias`
- `object_bytes=N`: `N` bytes starting at the pointer are readable when the
  function is entered, so a consumer may load from them where the code would
  not (out of a loop that may not run) as long as nothing that may free them
  has run since the entry. It does not say that accesses through the pointer
  stay inside those bytes, and it is not a host ABI size. Omission promises
  nothing.

For the current LowIR subset, every explicitly spelled pass mode and alias
metadata require parameter type `ptr`. `object_bytes` likewise requires `ptr`
and a positive integer value.
`indirect_result` must appear on the first parameter and requires function return type `void`.

Stack slot syntax:

```text
slot $name : <type>
```

Slots may use ordinary scalar types or addressable object storage types such as
`obj<16x8>` when later lowering stages need concrete local object backing storage.

Blocks are written in order:

```text
block ^label:
  ...
```

Each block must end in exactly one terminator instruction.

### Debug Locations

LowIR function definitions and instructions may carry optional trailing
source-location metadata:

```text
function @main() -> i32 !dbg(main.cpp, 1, 1) {
  block ^entry:
%t = const i64 42 !dbg(main.cpp, 3, 7)
return i64 %t !dbg(main.cpp, 4, 3)
}
```

The `!dbg(...)` suffix is the current PA8-facing carrier for emitted
debug-info locations. On function definitions it records the function-level
source location. On instructions it records instruction-level locations.

The current fields are:

- source file
- 1-based source line
- 1-based source column

In the current textual format, the source-file field must be a single unquoted
LowIR token. Relative Unix-style source paths such as `main.cpp`,
`src/main.cpp`, or `tests/input.t` are valid. The parser rejects zero or
negative line/column values.

`cppgm++ --emit-lowir` emits these locations at every debug-information
level except `-g0` (`-g`, `-g1` to `-g3`, `-gline-tables-only`), and none
without a `-g` option. Each location is the line and column where a source
construct begins:

- A function definition is located where its definition begins.
- An instruction is located at the innermost statement or expression whose
  lowering emits it. In `int y = x + 1;` the addition is located at `x`,
  where `x + 1` begins, and the store to `y` at `int`, where the declaration
  begins. A condition's comparison and branch, and the jumps that close the
  arms, belong to the statement that tests it (`if`, `while`, `for`); the
  destructor calls at the end of a block belong to the block's `{`.
- Instructions outside every statement and expression take the function's
  location: the stores of incoming parameters, and the terminator that ends
  a function falling off its end (`return void`, or `unreachable` in a
  function with a result).
- The optimizer keeps them at `-O1` to `-O3`: an instruction that survives,
  is moved or is copied (an inlined body, an unrolled loop) keeps its
  location, and one that replaces another takes that one's location. A
  `phi` the optimizer creates carries none.

A source file whose name is not a single LowIR token gets no locations.

This metadata is semantically optional. Backends that do not yet emit
object-file debug information may ignore it, but PA8 parsing, dumping, and
`lowiropt` transport are expected to preserve it when present.

## Structural Well-Formedness

A LowIR unit is structurally well formed only if all of the following hold:

- every top-level symbol name appears at most once across declarations and definitions
- every object alias spelling appears at most once
- every object alias target names a top-level declaration or definition
- every `tls_for` target names a top-level thread-local global, and at most one
  wrapper names each thread-local global
- each function parameter name is unique within that function header
- each function slot name is unique within that function
- each function block label is unique within that function
- every function contains at least one block
- every block ends in exactly one terminator instruction
- no instruction appears after a terminator within the same block
- every referenced block target exists in the same function

The PA8 parser and validator are required to reject structurally malformed LowIR. Examples
of malformed programs include:

- a second `block ^entry:` inside the same function
- a second `slot $x : i64` inside the same function
- a `return` followed by another instruction in the same block
- a `jump ^missing` where `^missing` is never defined
- both `declare function @main(...)` and `function @main(...)` in the same LowIR unit

## Object ABI Conventions

LowIR stays semantic at the call boundary. Later object-aware milestones use explicit
call-boundary metadata plus restricted direct object boundary types where necessary,
rather than pretending that host register chunking is the stable IR contract.

The intended convention is:

- complete object values passed directly may use a semantic direct parameter type such as
  `obj<4x4>` or `obj<16x8>`
- complete object values passed indirectly use `ptr [pass=by_address]`
- complete object return-by-value may use a semantic direct return type such as
  `-> obj<8x4>` when the frontend wants that boundary explicitly represented in LowIR
- complete object return-by-value may also use an explicit leading destination pointer
  `ptr [pass=indirect_result]` when the source boundary is itself indirect
- source-level references use the shared `ptr [pass=by_address]` boundary;
  LowIR retains the required address semantics, not the source-level origin
- array/function decay is represented by an ordinary `ptr`; LowIR does not
  retain the source-level origin of an already-lowered pointer value
- variadic functions use explicit `arity=variadic` metadata on the function declaration or
  definition
- complete object locals may be represented by addressable storage rather than scalar
  temporaries, for example `slot $tmp : obj<16x8>`

So later milestones may use function headers such as:

```text
function @make_small_pair(%a : i64, %b : i64) -> obj<8x4> {
  ...
}

function @make_pair(%ret : ptr [pass=indirect_result, object_bytes=16], %a : i64, %b : i64) -> void {
  ...
}

function @consume_pair(%p : ptr [pass=by_address, object_bytes=16]) -> void {
  ...
}

function @sum_pair(%p : obj<8x4>) -> i64 {
  ...
}

function @consume_ref(%p : ptr [pass=by_address, object_bytes=16]) -> void {
  ...
}
```

These annotations describe the semantic call boundary, not the final host register
assignment. Host-specific coercions such as pair-in-register splitting remain backend work,
not stable LowIR syntax.

One host fact cannot be derived from `obj<NxA>` alone: which eightbytes of a direct object
the x86-64 psABI (3.2.3) classifies SSE rather than INTEGER, which depends on the member
types. A direct object parameter states it as `classes=...` parameter metadata, and a direct
object result as `result_classes=...` function or call-signature metadata:

```text
function @scale(%p : obj<16x8> [classes=sse_integer], %k : f64) -> obj<16x8> [result_classes=sse_integer] {
  ...
}
%r = call obj<16x8> %fp($pair, 2.0) as (%a0 : obj<16x8> [classes=sse_integer], %a1 : f64) -> obj<16x8> [result_classes=sse_integer]
```

The value names one class for each eightbyte, the first eightbyte first, joined by `_`:
`integer`, `sse`, or `none` for an eightbyte that holds no member (padding, an empty class).
Classes describe only a direct `obj<NxA>` parameter or result of at most 16 bytes, and name
exactly its eightbytes. Without them every eightbyte is INTEGER, so the C++ lowering states
them exactly when some eightbyte is SSE or `none`, or the object is `memory`; a larger direct
object is always passed in memory. Arguments past a variadic function's fixed parameters
state no classes.

`memory`, valid only on a parameter, instead says the object is passed in memory: an object
with a misaligned member, or with an x87 (`long double`) or 16-byte vector member. A result
with a misaligned member is returned through a `pass=indirect_result` parameter instead of a
direct result. The psABI returns an x87 or 16-byte vector member in `st0` or in a whole XMM
register, which these classes do not describe, so such a result states no classes.

The native backend places an INTEGER eightbyte in the next argument GPR (rax then rdx for a
result), an SSE eightbyte in the next XMM register (xmm0 then xmm1 for a result), and nothing
for a `none` eightbyte. An object whose eightbytes do not all fit in the remaining registers,
or whose classes are `memory`, goes on the stack whole. The placement is the same in course
objects, link mode and `lowir2native` as in host objects, and objects passed this way
interoperate with GCC- and Clang-compiled code.

PA8's reader/writer preserves the object-boundary forms exercised by its
fixtures. Later C++ lowering creates these boundaries, and the native
backend implements their calling convention.

## Reserved Runtime Hooks

Later lowering milestones may introduce synthetic helper functions and globals that
participate in program startup, shutdown, or exception/runtime support.

The preferred convention is role-driven:

- one `function` definition with `[role=entry]`
- zero or one `function` definition with `[role=init]`
- zero or one `function` definition with `[role=fini]`

If present as definitions, the native backend runs the
`init` hook before the `entry` function and the `fini` hook after it.

Later hosted/EH lowering may also introduce reserved function runtime-support roles:

- functions:
  - `eh_allocate_exception`
  - `eh_begin_catch`
  - `eh_end_catch`
  - `eh_rethrow`
  - `eh_throw`
  - `eh_personality`
  - `eh_resume`

LowIR programs express exception flow through the EH operations below.
The native/runtime assignments implement unwinding and exception storage;
PA8 does not implement an exception runtime.

When no role is stated, the spellings `@main`, `@__cppgm_init` and `@__cppgm_fini`
imply the `entry`, `init` and `fini` roles. New LowIR states the role.

## Operands

The PA8 subset uses:

- temporaries: `%0`, `%tmp`
- stack slots: `$x`
- globals/functions: `@g`, `@main`
- block labels for terminators: `^entry`
- scalar literals (`integer` or `floating`)

## Required PA8 Instructions

### Constants and Copies

```text
%t = const <type> <literal>
%t = copy <type> <value>
```

`<literal>` may be either an integer literal or a floating literal. Floating literals are
valid with `f32`, `f64`, and `f80` types. `f80` literals use the usual `L` suffix, for
example `1.25L`.

Three values have no digit spelling and are written as words instead. `nullptr` is the
null pointer constant and is valid with `ptr`. `inf` and `nan` are the floating infinity
and quiet NaN, and `snan` is the signalling NaN; each may also be written
in uppercase as `INFINITY`, `NAN`, or `SNAN`, and `inf` may carry a leading `-`.

`f80` storage is 16 bytes at 16-byte alignment wherever LowIR lays it out: a
slot, a structured global's item, the stride of `index f80`. The first 10 bytes
carry the x87 extended value and the trailing 6 bytes are zero padding. The
vocabulary header states the same width and alignment, so every layout the
backends compute agrees with the one the text describes.

### Control-Flow Value Merges

```text
%t = phi <type> [^predecessor: <value>, ...]
```

`phi` selects the value paired with the ordinary control-flow edge by which
execution entered the current block. A `phi` must name every distinct ordinary
predecessor of its block exactly once, and may not name any other block. All
`phi` instructions in a block precede its ordinary instructions and execute in
parallel, so one `phi` may use the value produced by another `phi` on a loop's
previous iteration.

The result and every incoming value have the same directly representable
scalar type: `i1`, `i8`, `u8`, `i16`, `u16`, `i32`, `u32`, `i64`, `f32`,
`f64`, or `ptr`. `i128` and `f80` are not `phi` types: a value of either
type that must survive a merge stays in a slot. A block reached as an
exception handler target may not contain `phi`; exception state remains
represented by the explicit exception/runtime instructions.

A conditional scalar choice is represented with ordinary control flow: branch
to the blocks that produce the alternative values, jump from those blocks to
a merge block, and take the chosen value there. Convention of the C++
lowering (enforced): every conditional value the source writes (`?:`, and
`&&` or `||` used as a value) takes this form and carries the value through
memory, each arm storing its value into one slot and the merge block loading
it; the lowering emits no `phi`. `phi` is the optimizer's form, which slot
promotion (PA32) makes from such a slot. An optimizer may also turn a choice
whose arms only compute values into a `select` (Unary and Binary Operations
below).

### Memory and Addressing

```text
%t = addr @global
%t = addr @function
%t = addr $slot
%t = load <type> <storage>
%t = load volatile <type> <storage>
%t = load [volatile] typed <type> <storage>
store <type> <value>, <storage>
store volatile <type> <value>, <storage>
store [volatile] typed <type> <value>, <storage>
%t = atomic_load <type> <ptr-value>, <order>
atomic_store <type> <value>, <ptr-value>, <order>
%t = atomic_exchange <type> <ptr-value>, <value>, <order>
%t = atomic_compare_exchange <type> <ptr-value>, <expected-ptr>, <desired>, <success-order>, <failure-order>
%t = index <type> [projection=<kind>] <base>, <offset>
```

In the PA8 subset, `index` computes a pointer by advancing a base pointer by `offset`
elements of `<type>`.

`[projection=<kind>]` is optional metadata that labels the source of an address. The
currently supported projection kinds are:

- `array_element`
- `field`

`array_element` labels the address of an array element (`p[i]`), and `field` the
address of a member (`s.x`, `p->x`, `obj.*pm`). When omitted, `index` is plain
pointer arithmetic; already-lowered source origins such as base-subobject selection
have no spelling. No pass of the course solution draws a conclusion from the label, so a
consumer may not depend on it, a producer may omit it, and the comparison ignores it.

`typed` is an instruction fact (see Instruction Facts below).

`load volatile` and `store volatile` mark an observable access to a
volatile-qualified object. Volatility is a property of the operation, not of
the pointer type: no pass may remove, merge, reorder, or forward a volatile
access, and storage reached by a volatile access may not be promoted or
discarded. Ordinary `load`/`store` operations to the same storage keep their
usual optimization freedoms. Bulk object operations (`copyobj`, `zeroinit`)
have no volatile form; objects with volatile subobjects must be transferred
through scalar volatile accesses.

Explicit scalar initialization, including aggregate members and elements,
retains the destination's volatile access marker. Preliminary zero-initialization
of class storage before a constructor is a storage initialization operation;
it does not make every byte write a volatile scalar access. Subsequent
constructor accesses and copies retain their ordinary volatile requirements.

Atomic memory-order operands use the GNU/Clang order encoding:

- `0` relaxed
- `1` consume
- `2` acquire
- `3` release
- `4` acq_rel
- `5` seq_cst

### Bulk Object Memory Operations

PA8 must support explicit bulk-memory instructions instead of requiring every object copy or
zero-initialization path to be spelled as a hand-written sequence of scalar `load`/`store`
operations:

```text
copyobj <bytes>x<align> <src>, <dst>
zeroinit <bytes>x<align> <dst>
```

Where:

- `<bytes>x<align>` is a positive byte-count/alignment span
- `<align>` must be a positive power-of-two byte alignment
- `<src>` is either:
  - a pointer-valued operand, or
  - a direct object-typed value `obj<bytesxalign>` whose byte size and alignment
    match `<bytes>x<align>`
- `<dst>` is a pointer-valued operand

`copyobj` copies exactly `<bytes>` bytes from `<src>` to `<dst>`. When `<src>` is an
object-typed value, the operation copies from that semantic object value rather than
requiring the IR to spell the host ABI chunking explicitly.

`copyobj` is a non-overlapping semantic object copy: the source and destination
spans must not overlap (byte-identical source and destination addresses are the
one permitted degenerate case, and lowering may elide such a copy). Producers
lower semantic object transfers, which never alias their storage, and native
lowering is free to use forward byte copies or chunked register moves that would
be wrong for overlapping spans. An overlap-capable move remains a hosted/runtime
operation (`memmove`); if a later optimizer needs a first-class overlap-safe IR
operation, it must be added as a distinct instruction rather than by weakening
this contract.

`zeroinit` writes zero to exactly `<bytes>` bytes starting at `<dst>`.

These instructions are intended for lowered object storage, trivial copies, and simple static
zero-initialization paths. They do not imply source-language copy semantics by themselves;
later milestones still decide when a copy constructor, destructor, or other semantic action
is required.

### Unary and Binary Operations

```text
%t = unary <op> <type> <value>
%t = binary <op> [nowrap] [exact] <type> <lhs>, <rhs>
%t = cmp <pred> <type> <lhs>, <rhs>
%t = select <type> <cond>, <a>, <b>
%t = convert <op> <dst-type> <src-type> <value>
%t = atomic_add_fetch <type> <ptr-value>, <delta>, <order>
atomic_thread_fence <order>
atomic_signal_fence <order>
```

Required PA8 unary operators:

- `neg`
- `not`
- `bitnot`
- `bswap` for `i16`, `i32`, and `i64`
- `ctz` and `clz` for `i32`, `u32`, and `i64`

Required PA8 binary operators:

- `add`
- `sub`
- `mul`
- `div`
- `mod`
- `udiv`
- `umod`
- `and`
- `or`
- `xor`
- `shl`
- `shr`
- `ushr`

For floating types, the required binary subset is:

- `add`
- `sub`
- `mul`
- `div`

Required PA8 comparison predicates:

- `eq`
- `ne`
- `lt`
- `le`
- `gt`
- `ge`
- `ult`
- `ule`
- `ugt`
- `uge`

For `cmp`, `<type>` is the comparison operand type, not the result type. The
destination temporary is a canonical integer truth value and is materialized as
`i64` `0` or `1`. This is true for floating comparisons such as
`%ok = cmp eq f80 %a, %b`, pointer comparisons, and narrow integer comparisons:
the result's type is `i64` whatever the operand type.

`nowrap` and `exact` are instruction facts (see Instruction Facts below).

`select` is `<a>` when the integer `<cond>` is nonzero and `<b>` otherwise.
`<type>` is the type of `<a>`, `<b>` and the result: an integer type no wider
than 64 bits, or `ptr`. Both values are computed before the `select`, so it
has no effect beyond its result. Convention of the C++ lowering (enforced):
it produces `select` only to pick the fallback of GCC's `__builtin_clzg` and
`__builtin_ctzg` for a zero operand; every other choice the source writes is
branched (Control-Flow Value Merges above). An optimizer may form it from a
branch to arms that only compute values (if-conversion), and a backend may
lower it to a conditional move (`cmov` on x86-64) or to branches.

For integer operations where signedness changes the result, the plain forms remain the signed
operations:

- `div`, `mod`, `shr`, `lt`, `le`, `gt`, `ge`

and the `u...` forms are the corresponding unsigned operations:

- `udiv`, `umod`, `ushr`, `ult`, `ule`, `ugt`, `uge`

`atomic_add_fetch` atomically adds `<delta>` into the pointed storage and returns the
updated value.

`atomic_exchange` atomically replaces the pointed storage with `<value>` and returns the
previous value.

`atomic_compare_exchange` atomically compares the pointed storage against `*<expected-ptr>`.
If they compare equal, it stores `<desired>` and returns `1`. Otherwise it writes the current
memory value back through `<expected-ptr>` and returns `0`.

`unary bswap` performs byte reversal within the operand width. For example:

- `i16 0x1234` becomes `0x3412`
- `i32 0x11223344` becomes `0x44332211`
- `i64 0x0102030405060708` becomes `0x0807060504030201`

For native lowering, an implementation strategy is:

- use a direct machine/runtime byte-swap instruction where the execution substrate has one
- otherwise lower `i16` through a small shift/or sequence
- keep the operation as a first-class LowIR unary op rather than expanding it during C++
  lowering, so later native/object backends can choose the best lowering directly

`unary ctz` and `unary clz` count the zero bits below the lowest set bit and
above the highest set bit, at the operand's width and in the operand's type
(`ctz i32 8` is `3`, `clz i64 1` is `63`). For a zero operand the result is
an unspecified value of the type, as GCC's `__builtin_ctz` and
`__builtin_clz` are undefined there; the instruction has no other effect.
Convention of the C++ lowering (enforced): the GCC bit-scan builtins
(`__builtin_ctz`, `__builtin_clzll`, `__builtin_ctzg` and the rest) lower to
them, an operand narrower than 32 bits widened to `u32` first and, for
`clz`, the bits the widening added subtracted; `__builtin_clzg` and
`__builtin_ctzg` then `select` their second operand for a zero operand.
x86-64 lowers them to `bsf`, and to `bsr` followed by an `xor` with the
width less one.

Array-to-pointer and function-to-pointer decay use the ordinary pointer value
produced by `addr`, `index`, a parameter, or `copy ptr`. There is no decay-specific
unary operation: once the source construct has become a `ptr`, its source-level
origin does not change LowIR behavior.

PA8's model and text checks preserve the atomic operations and their
operands. Machine-level atomic behavior belongs to native lowering.

This compare-exchange primitive is also the intended stable LowIR building block for more
complex atomic read-modify-write operations. Later lowering stages may implement operations
such as atomic fetch-and, fetch-or, and fetch-xor by expanding them into compare-exchange
loops without changing the LowIR surface again.

Later native/object milestones must preserve the same LowIR source contract while providing
real machine-level atomic behavior.

For floating types, the same predicate spellings apply over `f32` / `f64` / `f80`
operands.

Required PA8 scalar conversion operators:

- `sext`
- `zext`
- `trunc`
- `sitofp`
- `uitofp`
- `fptosi`
- `fptoui`
- `fpext`
- `fptrunc`

These conversion operators make the signedness and direction explicit in the IR:

- `sext` / `zext`
  widen an integer value from a narrower explicit source width into a wider
  destination width
- `trunc`
  narrows an integer value to a smaller destination width
- `sitofp` / `uitofp`
  convert integer values to floating values
- `fptosi` / `fptoui`
  convert floating values to integer values
- `fpext`
  widens a floating value to a larger floating type
- `fptrunc`
  narrows a floating value to a smaller floating type

Examples:

```text
%w = convert zext i64 u8 %byte
%x = convert sext i64 i32 %word
%y = convert trunc i16 i64 %wide
%a = convert sitofp f64 i32 %x
%b = convert uitofp f80 i64 %u
%c = convert fptosi i32 f64 %f
%d = convert fpext f80 f32 %z
%e = convert fptrunc f32 f80 %ld
```

The plain integer types in LowIR do not encode signedness by themselves, so the signedness
for integer/float conversions lives in the operator spelling rather than the type text.

### Stack allocation

```text
%p = stack_alloc <size>
```

`stack_alloc` allocates `<size>` bytes in the current function's stack frame and
returns a `ptr` to the allocated storage. Native backends align the allocation to
the target stack requirement and restore the stack through the normal function
epilogue. This is the stable LowIR lowering for source builtins such as
`__builtin_alloca`; it is not a callable external symbol.

### Variadic Argument Access

```text
va_start <list>
%t = va_arg <type> <list>
```

`va_start` prepares the argument list object `<list>` in a function declared
`[arity=variadic]`, and `va_arg` reads the next argument from it as `<type>` and advances
it. These are the stable LowIR lowerings for `__builtin_va_start` and
`__builtin_va_arg`; neither is a callable external symbol. PA8 only has to carry both
forms through the text form, since the argument list layout is part of the calling
convention that PA24 fixes.

### Calls

```text
%t = call <type> @function(<arg-list>)
%t = call <type> <callee-value>(<arg-list>) as (<param-list>) -> <type>
call void @function(<arg-list>)
call void @function(<destination>, <source>, ...) [elision=copy]
call void <callee-value>(<arg-list>) as (<param-list>) -> void
```

The PA8 subset requires direct calls and calls through pointer-valued operands.
For indirect calls, the explicit `as (...) -> ...` call signature is part of the
stable LowIR surface because the callee operand alone does not describe the
semantic boundary. That call signature may also carry call-boundary metadata
such as `arity=variadic`, `effects=readonly`, `unwind=no` or `return=noreturn`
when needed.

`[elision=copy]` is a source-language permission on a direct `void` call with
at least destination and source arguments.  It says that an optimizing stage
may omit this copy/move construction and redirect the complete construction
of its private source temporary into the destination when the serialized
control-flow and lifetime uses prove that rewrite safe.  It is not a command
to erase the call: an `-O0` consumer executes the call and all ordinary source
cleanup exactly as written.  The permission is invalid on value-returning or
indirect calls and does not classify an unmarked call by its symbol spelling.
The C++ lowering states it by the rule in Facts The C++ Lowering States below;
consuming it is optional, and the course solution's optimizer does not.

### Terminators

```text
jump ^target
branch %cond, ^then, ^else
switch %selector, ^default, <case-value>:^case, ...
return void
return <type> <value>
unreachable
```

`switch` is the first-class multi-way terminator form. It transfers control to
the first matching `<case-value>:^case` arm, or to `^default` if no arm matches.

`unreachable` states that execution cannot validly continue from that point.
It takes no operands and emits no call or symbol declaration. A release backend
may emit no bytes for this impossible continuation.

## Exception Instructions

These instructions carry exception handling through LowIR. The C++ lowering
emits the handler-stack forms for cleanups from PA11 on, and the catch
clauses and the selector when PA21 adds `try`, `catch` and `throw`; the
native backend runs them on the course's handler-stack runtime, and PA26's
host objects turn them into unwind tables. They are part of the long-term
LowIR contract, and PA8's reader, validator and writer already carry the
handler-stack forms.

### Handler Stack Management

```text
eh_try ^handler
eh_cleanup ^cleanup
eh_end
```

These instructions manage the dynamic exception-handler stack.

- `eh_try ^handler`
  pushes a catch-style handler that transfers control to block `^handler` if an exception is
  thrown
- `eh_cleanup ^cleanup`
  pushes a cleanup-style handler that transfers control to block `^cleanup`
- `eh_end`
  pops the most recently installed handler on the normal control-flow path

Handlers are lexically explicit in LowIR. The backend/runtime ABI decides how they are
represented at run time.

Entering an `eh_try` handler removes its record. A `^cleanup` handler's
record stays on the stack while the handler runs, but an exception thrown
inside the handler passes it by; the handler's `eh_end` removes it, as does
`resume`.

### Throwing and Resuming

```text
throw <type> <value>
resume
```

- `throw`
  raises a new exception carrying the lowered runtime value
- `resume`
  continues unwinding the currently active exception after a cleanup block has run

`throw` is the low-level form: its payload is a scalar or pointer value
carried through the runtime's state. The C++ lowering does not use it; it
throws by calling the functions with the `eh_allocate_exception` and
`eh_throw` roles (Reserved Runtime Hooks).

### Accessing the Current Exception

```text
%t = exception <type>
```

This reads the current exception payload into a temporary of the given lowered type.

`<type>` is a non-`void` scalar or `ptr` type. The C++ lowering reads the
exception as `ptr`, the value it passes to the `eh_begin_catch` function.

### Catch Clauses and the Selector

```text
eh_catch @type_info, <selector>
eh_catch_all, <selector>
eh_filter @type_info, ..., <selector>
eh_cleanup
%s = exception_selector i32
```

A handler block may begin with clauses that say which exceptions it
handles. On entry the handler's selector is 0; the clauses then run in
order, and each one that matches sets the selector to its `<selector>` if no
earlier clause did.

- `eh_catch @type_info, <selector>` matches an exception that a handler for
  the type that `@type_info` describes would catch (15.3): the same type, a
  public unambiguous base class, or for pointers a qualification, base or
  `void*` conversion. `@type_info` is that type's type-information global;
  a reference handler names the referenced type without cv-qualifiers.
- `eh_catch_all, <selector>` matches every exception.
- `eh_filter @type_info, ..., <selector>` matches an exception that matches
  none of the listed types: the violation of a dynamic exception
  specification (with no types, `throw()`, every exception violates it).
  Only host objects use it.
- `eh_cleanup` without a target sets no selector. It says that the handler
  has cleanup work to do, and it keeps an `eh_try` handler's record on the
  stack until an `eh_end` in the handler removes it.

`exception_selector` reads the selector: the `<selector>` of the first clause
that matched, or 0 if none did. A handler is entered for every exception that
reaches its record, whatever its clauses; its code tests the selector to
choose a catch body, or runs its cleanups and `resume`s when the selector is
0.

Convention of the C++ lowering (enforced): a function's handlers are
numbered from 1, each keeping its number at every landing that lists it,
and a specification's filter is -1. A landing inside a `try` block also lists the clauses of every enclosing
`try` statement the exception can reach before it leaves the function, up to
the first `catch (...)`, so the selector names the handler however deeply the
throwing region is nested. A clause after the handler has removed its own
record acts as the enclosing handler's clause would on entry: it sets the
selector only if no clause has.

## Instruction Facts

Three keywords state facts about an instruction:

```text
%v = load typed ptr %field
store typed i32 %n, %count
%t = binary add nowrap i32 %i, 1
%q = binary div exact i64 %bytes, 12
```

- `typed`, on a `load` or `store` of a pointer, a floating type or an integer
  wider than one byte: the access reads or writes an object of the access's
  type through a glvalue of that type, in the sense of C++ [basic.lval]/10,
  not through a union member, a bit-field's storage unit or a byte copy.
  Two typed accesses whose types are incompatible therefore never access the
  same bytes of one object: a consumer may assume that a typed store does not
  change what a typed load of an incompatible type reads. Two pointers are
  compatible, as are two values of one floating type and two integers of one
  width (a signed and an unsigned integer of one width are compatible).
- `nowrap`, on an integer `add`, `sub`, `mul` or `shl`: the exact result
  (for `shl`, the operand times two to the count) is representable in the
  operation's type at the type's signedness. `i8`, `i16`, `i32`, `i64` and
  `i128` are signed; `u8`, `u16` and `u32` are unsigned.
- `exact`, on an integer `div` or `udiv`: the dividend is a multiple of the
  divisor; on `shr` or `ushr`: no set bit is shifted out.

A fact holds every time its instruction runs. A program in which one does
not hold has undefined behavior, which is what lets a consumer rely on it.
The validator rejects a fact on any other instruction or type. The keywords
follow `volatile` on loads and stores and the operation on `binary`, in the
order shown.

The C++ lowering (PA10 onward) states facts by these rules, and the
comparison checks them as part of the instruction:

- `typed` on every `load` and `store` whose type is `ptr`, a floating type or
  an integer wider than one byte, unless the access is volatile, reads or
  writes a bit-field's storage unit, or goes through an address computed from
  a union member: a member of a union (an anonymous union's included) and
  every `index` or `copy` of such an address. A pointer to a union member
  that is stored and loaded again is an ordinary pointer, and accesses
  through it are typed: GCC and clang allow punning through a union, not
  through pointers into one.
- `nowrap` on the `add`, `sub` and `mul` that compute the source's `+`, `-`
  and `*`, the compound assignments `+=`, `-=` and `*=`, and the steps of
  `++` and `--`, when the operation's type after the usual arithmetic
  conversions is a signed `int`, `long` or `long long`: their overflow is
  undefined (C++ 5p4). An unsigned or narrower operation (an `i16` step), a
  shift (GCC and clang define a signed left shift that overflows), pointer
  arithmetic, and the address, size and count computations the lowering adds
  for its own purposes carry none.
- `exact` on the division of a pointer difference by the element size, the
  `shr` for a power of two and the `div` otherwise: the difference of two
  pointers into one array is a multiple of the element size (5.7p6).

Every reader accepts the facts and every writer prints those the program
holds, so a program keeps them through serialization: an object compiled
from emitted LowIR text is the object compiled directly (PA32's round trip).
A consumer may ignore any fact, and an optimizer may drop one, since removing
a fact never changes a program's meaning; an optimizer may also add a fact it
proves. An optimizer that moves an instruction to where it may run when it
did not before (out of a conditional block, out of a loop that may not run,
into a branch's predecessor) drops its facts, and so does one that rewrites
an instruction into a different computation. Copies that run under the same
conditions as the original (inlining, unrolling, duplication along a path)
keep them, and of two equal instructions merged into one, the survivor keeps
its facts only if it ran wherever the other did.

## Facts The C++ Lowering States

The C++ lowering (PA10 onward) states the boundary facts below by these
rules, and the source-to-LowIR comparison checks them as it checks the
instruction facts. The lowering states only what the source states: a fact
proven from a function body or from knowledge of a library belongs to the
optimizer, which may add it to optimized output, but the lowering never
states it.

`unwind=no`: a function declaration or definition states it exactly when the
function's exception-specification allows no exception (15.4p12).

- `noexcept`, `noexcept(true)` and `throw()`.
- `noexcept(e)` whose constant expression is true, evaluated for each
  specialization of a template: with
  `template<class X> int f(X) noexcept(sizeof(X) == 4)`, `f<int>` states it
  and `f<char>` does not.
- A destructor without an exception-specification, unless a destructor of a
  base or member allows exceptions (12.4p3, 15.4p14).
- An implicitly declared special member function, or one defaulted on its
  first declaration, when every function it directly invokes is non-throwing
  (15.4p14); an inheriting constructor likewise (12.9).
- A deallocation function without an exception-specification (15.4p15).
- A lambda's call operator by its own `noexcept` or `throw()`, and the static
  function its conversion to a function pointer returns by the call
  operator's.
- Every ABI entry of a constructor or destructor (base, complete, deleting)
  by the function's own specification, and a thunk by its target's.
- The GNU builtins (`__builtin_memcpy`, `__builtin_strlen`, ...) and the
  functions the lowering declares for itself: `std::terminate`
  (`role=terminate`), `__cxa_pure_virtual`, `abort` for `__builtin_abort`,
  `cppgm_builtin_trap`, `cppgm_builtin_memset`, the memory builtins declared
  with the runtime identities `object=cppgm_builtin_memcpy` and
  `object=cppgm_builtin_memmove` (three parameters, the two pointers of
  memcpy `alias=noalias`), the internal `cppgm_call_terminate`,
  local-static destructor helpers, the `__aggregate` helpers, and the
  soft-float and complex helpers.
- A C-linkage function first declared non-throwing in a system header and
  redeclared without an exception-specification (glibc's `__THROW`).

A call signature (`as (...) -> T [...]`) states `unwind=no` exactly when the
callee's declared type is non-throwing.

- A virtual call by the called function's specification; the virtual call of
  a deleting destructor that `delete p` makes by the destructor's.
- A call through a pointer, reference or pointer to member whose declaration
  (a variable, parameter or data member) is non-throwing by the rules above,
  such as `void (*fp)() noexcept`; `noexcept(fp())` agrees (5.3.7p3).
- A lambda-expression called directly, by its call operator's.

`noexcept(false)`, a dynamic-exception-specification that names a type, and
a function or pointer declared without a specification state none, and so
does a call through the result of a call or of a conditional expression. An
indirect call keeps its unwind path whatever its signature states.

`return=noreturn`: a function declaration, definition or call signature
states it when any declaration of the function carries `[[noreturn]]` or GNU
`noreturn`, or the function is one of the runtime functions listed here.

- A user function, member functions included, declared `[[noreturn]]` or
  `__attribute__((noreturn))` on its first declaration, its definition or a
  later declaration.
- A function template's specializations by the template's attribute; an
  explicit specialization of a function template by its own declarations
  only, and an explicit specialization of a member of a class template by
  the member's.
- A virtual call of a `noreturn` virtual function.
- `__cxa_throw`, `__cxa_rethrow`, `__cxa_call_unexpected`, `__cxa_bad_cast`,
  `__cxa_bad_typeid`, `__cxa_throw_bad_array_new_length`,
  `__cxa_pure_virtual`, `std::terminate`, `abort` for `__builtin_abort`,
  `cppgm_builtin_trap` and the internal `cppgm_call_terminate`.

A call through a pointer or reference states none: the attribute appertains
to a function, not to its type (7.6.3p1). A call of a `noreturn` function
does not end its block; what follows it is lowered as written.

`effects`: a function declaration, definition or call signature states
`effects=readnone` when any declaration of the function carries GNU `const`,
and `effects=readonly` when one carries GNU `pure` and none carries `const`;
`__builtin_strlen` states `effects=readonly`.

- `__attribute__((pure))`, `__attribute__((const))`, `[[gnu::pure]]` and
  `[[gnu::const]]` on any declaration: a later declaration adds to an earlier
  one, and a definition without the attribute keeps it.
- A function template's specializations by the template's; an explicit
  specialization by its own declarations.
- The attribute on a lambda-declarator, for the call operator.
- A virtual call of such a function.

What a library function does (`sqrt`, `strlen` declared by `<string.h>`) is
knowledge for the optimizer; the lowering states nothing for it.

`storage=readonly`: a global definition states it when the object is
statically initialized, not thread-local, and of a const, non-volatile type
whose class, at any depth (an element's, a base's, a member's), has no
`mutable` or volatile member and no non-trivial destructor; the lowering's
own constant data states it too.

- A const scalar, pointer or member pointer, and a const complex or vector
  value.
- A const array, whose qualifiers are its element type's (3.9.3p5).
- A const object of a trivially destructible class, statically initialized by
  constant initialization, by a constexpr constructor or by the lowering's
  static construction of a polymorphic object.
- Namespace-scope variables, static data members (of class templates too) and
  block-scope statics, GNU weak definitions included.
- The temporary that a constant-initialized reference of static storage
  duration is bound to, by the temporary's own type: `const int& r = 3;`
  makes a readonly `const int`; `int&& r = 3;` and `const P& r = P{1, 2};`
  do not.
- String literals, `__constexpr_template__` images, vtables and construction
  vtables, VTTs, and RTTI objects and their names.

A dynamically initialized object, a volatile object or one with a volatile
member, a thread-local object, an object of a class with a `mutable` member
(7.1.6.1p4) or a non-trivial destructor (12.4p2), a reference, a temporary
initialized at startup (an `std::initializer_list` array), and a declaration
state none.

`inline_hint=yes`: a function declaration or definition states it exactly
when the function is an inline function (7.1.2).

- A function declared `inline` or `constexpr`, constructors included.
- A function defined in its class definition (9.3p2): member functions,
  static and virtual ones, member templates, members of local classes, and
  friends defined in the class (11.3p7).
- An implicitly declared special member function, one defaulted on its first
  declaration, and an inheriting constructor.
- A lambda's call operator and its conversion's static function (5.1.2p5).
- A specialization of a template declared inline (an `inline` function
  template, a member defined in its class template); an explicit
  specialization only when it is itself declared `inline` or `constexpr`
  (14.7.3p12).
- The internal `.always_inline` copy the lowering makes of a GNU
  `always_inline` function.

A member defined outside its class without `inline`, a function defaulted
outside its class (`S::S() = default;`) and an implicit specialization of a
function template not declared inline state none.

`no_inline=yes`: a function declaration or definition states it when any
declaration of the function carries GNU `noinline` (on a lambda-declarator,
the call operator; for a function template, its specializations, while an
explicit specialization keeps only its own declarations'), and the internal
`cppgm_call_terminate` helper states it.

`object_bytes=N`: a pointer parameter states it, `N` the size of the object
type, when the pointer designates the start of a complete object of that
type.

- `this` of every member function, of every constructor and destructor entry
  (base, complete, deleting), and of the `__aggregate` helpers (an aggregate
  has no base classes).
- A reference parameter to a complete object type, an `__aggregate` helper's
  reference-member parameter included.
- A class object passed by address (`pass=by_address`) and an indirect
  result (`pass=indirect_result`).
- In a call signature: a virtual call's `this`, by the called function's
  class; a pointer or reference call's parameters, as their declaration
  states them; and the object of a call through a pointer to member of a
  class without base classes.

A thunk's `this` (it points into the object at the thunk's adjustment) and
the object of a call through a pointer to member of a class with a base
class (which the pointer may adjust to a base subobject, 5.5p6) state none.

`alias=noalias`: a pointer parameter states it on `__builtin_memcpy`'s
destination and source and on a copy or move constructor's `this` and source
(12.8p2-p3), further parameters with default arguments allowed, in every ABI
entry of the constructor and every call signature that copies its boundary.
A constructor template, a converting constructor, an assignment operator,
`__builtin_memmove` and `__restrict` state none.

`classes` and `result_classes`: a direct object parameter of at most 16
bytes states `classes`, and a direct object result `result_classes`, exactly
when some eightbyte is SSE or `none` or (a parameter only) the object is
`memory`, by the rules of Object ABI Conventions above.

- `struct { double a, b; }` is `sse_sse`, `{ double d; long i; }` is
  `sse_integer`, `{ double d; E e; }` with an empty `E` is `sse_none`, and an
  empty class is `none`.
- A parameter with a misaligned member, an x87 (`long double`) member or a
  16-byte vector member is `memory`.
- A result with a misaligned member is returned through
  `pass=indirect_result`, and an x87 or 16-byte vector result states no
  classes.
- An argument past a variadic function's fixed parameters states none.

`elision=copy`: a direct call of a copy or move constructor that is not
trivial states it when the call copy-initializes an object from a
conditional expression (5.16) that is a prvalue of the object's own class.

- `T x = c ? T(1) : T(2);`, also parenthesized, nested, or with a
  throw-expression arm (`c ? T(1) : throw 1`).
- A by-value argument (`take(c ? T(1) : T(2))`), an aggregate member
  (`Agg a = {c ? T(1) : T(2)}`) and a namespace-scope object.
- A conditional with one lvalue arm and one prvalue arm (`c ? a : T(3)`, a
  prvalue).

Direct-initialization (`T x(c ? ...)`, `T x{c ? ...}`, `new T(c ? ...)`, a
mem-initializer), a `return` or `throw` operand (whose operands construct the
result in place), a glvalue conditional, a trivial copy, and a derived
class's prvalue sliced into a base (12.8p31 permits omitting the copy only of
an object of the destination's class) state none.

The lowering never states `force_inline`. It may state `projection`, which is
optional, has no consumer and is not compared.

## Structural Rules

- Every temporary must be defined before it is used.
- Every block must end in a terminator.
- Only terminators may transfer control.
- Stack slots represent addressable local storage.
- Serialization must be deterministic for the same model.
- Every block has one stack of active exception regions: a full-expression's
  region closes before any control-flow join, and every path into a block
  enters it with the same regions open. PA21's lowering keeps this, PA32's
  region removal and slot promotion rely on it, and PA26 requires it of host
  objects.

## What The Comparison Absorbs And What It Enforces

PA8 uses a separate roundtrip comparator: it reads both outputs with the
supplied `lowir-ref` and compares their canonical serialization, preserving
declarations and required model data. Its construction exercises compare
only generated-program outcomes using `lowir2native-ref`. Neither mode
uses the C++-lowering presentation rules below.

The source-to-LowIR assignments (PA10 through PA23), PA32's `-O0` driver
lane and its regression lane judge an output by comparing it with the course
solution's LowIR after a presentation canonicalization: the relaxed LowIR
comparison of `scripts/compare_results_common.pl`. PA8's debug-information
lanes for `cppgm++ --emit-lowir` (`tests/debuginfo/source` and `source-g0`)
use it as well, and also require the reference's `!dbg` locations (Debug
Locations above): each one, as often as the reference carries it, and no
other. This section is the whole
list of what that canonicalization absorbs, and of the conventions it
enforces instead. A difference the comparison accepts is on the first list;
a difference it rejects is on the second.

The rule behind the split: a normalization is worth having only when a
student can hold it as one sentence and predict, reading their own output,
whether it will compare equal. When the normalization would be harder to
understand than the convention it protects against, the comparison does not
absorb the difference; the convention is written here instead, so that a
student reads the rule before failing it rather than reading a canonical
diff of code they did not write.

### The comparison absorbs

- Names of temporaries, slots, blocks and parameters: each is renamed
  positionally within its function.
- The order of slot declarations.
- The order of top-level entries: Program Structure above defines the
  canonical presentation, and the comparison canonicalizes it.
- A declaration nothing uses, and a declaration the output makes that the
  reference does not.
- A definition of a function the reference only declares: the entity was
  emitted, and its body is not compared.
- What the output does not emit: a reference declaration, or a weak or
  internal reference definition, that nothing else in the reference names
  and that the output has no function of the same name or `object` for.
  Emission is demand-driven, and an output that emits less of what nothing
  needs compares equal; it may not emit a definition the reference lacks.
- `alias object` lines.
- The `resume` of a function stated `unwind=no`: the comparison drops the
  instruction from both sides, since the backend ends such a function's
  unwinding in termination either way.
- Internal symbol names: functions and globals that are not externally
  visible are paired by their `role` and `object` metadata, then by shape,
  then by position.
- The order of items inside a metadata group, and whether they share one
  group or stand in several: `[unwind=no, inline_hint=yes]` and
  `[inline_hint=yes] [unwind=no]` are one.
- The metadata keys that belong to later stages (`linkage`, `binding`,
  `replaceable`, `object`, `tls_for`, `keep_alias`, `prefer_local`, the
  optimizer's `force_inline`, the optional `projection`, and the few others
  that `lowir_metadata_item_ignored_for_compare` lists): they are validated
  but not compared.
- Indentation and blank lines.
- A literal is its value, not its spelling. `16`, `0x10` and `020` are one
  literal; `-1` and `4294967295` are one `i32`; `1.5`, `1.50` and `15e-1`
  are one `f64`, and so is any decimal that rounds to the same `f64`;
  `nullptr` and `0` are one `ptr`.
- The operand order of a commutative operation does not matter. For
  `binary add`, `mul`, `and`, `or` and `xor`, and for `cmp eq` and `ne`,
  the two operands may appear in either order.
- Where static destruction happens: a `fini` function may instead be a
  function registered to run at exit, provided it makes the same destructor
  calls, on the same objects and the same elements of each (a destruction
  loop stands for any element count), in the same order. The function that
  registers it may then differ from the reference's only by the inserted
  registration calls (calls of a function the output declares beyond the
  reference, passing the registered function).

### The comparison enforces

Each of these is a choice the course fixes in words. The course solution
follows them, and so must yours.

- Branch sense follows the source. A conditional branch tests the value the
  source wrote, in the source's sense: a comparison is one `cmp` with the
  source's predicate (`!=` is `cmp ne`, never an inverted `cmp eq` with the
  arms exchanged), `!x` is `cmp eq x, 0`, a bare scalar condition is
  branched on directly, and the first target of the branch is the source's
  true path.
- A retype is a `copy`. A conversion that keeps the bits and changes only
  the LowIR type (an integer to another type of the same width, an
  enumeration to its underlying type, a bit-field read completing to its
  declared type, a `nullptr` materialized as a `ptr` temporary) is written
  `copy <type> <value>`; it is not omitted and it is not written as
  `convert`.
- Instruction facts are part of the instruction. `typed`, `nowrap` and
  `exact` follow the production rules of Instruction Facts above; an output
  that misses one, or states one the rules do not, differs from the
  reference.
- A structured global states its storage type exactly when its alignment is
  greater than that of its items (Structured Global Data Initializers above).
- The facts the C++ lowering states are part of its output. `unwind`,
  `return`, `effects`, `storage=readonly`, `inline_hint`, `no_inline`,
  `object_bytes`, `alias`, `classes`, `result_classes` and `elision=copy`
  follow the rules of Facts The C++ Lowering States above, on declarations,
  definitions, parameters and call signatures alike; an output that misses
  one, or states one the rules do not, differs from the reference. PA32's
  optimized lanes leave these facts uncompared, since an optimizer may add a
  fact it proves or drop one; its `-O0` driver lane compares them.
- Instructions follow the source's evaluation order. Where the source fixes
  the order, the LowIR follows it: the right operand of an assignment is
  evaluated before the address of its left operand. Where the language
  leaves the order of independent operands open, the course fixes it left
  to right: the operands of a binary operator, the arguments of a call, and
  the loads each operand needs are evaluated in source order, and two
  independent instructions are not exchanged.
- Locals live in slots. Every parameter is stored to a slot of its own at
  entry (an indirect result's destination and `this` included), every
  automatic object has a slot, and every use loads it; a conditional value
  goes through a slot (Control-Flow Value Merges above). The lowering emits
  no `phi`, and `select` only for the bit-scan fallback. PA32 promotes
  slots to values; the lowering does not, so that its output is the
  optimizer's input in one form.
- A cleanup handler of a non-throwing function ends in `resume`:
  termination belongs to the terminate guard (PA21) or to the backend, and
  the handler does not call `std::terminate` itself. (The comparison drops
  the `resume`, so a handler that otherwise matches compares equal.)
- The conventions the assignments add. Each source-to-LowIR handout states
  the further conventions its fixtures fix, and they are enforced like the
  ones here: PA10 (short-circuit conditions branch directly, with no
  boolean slot; an array with a known initializer copies from readonly
  constant data), PA11 (which trivial and no-effect destructors are not
  called; a constructor's cleanup region per constructed subobject, open
  whether or not anything in it can throw; shared cleanup continuations for
  equal lexical destructor suffixes; one `zeroinit` per contiguous
  value-initialized span), PA12 (a named local returned in `%ret`; cleanup
  regions placed by the shape of the full-expression; `copyobj` for a
  trivially copyable prefix and for a trivial value transfer; one transfer
  per bit-field allocation unit), PA16 (constant arrays from readonly
  data), PA21 (the terminate guard and when a definition may throw; shared
  unwind suffixes), PA23 (virtual bases found through one VTT load per
  block, or a static offset for a complete object).
- Metadata of earlier forms is rejected by the validator rather than
  absorbed: the parameter keys `capture=` and `access=`, the `index`
  projections `base_subobject` and `reference_field`, the symbol key
  `trivial_lifecycle=`, the function role `unreachable`, a decay operation
  spelled as an instruction, and a reference-passing label on a parameter.
  None of them is defined above; an output must not state them.

Why these are conventions rather than normalizations: absorbing branch
sense needs a rewrite with a precondition (the compare's only use is the
branch that follows it) and a table of inverted predicates; absorbing a
missing `copy` would accept an output in which a temporary's type no
longer says what its uses need; absorbing the order of independent
instructions needs a dependency-respecting sort of every block, and would
still leave the order of side effects pinned; absorbing a missing fact
would accept a lowering that leaves the optimizer less to work from, and
absorbing an extra one would accept a promise the source never made. Each
is harder to hold than the sentence it would replace.

## Later Reserved Extensions

The following are not required in PA8, but the IR is expected to grow to cover them without
changing its overall shape:

- field/member address operations
- construction and destruction actions
- virtual-call and vtable-related operations
- richer constant/data initializers
- template-instantiated function and object definitions
- exception-aware source-language lowering that targets the exception instructions

Some later-oriented forms are already required in PA8 because later lowering milestones need
the backend boundary to be in place before they start:

- object-lowered parameter/return ABI conventions
- `copyobj`
- `zeroinit`
- structured global data initializers
- the handler-stack exception instructions:
  - `eh_try`
  - `eh_cleanup`
  - `eh_end`
  - `throw`
  - `exception`
  - `resume`

Those later operations should be added as new instruction/data forms in the same IR family,
not by replacing LowIR with a new backend boundary.
