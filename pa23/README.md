## CPPGM Programming Assignment 23 (`cppgm++ --emit-lowir`)

### Overview

PA23 extends PA22 with the first supported object layouts that require more than the
earlier single-vptr, non-virtual-base ABI:

- virtual inheritance for shared base-subobject layout and access
- polymorphic multiple inheritance with more than one active vtable view
- pointer-form `dynamic_cast` across sibling polymorphic bases
- RTTI / `typeid` through non-primary polymorphic base views

PA23 still produces LowIR. It does not introduce a new output format.

### Prerequisites

You should complete Programming Assignment 22 before starting this assignment.

You will want to reuse:

- the preprocessing and tokenization pipeline from PA1-PA4
- the PA5 AST as the syntax boundary
- the PA6-PA7 semantic foundation
- the PA10-PA22 LowIR lowering path
- the PA8 LowIR contract

### Starter Kit

The starter kit contains:

- `pa23/README.md`, `pa23/Makefile`, and the test scripts in `pa23/scripts/`
- your cumulative `dev/cppgm++.cpp` compiler entry point
- the `pa23/cppgm++.cpp` symlink back to `../dev/cppgm++.cpp`
- shared support sources and headers under `dev/src/`
- the grammar for this assignment called `pa23.gram`
- an HTML grammar explorer of `pa23.gram` in the sub-directory `grammar/`
- a checked-in local test suite under `tests/`

Students should implement the assignment in `dev/cppgm++.cpp` and any reusable
student-owned helpers they add under `dev/src/`. The assignment directory, grammar files,
test fixtures, comparison scripts, and checked-in reference outputs are support
files, not implementation files to edit for normal solutions. Reuse your earlier compiler infrastructure when implementing this milestone.

Use the supplied reference tools to inspect example output. Tests compare
your compiler with the checked-in contract references.

### Input / Command-Line Arguments

Behaviour is undefined unless the command-line arguments match:

    $ cppgm++ --emit-lowir -O0 -o <outfile> <srcfile1> <srcfile2> ... <srcfileN>

`-O0` is the PA23 test mode. Other optimization levels are later optimizer work and
are not required for this milestone.

### Output Format

`cppgm++` shall write LowIR text to `<outfile>`.

The authoritative LowIR definition is `../pa8/lowir.md`. PA23 extends the PA22 lowering
surface only by making more of the C++ source language lower into the already-defined LowIR
family.

LowIR top-level declaration/definition order is a presentation convention, not
a dependency order. Reference outputs and canonical dumps use the order defined
in `../pa8/lowir.md`: `declare global`, `declare function`, `global`, then
`function`, but the relaxed LowIR comparison canonicalizes top-level entries
before comparison. Your output must still be repeatable for the same
inputs; `../pa8/lowir.md` defines the canonical reference presentation and
notes where internal LowIR symbol names are only a presentation tie-breaker.
Your output must also preserve order-sensitive LowIR regions when they are present: instruction order inside
blocks, item order inside structured globals, vtable slot order, and action
order inside generated initialization, finalization, constructor, destructor,
and cleanup bodies.

