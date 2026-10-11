# PA33 design example

These notes explain one implementation, the course solution in `dev/`. The
[assignment README](../README.md) and the course fixtures define the
required contract and quality bar. The pass order, data structures and
numeric limits below are design choices, recorded so that the solution's
dumps are explainable. The regression lane pins those dumps for the
compiler team and checks a few structural properties of them. Paths are
under `dev/src/`.

### One backend, two exception models

`lowir2native` and both `cppgm++` object paths use the same selector
(`native/select/`) and encoder (`native/x86/`) at the requested level. The
selector also follows an exception model: `lowir2native`, `.obj` objects
and `cppgm++` link mode use the course model (handler records, the course
runtime); `cppgm++ -c` to any other output uses the host model (Itanium
tables). MIR dumps come only from `lowir2native`, so every dump the lanes
read is a course-model dump. Decisions marked *host* below never appear in
one; `cppgm++ --stats-functions` and a disassembly of the object show
them.

Per function, the selector analyzes the LowIR, assigns registers to values
that cross blocks, selects instructions while allocating block-local
values, lays out the frame (`finish_frame`) and, from `-O1`, runs the MIR
cleanups and branch layout. The dump and the encoder read the same
`native::Function`.

- `-O0` is the PA24 backend, whose shapes PA24's exact fixtures pin.
- `-O1` turns on nearly everything below.
- `-O2` adds a search tree for sparse switches and, *host*, tail jumps.
- `-O3` selects exactly as `-O2`. In `cppgm++`, PA32's optimizer also runs
  on the unit at the requested level (not at `-O0`) before native
  lowering, so the LowIR differs too.

So the lane's `-O1`, `-O2` and `-O3` dumps of one LowIR file are identical
unless a switch takes the search tree.

### O0

Values used outside their defining block get registers from a hull
allocator (`native/select/global_allocation.cpp`). A hull runs, in block
layout order, from the definition through the last block where the value
is live; one linear scan by start gives at most seven GPR hulls a register
at once, preferring a parameter's incoming register and, for hulls that
cross calls, callee-saved registers. A block reserves the register of every
hull overlapping it. Functions with exception regions keep all such values
in frame homes. The dump keeps PA24's jumps (jumps to the next block,
jump-only blocks, `epilogues shared`), which the encoder threads. A compare
that materializes its result loads an immediate operand into `rdx` (unless
a 32- or 64-bit result is returned directly); division by a constant other
than an unsigned power of two uses `div` or `idiv`; each course `eh_push`
takes its own 80-byte frame slot for the handler record.

### Registers at O1 and above

`native/select/register_allocation.cpp` assigns cross-block values instead.
Its candidates are the integer and floating values used outside their
defining block, except floating parameters, which keep a `param-slot` home.
Liveness is a backward walk per value over its blocks, within one budget
per function (64 steps per instruction and block, plus 4096); when it runs
out, the remaining candidates keep frame homes.

Candidates are assigned by weighted use count, each use weighed by a static
estimate of its block's executions (a loop iterates at most ten times per
entry; blocks that only lead to a throw, resume or noreturn call weigh
nothing). A candidate takes one register for all its ranges: one that no
instruction inside them clobbers and no assigned range overlaps, and that
leaves room at every point for the block-local values live there (12 GPRs;
`xmm0` to `xmm7`). It prefers its phi web's register (a web whose ranges
are disjoint shares one register, so its edge transfers move nothing), its
tied first operand's, a parameter's incoming register, `rax` or `xmm0` for
a call result, a return or argument register it will be moved to, then
caller-saved before callee-saved. A range that crosses a call can only be
in a callee-saved GPR; a low-weight value does not open another
callee-saved register.

A candidate that fits no register for its whole life may take one outside
the loops it passes without a use, stored on each edge into such a loop and
reloaded on each edge out (`loop_splits`), or, for an integer value,
outside just the blocks that clobber or crowd its register (`home_splits`).
Otherwise it keeps a frame home: it is defined in a block-local register or
in scratch, stored, and reloaded in other blocks.