The generated LowIR must be well-formed and must match the checked-in `.ref` files under
the relaxed LowIR comparison used by the harness. That comparison still checks the
semantic LowIR shape and required IR facts, but it does not make helper metadata
presentation or other non-semantic text details part of the student contract.
The instruction facts and the boundary facts the lowering states (`unwind`,
`inline_hint`, `object_bytes`, `storage=readonly` and the others) are compared
exactly; `../pa8/lowir.md` ("What The Comparison Absorbs And What It
Enforces") lists both sides.

### Error Handling

If an error occurs during preprocessing, tokenization, parsing, semantic analysis, or LowIR
generation, `cppgm++` shall `EXIT_FAILURE`.

The output file is not required to be meaningful on failure.

### Standard Output / Error

Standard output and standard error are ignored for automated testing of `cppgm++`.

You are free to use them for debugging, tracing, or diagnostic messages.

### Testing

Tests compare your output with the checked-in references using the LowIR
comparison described in [the format guide](../pa8/lowir.md). The `Makefile` invokes
`cppgm++` with `--emit-lowir -O0`.

The local checked-in tests live in `tests/general/`. They exercise PA23
source-to-LowIR behavior over virtual inheritance, non-primary polymorphic
views, sibling `dynamic_cast`, and RTTI through adjusted base views.

For each test case `x`:

- `cppgm++` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

PA23 is tested against generated LowIR text using the relaxed LowIR comparator described
above. For execution feedback, use the supplied `lowir2native-ref -O0`
backend introduced in PA8. Your own native backend is implemented in PA24.

The shipped PA23 tests are the contract for this milestone.

Self-checking controls in `tests/controls/` state their expected outcome in
their leading comment (`expect exit N` or `expect reject`) or in an
`x.expect` sidecar. `make test` compiles each with your
`cppgm++ --emit-lowir -O0`, builds the LowIR with the supplied
`lowir2native-ref -O0` and requires the stated exit status (and the output in
`x.stdout`, when present), or requires your frontend to reject the program.
The LowIR text of a control is not compared. `make test-controls` runs only
the controls; `make check TEST=tests/controls/<name>.cpp` runs one.

### PA23 Syntax Spec

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa23.gram`. The grammar defines accepted syntax only;
the PA23 semantic and lowering requirements are defined by the Assignment
Boundary and Out Of Scope sections below.

As in the earlier assignments, that grammar defines accepted input syntax only. The output
format for `cppgm++` is specified by this README, PA8 `lowir.md`, and the checked-in
`.ref` files.

PA23 does not add a new source-language grammar format. It instead enables more
of the already-accepted C++11 syntax to participate in semantic analysis and
lowering.

A checked-in HTML grammar explorer for that grammar lives in `grammar/`. Treat
`pa23.gram` as the source of truth.

`pa23.gram` uses the same token vocabulary and the same extended BNF operators as
`../shared/source.gram`.

If this README and `pa23.gram` appear to disagree about source syntax, treat `pa23.gram`
as authoritative. If this README and PA8 `lowir.md` appear to disagree about LowIR syntax,
treat `lowir.md` as authoritative. If they disagree about the PA23 lowering slice, treat the
`Assignment Boundary` and `Out Of Scope` sections below as authoritative.

### Assignment Boundary

PA23 supports the following in addition to the PA22 subset:

- virtual inheritance for shared base-subobject layout in complete objects
- field access through shared virtual bases
- supported constructor and argument forwarding cases with the Itanium calling convention:
  a function takes `this` (for a member) and its visible arguments, and a base-object
  constructor or destructor of a class with virtual bases also takes the VTT. No parameter
  carries a hidden virtual-base address. A reference, a pointer, a by-value parameter or
  an implicit object reaches its virtual bases through the object's vtable; a by-value
  parameter is a complete object, so it may also use its static layout. A base-object
  constructor or destructor reads its own construction vtable from the VTT
- reaching a virtual base through a reference or pointer by that base's own vbase-offset
  row, even when it is a virtual base of another virtual base
- virtual-base construction by the most derived class's constructor, implicit and
  synthesized constructors included: depth-first and left to right, so a virtual base's
  own virtual bases are constructed before it, with destruction in reverse order. A
  synthesized copy constructor or assignment operator reaches the virtual bases of an
  object that may be a base subobject through that object's vtable
- polymorphic multiple inheritance with separate vtable views for non-primary polymorphic
  bases, virtual destructors included
- virtual dispatch through primary views whose virtual-base ABI carries
  function/adjustment rows, and through non-primary polymorphic base pointers
  and references
- pointer-form `dynamic_cast<T*>` across sibling polymorphic bases in the supported object
  model
- `typeid(expr)` through supported non-primary polymorphic base lvalue views
- member pointers over these layouts: a pointer to a virtual member function names the
  function's entry in its class's primary vtable (the entry that returns the overrider's
  result unadjusted); a conversion between pointers to members of a class and of a
  non-virtual base adjusts `adj` by that base's offset; `.*` and `->*` first convert the
  object to the member's class, through a virtual base where needed. A conversion between
  pointers to members of a class and of one of its virtual bases is ill-formed (4.11p2,
  5.2.9p12), and in deduction it is a substitution failure

Within this milestone, PA23 should produce valid LowIR for ordinary source programs over
that subset. That LowIR should be accepted by PA24 `lowir2native` for the supported cases.

To complete PA23, implement these goals:

1. Shared virtual-base layout.
   Complete objects with a virtual diamond should expose one shared base-subobject at a
   deterministic offset. Every path through one virtual base reaches the same subobjects
   inside it, so a non-virtual base of a shared virtual base converts unambiguously
   however many paths reach that virtual base (10.1). The virtual bases follow the
   non-virtual part in inheritance graph order (Itanium C++ ABI 2.4 III). Each is placed
   as a non-virtual base is: from the data size, so a virtual base can occupy a non-POD
   base's tail padding, and an empty virtual base at offset `0` when no empty subobject of
   its type is there. A class with no non-virtual dynamic base takes a nearly empty
   virtual base (one whose non-virtual part is its vptr alone) as its primary base
   (2.4 I.b). That base sits at offset `0` and shares the class's vptr and vtable. A
   virtual base that is some base's primary base is not placed again: it lies with the
   subobject that claims it. The claimant is the first base subobject in inheritance graph
   order (virtual bases taken where the walk first meets them) whose primary base it is,
   or that subobject itself when the walk of its own bases reached the virtual base first.
   The class's own primary base is not claimed this way. The empty subobjects of the
   virtual primary base and of each claimed base count when later empty bases are placed.

2. Polymorphic dispatch over adjusted vtable views.
   Calling a virtual through a class with virtual-base adjustment rows must select the
   requested logical slot. Calling through a later polymorphic base must lower through the
   correct vtable view and apply the required `this` adjustment. A virtual function a
   class declares has its own entry in the class's primary vtable unless it overrides a
   function of the primary base without a return adjustment (Itanium 2.5.2). So an
   overrider of a function of a non-primary or virtual base occupies such an entry, which
   calls through the class use, in addition to its adjusted entries in the other tables. A
   final overrider runs on the subobject that the table's inheritance path leads to, not
   on another subobject of the same class. Where paths meet at a shared virtual base, an
   overrider dominates another only if its subobject contains the other's. The function
   has two final overriders, which is diagnosed (10.3p2), when one overrider is reached in
   two subobjects or when two overriders lie in subobjects neither of which contains the
   other. Each vtable segment must contain only the vcall-offset and virtual-base-offset
   rows owned by that segment; in particular, vcall rows belonging to a secondary
   virtual-base view must not enlarge the primary segment or its address point.

3. Sibling cross-cast support.
   Pointer-form `dynamic_cast` across sibling polymorphic bases should lower into the
   supported RTTI / vtable-view scan.

4. RTTI through non-primary views.
   `typeid(expr)` should observe the dynamic type through a supported non-primary
   polymorphic base reference.

5. Exception boundaries of the lifecycle entries.
   Constructing a variable or an array element with a constructor that cannot throw
   opens no cleanup region around that call: the objects completed before it need no
   unwind path there. An implicitly declared constructor cannot throw when none of the
   constructors it calls can (15.4p14), also when its class has virtual bases; a
   user-provided constructor without an exception specification may throw. A
   destructor's implicit exception specification counts its virtual bases' destructors
   too. Each entry of a destructor that cannot throw, the base-object entry included,
   runs under PA21's terminate guard when its definition may throw.

### Out Of Scope

The following are explicitly out of scope for PA23:

- virtual-base constructor, copy, assignment, and destructor sequencing beyond the
  supported cases above
- virtual inheritance combined with the unsupported special-member or exception cases
- toolchain-driver and host-linker integration

Inputs that rely on those features have undefined behaviour for this milestone.

### Stage Handoff

The intended next stage is PA24, which lowers the completed LowIR family to
native code before PA25 turns the source pipeline into a practical `cppgm++`
toolchain driver and standard object-output flow.

So PA23 should leave behind:

- a stable multi-vtable / virtual-base LowIR lowering path
- deterministic lowering for the supported sibling-cast and RTTI-view cases
- explicit remaining deferrals only where the practical toolchain and remaining ABI/runtime
  work need to take over
- enough stable source behavior that PA25 can start carrying complete
  programs as C++ end-to-end tests through the practical driver/link path

### Design Notes (Non-Normative)

PA23 should extend the existing object-model and RTTI lowering path, not replace it.

The same monotonic-extension rule applies here:

- PA23 should add its new behavior only when the source actually uses the supported PA23
  feature set
- it should not perturb PA22 outputs for programs that remain entirely within the PA22
  subset
- in practice, the richer vtable / RTTI layout should stay source-driven rather than
  changing earlier single-vptr cases unnecessarily

For Itanium-layout vtable segments, emit any vcall-offset rows before the
virtual-base-offset rows, followed by offset-to-top, RTTI, and the function
slots. Each segment's address point follows that segment's own negative rows;
segments of one group may have different numbers of them. The
virtual-base-offset rows follow the Itanium ABI (2.5.2): the first virtual
base in inheritance graph order sits nearest the address point, after the rows
a primary base already places there, and a VMI descriptor's `__offset_flags`
name the row.

A virtual base's table also carries vcall offsets, and so does a class's
primary table when its primary chain reaches a virtual primary base. There is
one vcall offset per virtual function signature the virtual base declares:
those of its non-virtual primary chain first, then those of its other
non-virtual bases. They are placed as clang places them, so a virtual primary
base's vcall offsets come before its vbase offset. An overrider reached
through a virtual base, from that base's table or from a secondary table
inside it, is a virtual thunk (`_ZTv<n>_<k>_`). The thunk moves `this` to the
virtual base by the fixed part `n` and adds the vcall offset at `k`, so a
class that another compiler derives can use it. The entry's subobject is the
nearest class in the table's primary chain that declares an override. For a
covariant entry it is the nearest class below the overrider's class whose own
entry is not a thunk, as g++ picks it. A covariant result moves to the virtual
base that its overridden result lies in, and then by the fixed offset inside
it.

A primary base of a secondary base shares that base's table and has no table
of its own in the group. A virtual primary base may lie elsewhere in the
class, lost to another subobject. Its entries are null where their walk to the
declaring class passes a class that lost its own primary base, judged where
each class of the chain lies. A call, or a pointer to the member, dispatches
through the place of the entry that returns the overrider's result unadjusted,
as g++ does. It never uses a covariant thunk's place, which a lost primary
base may leave null. The group's tables follow its primary table in clang's
order: the non-virtual bases' secondary tables depth first, then the virtual
bases' tables in inheritance graph order.

A thunk states its target's boundary facts, except that its `this`, which
points into the object at the thunk's adjustment, states no `object_bytes`;
a deleting destructor's thunks state `unwind=no` as D0 does.

Every thunk that a class's tables include is defined with its function and
only referred to elsewhere. A virtual destructor's deleting entry (D0) and its
thunks are defined with the destructor too, not where the vtable is emitted.
Only a weak vtable whose destructor is defined elsewhere keeps a weak copy of
D0 beside it. g++ also defines a static-`this` covariant thunk (`_ZTch0_...`)
beside a virtual-`this` one, as the target that its thunk code jumps to; no
vtable names it, so it is not emitted here. The key function is the first
non-inline, non-pure virtual function the class declares.

The VTT follows Itanium 2.6.2. It holds the primary vtable; a sub-VTT for each
direct non-virtual base that has virtual bases (without its virtual sub-VTTs);
the secondary virtual pointers in inheritance graph order, one for each
dynamic base that has virtual bases or lies in a virtual base and is not a
non-virtual primary base; then a sub-VTT for each virtual base that has
virtual bases. A base-object constructor stores each secondary virtual pointer
of its VTT where the complete object placed that subobject, and takes the rest
from its own vtable group. A class that is both a non-virtual base and a
virtual base is constructed with the sub-VTT of the subobject being built.

A construction vtable group (`_ZTC<complete><offset>_<base>`) lays the base
out as clang lays out a construction group, with the complete object's offsets
and vcall offsets. A virtual primary base that does not share its class's
place there gets its own table. The entries are the base's own; g++ instead
keeps a lost primary base's entries there and adds no vcall offsets to the
base's own table. A destructor's entries are null.

A synthesized constructor or destructor entry states `no_inline=yes` only
when the source function carries GNU `noinline`, whether or not the entry has
its own object symbol or a retained definition.

Find a virtual base where the object says it is: its vbase-offset row in the
object's vtable, or a static offset when the object is a complete object of a
known type (a named variable, a temporary, a by-value parameter). A by-value
parameter given a derived object is copy-constructed (or, from a prvalue,
move-constructed) from the argument's base subobject, whose virtual bases are
found through its vtable. Hidden virtual-base arguments would make the calling
convention depend on which translation unit sees the complete type, and they
would differ from what g++ and clang pass: a g++ caller of a base-object
constructor or member function built here would hand it no such arguments.
Within a base-object constructor or destructor, read the rows from the
construction vtable that the VTT names, not from the object's vptr, which may
still hold a base's table. The VTT never changes, so one load of the table and
of each row serves a block (the references do this, and the comparison does
not absorb a reload). Vtables, construction vtables and VTTs are
`storage=readonly` data, which the optimizer reads through once the VTT is
known.