Course-model functions with exception regions still keep every cross-block
value in its frame home, since the dispatcher restores callee-saved
registers to their values at the push. *Host*, such functions get
registers, with each landing treated as a successor of the calls that enter
it, unless a landing has phis.

Block-local values are allocated as they are selected
(`native/select/allocation.cpp`) from the same registers less those of
cross-block values live there. One that finds none is computed in `r10` or
`r11` (`xmm14` if floating) and stored to a frame home. `r10`, `r11`,
`xmm14` and `xmm15` are scratch and never hold values. A function with
locals aligned beyond 16 reserves `r12` for their base. *Host*, `rbp` is a
sixth callee-saved register when the frame pointer is omitted.

### Selection at O1 and above

Selection changes in many small ways (the `level_` tests in
`native/select/`). The most visible in dumps:

- division and remainder by a constant other than 0, 1, -1 and the
  minimum `i64` use shifts and multiplies (`mul` or one-operand `imul`
  for a high half) instead of `div` or `idiv`;
- a materialized compare takes its immediate directly, and a branch on
  `x & y` becomes `test` or `bt`;
- non-bitwise `u32` arithmetic uses 32-bit forms, and narrow values and
  Booleans skip extensions their readers do not need;
- a frame, direct-symbol or register-plus-offset address used only as call
  arguments and memory operands holds no register: each argument forms it
  with `lea` and each access folds it into its operand;
- a switch becomes tests of its case runs, with a bit-test window for at
  most five targets (three runs within 64 values), or a `jump_table` (more
  than five targets, at least five cases, at most 4096 entries, one case
  per ten entries);
- in the course model, a direct `strlen` call carries `strlen_prefix=16`:
  the encoder probes the first 16 bytes inline when they lie in one page
  and calls only if they hold no NUL.

At every level, a course-model builtin `memcpy` whose result is unused
becomes `copy_bytes_dynamic`.

### Frame at O1 and above

`finish_frame` sizes the frame from the frame objects, the 48-byte
conversion scratch area when needed, and the callee-saved save area at the
bottom. From `-O1`:

- course handler records are pushed below the frame by `eh_push`, one per
  open region, except in functions with `stack_alloc`;
- object slots of one type share storage when no block naming one reaches
  a block naming the other (functions without exception regions; at most
  64 candidates);
- an object parameter whose only reader copies it whole into a slot at
  entry lives in that slot;
- after the cleanups, stores that no path reads are dropped from frame
  objects whose every access is direct (`dead_frame_stores`), and the
  objects are reordered so that the most accessed per byte lie nearest the
  frame register (`ordered_frames`). Neither runs in variadic functions or
  with pushed handler records. Dead stores also stay with host landings;
  the order also stays with any `eh_push`, an address that may reach
  outside its object, or over-aligned locals;
- frame space is not reclaimed after the cleanups, so a binding may name
  an object that no instruction touches any more;
- every return has its own epilogue (`epilogues duplicated`);
- *host*: in a non-variadic function where RSP never moves after the
  prologue (no `stack_alloc`, `va_arg`, i128 conversion, stack argument
  area or over-aligned locals) the frame pointer is omitted. Such a
  function may be shrink-wrapped, running a frameless early-return region
  before a `prologue` instruction, and a leaf that no longer uses its
  frame drops it.

### Cleanups and layout at O1 and above

After `finish_frame`, one linear walk each (`native/select/mir_cleanup.h`)
merges adjacent constant stores, forwards a return's own frame stores into
its loads, turns reloads of a home that a register still holds into copies
or nothing, folds copies and add pairs into `lea`, redirects a load from a
frame object that a bulk copy just filled to the copy's source, and drops
repeated `test`s, redundant copies and zero extensions.

Then `layout_branches` (`native/select/branch_layout.cpp`):

- a latch jumping back to a loop test of at most 12 instructions ends with
  a copy of the test instead (`rotated_loops`);
- a jump to a test of a register that the jumping block set to a known
  value goes straight to the decided successor;
- blocks are chained along their likeliest edges by static estimates,
  cold chains last; *host* shrink-wrapping runs here;
- in a function with no stack frame and no saves, a jump to a block that
  only returns becomes the return;
- jump-only blocks are threaded away (`threaded_blocks`; a cycle of them
  stays);
- in the final order a jump to the next block goes and `jcc` to the next
  block followed by `jmp` becomes one inverted `jcc` (`layout_branches`);
  where neither target of `jcc; jmp` is next, the `jmp` goes to the colder
  one.

The laid-out dump is the encoded control flow: blocks may fall through, and
the encoder adds no threading of its own.

### O2 and O3

`-O2` adds two things; `-O3` adds nothing in the backend.

- Switch case-run tests numbering at least four, with at least two gaps
  between them, are searched by halves: a compare with the middle test's
  value branches to a deferred subtree block, `^switch.N` in the dump. A
  selector held in a scratch register keeps the linear tests.
- *Host*: a call that the function returns from at once (`ret` next in its
  block, or first in the block it falls into) becomes `tail_jump`, the
  epilogue and then a jump, when the function is not variadic, takes no
  frame address and never names RSP, the call has no stack arguments,
  landing or `strlen` prefix and is not noreturn, and the `ret` returns
  nothing or `rax` or `xmm0`. No dump shows `tail_jump`; the census counts
  it among a loop's `calls`.

### Reading a dump

The `frame` section gives the final `stack_size` and `scratch_bytes`,
`frame_pointer keep` or `omit` (always `keep` in course-model dumps),
`epilogues shared` or `duplicated`, `aligned_base` for over-aligned locals,
the `preserve` list of saved callee-saved registers, and one binding per
frame object: `slot $s` for a LowIR slot, `param-slot %p` for a
parameter's home, `temp %v` for storage made for a value (usually its
home). Instructions name final physical registers. A phi's or split
value's edge transfers end the predecessor, or fill an edge block
`^from~to` when the predecessor ends in a conditional branch.

`copy_bytes NxA` and `zero_bytes NxA` print the same at every level; from
`-O1` the encoder expands every one of up to 128 bytes into direct moves,
which leave `rdi`, `rsi`, `rcx` and `rax` intact. The dump does not print
removed instructions or whether handler records are pushed; a smaller
`stack_size` shows the latter.

### Statistics

`lowir2native --stats` (telemetry builds) prints one `lowir2native_stats`
line on standard error, summed over the functions. `loop_splits`,
`home_splits`, `jump_tables`, `rotated_loops`, `threaded_blocks`,
`layout_branches`, `ordered_frames`, `dead_frame_stores` and
`strlen_prefixes` count the steps above and stay zero at `-O0`.
`global_registers` counts cross-block values given a register by either
allocator, `homes` the frame homes made, `spills` the block-local values
that found no register, and `hull_fallbacks` the values left in homes by
the liveness budget. From `-O1`, `mir_instructions` counts what layout
left in the blocks, which can include removed instructions the dump omits.
The encoding counters, `native_returns` through `code_bytes`, stay zero
unless `-o` runs the encoder.

`cppgm++ --stats-functions` prints, for each function the object backend
selects in the model of the output, one `function_census` line and one
`loop_census` line per natural loop of its final MIR
(`native/object/function_census.h`). `function_census` gives the MIR
count, the frame loads, frame stores and register copies in the MIR, and
the selector's counters for the function: `planned` (cross-block values
given a register), the block-local allocator's grants, releases and busy
counts, `spills`, `frame_homes`, and where cross-block values were defined
(`defined_in_plan` in their register, `defined_other_register` in a
block-local register then stored, `defined_frame` in scratch then stored).
`loop_census` gives each loop's blocks, instructions, calls, exception
instructions, nesting depth and frame operands, with the function's frame
binding and callee-saved counts.
