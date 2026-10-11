# PA32 design example

These notes explain one implementation. The [assignment README](README.md)
and course fixtures define the required contract and quality bar. The pass
order, data structures and numeric limits below are design choices.

### One Design: The Course Solution's Optimization Levels

Everything in this section describes the course solution. It is one way to
meet the quality bar, and its numbers (the 1,536-instruction caller budget,
the 40-, 24- and 6-instruction callee caps, the four-pass unrolling limit
and the rest) are its choices, recorded so that its outputs can be
explained. None of them is a requirement; the course fixtures do not
compare your output's shape with the course solution's.

`lowiropt`, `cppgm++ --emit-lowir -O<n>` and the object paths
(`cppgm++ -c -O<n>` from source or from LowIR text) run one optimizer over
the typed program that the LowIR reader or the lowering builds:

- `-O0` runs no transform; the program is written back in canonical form.
- `-O1` runs three inlining waves over the direct-call graph. The first
  wave runs the per-function pipeline below on every function, the later
  waves on the functions they change, and each wave is followed by pruning
  of unreachable definitions.
- `-O2` runs the same waves with larger single-call limits and more
  admission rules, adds memory, loop, switch and branch rewrites to the
  per-function pipeline, then propagates arguments into internal functions
  and makes a final pass over every function.
- `-O3` is `-O2` plus full unrolling of small counted loops and value
  numbering across back-to-back diamonds.

Every pass bounds its work: it is close to linear in the size of the
function, or a stated cap limits it, and reaching a cap only makes the pass
do less.

### The Per-Function Pipeline

1. Slots and scalar rounds. A round splits object slots into fields,
   promotes scalar slots to SSA values, then runs scalar rounds (below).
   At most four rounds run; after the first, a round in which splitting and
   promotion change nothing ends the loop, as does running out of slots. A
   function with an object slot runs scalar rounds before its first split,
   so the copies that fill the slot are simplified first; that run counts
   as one of the four rounds.
2. Remove protected regions that nothing can unwind out of, and coalesce
   scalar copies. If either changed the body, split and promote again and
   run scalar rounds.
3. Rewrite loops; at `-O3` unroll loops; at `-O2` and above form switches.
   Scalar rounds follow if any of these changed the body.
4. Record whether the function can unwind, for its callers.
5. At `-O2` and above: let a local that every return copies whole into the
   indirect result become that result, then rewrite branch compares.
6. Compact the body, so a caller copies only live code.

A scalar round runs sparse conditional constant propagation (Wegman and
Zadeck, over executable edges), CFG cleanup, value simplification, memory
forwarding, dead-code removal, jump threading and CFG cleanup again. Rounds
repeat until one changes nothing, at most four times. A one-block function
runs value simplification, memory forwarding and dead-code removal instead
(at most four rounds), then one CFG cleanup. A pass that found nothing to
do is not run again until the body changes.

**Value simplification** walks the dominator tree once per run:

- It folds `copy`, `unary`, `binary`, `cmp` and `convert` of constants the
  way the back end computes them: integers wrap to their width; division,
  remainder and shifts fold only where they cannot trap or depend on an
  out-of-range count; `f64` arithmetic is x87 extended precision rounded
  to double.
- It applies algebraic identities (`x + 0`, `x * 1`, `x & -1`, `x - x`, a
  mask that clears no bit `x` can have, and so on), reassociates `add`,
  `mul`, `and`, `or` and `xor` chains with constants, merges constant
  `index` steps, orders commutative operands, collapses a phi whose inputs
  are all one value, and merges two phis of a block that agree on every
  input.
- It reuses a pure computation already made in the same or a dominating
  block (hashed by operation, type and operands).
- When a block's only predecessor ends in a branch, the edge's facts hold
  in every block the entered block dominates: the condition is nonzero on
  the true edge, and an edge that proves `x == k` for a literal `k` (`eq`
  taken, `ne` not taken, and unsigned tests that pin `x`) replaces `x` with
  `k`. A two-input phi that picks the value its edges prove equal
  (`min(n, max)`, `max(n, 0)`) becomes that value.
- A nonvolatile integer or pointer load at a constant offset into a global
  of `storage=readonly`, or into an internal global the unit never writes
  (every use of its address reads through it and no other global's data
  holds it), becomes the stored constant or address; replaceable and
  thread-local globals are never read this way. `strlen` of such byte data
  with a known NUL becomes its length (at most 4,096 data items), and
  `memset(p, 0, n)` and `memcpy(d, s, n)` with a literal `n` of at most 128
  become `zeroinit` and `copyobj`.
- At `-O2` and above an indirect call whose target is the `addr` of a
  function becomes direct, and `memcmp` of a literal length of at most 16,
  compared only with zero, becomes loads of one or two chunks of each side.

**Memory forwarding** walks the dominator tree with facts: the value a
nonvolatile load read or a store wrote at an address and type, and byte
ranges known to be zero. A load reuses a fact of its address and type (or
of the other integer or pointer type of that width, through a retyping
copy); a load in a known-zero range reads zero.

- An address is a slot, global or pointer temporary plus a constant offset.
  Distinct slots and globals never overlap, nor do disjoint ranges of one.
  A private slot, whose address reaches only loads, stores, pointer copies,
  constant `index` steps, `copyobj` and `zeroinit`, is reached by no other
  pointer. Two `typed` accesses, one of a pointer and one of a non-pointer
  wider than a byte, name different objects unless they share a base.
- A write ends every fact it may alias. A call that may write memory, an
  atomic operation, `va_start` or `va_arg` ends every fact except those of
  private slots. At most 64 facts and 16 zero ranges are live.
- A block inherits its immediate dominator's facts less whatever the
  blocks between them may write (at most 32 such blocks, none a handler).
  A handler block starts with no facts.
- Object copies whose source bytes are known constants (at most 256 bytes)
  or known stored values (at most 32 bytes) become zeroing and stores;
  writes to a private slot that nothing reads are removed.
- After the walk, a load that precedes every write in its block can become
  a phi: through a phi of addresses, when every edge's value is known; at
  a join of at most 8 predecessors, when some know the value and the
  others end in a jump to the join and receive the load (no edge is
  split); or at most 3 single-predecessor blocks below a loop header, when
  every predecessor of the header knows the value.
- At `-O2` and above a store of the value the location already holds is
  removed, and a store to a slot that is dead on one side of a two-way
  branch moves to the other side when that side has no other predecessor.

**Dead-code removal** marks from the instructions that must stay and
removes the rest: unused pure values, nonvolatile loads, stack allocations,
and calls whose boundary is `readnone` or `readonly`, `unwind=no` and not
`noreturn`, dead phi cycles included.

**CFG cleanup** (at most 8 rounds per run) ends a block after a `noreturn`
call, turns a branch or switch with a literal selector or a single target
into a jump (a switch compares at its selector's type), removes unreachable
blocks and conditional edges into a block that is only `unreachable`,
bypasses jump-only blocks other than the entry and handler blocks (into a
phi block of at most 32 predecessors, when the phi inputs agree), and
merges a non-handler block into its only predecessor when that predecessor
jumps to it.

**Jump threading** lets a block whose only predecessor branched on `c` take
the known side of its own branch on `c`, and lets a predecessor whose phi
inputs decide a small block's branch (at most 8 instructions besides its
phis) jump to the decided successor. At `-O2` and above, facts from
dominating branches fold the compares they decide, and a predecessor whose
recent path decides a test block's compare against a literal jumps to the
decided side (copying at most 6 of the test block's instructions).

**Slot promotion** turns a scalar slot (any integer up to 64 bits, `ptr`,
`f32`, `f64`) into SSA values when every use is a direct nonvolatile `load`
or `store` of its type, or of the other integer type of its width. Phis go
at the iterated dominance frontier of its stores over ordinary edges,
semi-pruned (a slot no block reads before storing gets none). One run's
phis have at most 4,096 inputs plus four per other instruction; past that,
the remaining slots keep their memory. A slot that would need a phi in a
handler block stays a slot (see also Safety Rules).

**Slot splitting** replaces an object slot with up to 16 typed field slots
and promotes them. Every use of its address (through `addr`, constant
`index` steps and pointer copies) must be a nonvolatile scalar `load` or
`store`, a `copyobj` or `zeroinit` at a constant non-negative offset, or a
call's indirect-result argument, and the typed accesses must agree where
they meet; copies and zeroing then expand per field. At `-O2` and above an
address-taken scalar slot is rewritten the same way, and eightbytes that
copies move whole may become one integer field.

**Region removal** carries the stack of active regions over ordinary edges
from the entry. A region is kept when an instruction inside it may unwind:
a `throw`, a `resume`, or a call that may unwind and is not to a callee
proven not to. Other regions lose their opener and `eh_end` instructions,
and their handlers become unreachable. Conflicting stacks at a join, or
catch and exception instructions in ordinary code, keep every region. At
`-O2` and above a cleanup whose handler only ends the region and resumes
goes even when the region can unwind.

**Copy coalescing** merges adjacent nonvolatile scalar load/store pairs
that copy a contiguous range between storage that cannot overlap (distinct
slots or globals, a private slot, two `noalias` parameters) into one
`copyobj`, and forwards copies made through a temporary slot. At `-O2` and
above an object slot of at most 64 bytes written only with zeros is zeroed
whole, and a slot filled by one copy from constant data and otherwise only
read reads that data instead.

**Loop rewriting** works on natural loops found from back edges into a
dominating header; irreducible loops and loops containing a handler block
are not reported. It makes one rewrite per scan, at most 8 scans per run,
and tries at most 16 candidate loops per scan, innermost first:

- A loop with one latch and one exit, tested by `ne` or `eq` of an
  induction variable against an invariant, whose body holds only phis,
  nontrapping pure values and control flow, and whose values are unused
  after it (except the tested value, which equals the bound), is bypassed.
- A fill loop, one nonvolatile store per pass of a repeated-byte value
  through a pointer induction stepped by the store's size, becomes a call
  to the runtime's `memset` builtin in the preheader.
- At `-O2` and above a copy loop between two pointer inductions becomes a
  `memmove` builtin call (the loop still runs when the destination starts
  inside the source), and a counted loop of at most 64 passes and 4 stores
  of literals or of values loaded from storage it does not write becomes
  preheader stores or copies (at most 256 bytes each). A few smaller
  rewrites widen fills, replace count-down counters with pointer tests and
  hoist a load that the loop only stores back.

**Branch compares** (`-O2` and above): a branch on an expanded `memcmp`
equality tests its chunks one at a time; a branch on a compare of a phi
with at most 4 predecessors gives each predecessor its own compare; a
compare used only by branches in other blocks is computed again beside
each of them. **Switch formation** (`-O2` and above) turns tests of one
value against literals (at most 24 blocks, 256 case values) into a
`switch` when that needs fewer runs of consecutive values per target.

### Inlining and Pruning

The direct-call graph gives every function definition a dense index; its
strongly connected components mark recursion (a function on a call cycle)
and give a callee-first order. Each wave visits the functions in that
order, deciding the calls in a function before visiting its callers.

- The ordinary wave inlines admitted calls into each function and then
  runs the per-function pipeline on it, so every callee is simplified
  before its callers decide.
- The single-call wave, after pruning, applies the single-call policy
  (forced callees and the `-O2` loop rules below also apply): removing
  dead callers can leave a definition with one call.
- The late wave admits small optimized bodies with a fresh caller budget.

The later waves run the pipeline only on functions they changed. `lowiropt`
and `--emit-lowir` leave a this-adjusting thunk (object name `_ZTh...` or
`_ZTv...`) as lowered.

A callee is never inlined when it is on a call cycle (apart from the
recursive rule below), is `no_inline=yes` or replaceable, is variadic,
contains `va_start`, `va_arg`, `stack_alloc` or an EH instruction
(`eh_*`, `throw`, `resume`, `exception`, `exception_selector`; but see
folded copies below), or does not match the call's arity, result type and
parameter passing. In handler code (blocks a handler entry dominates) a
callee that may unwind stays a call. A callee without EH instructions may
be inlined inside an active region; its cloned calls stay in that region.

Sizes count a callee's instructions outside cold blocks (blocks from which
every path ends in a `noreturn` call); cold code is copied but not charged.
At a call site inside one of the caller's loops the ordinary and late-wave
caps double.

| policy | `-O1` | `-O2`, `-O3` |
|---|---|---|
| `force_inline=yes` | always, not charged | |
| trivial leaf (no call), ordinary wave | 4, not charged | |
| `inline_hint=yes` | 48 | |
| one block, one return, no call | 40 | 48 |
| other, no call outside cold code | 24 | 48 |
| calls outside cold code | 6 | |
| per-caller budget, each wave | 1,536 | |
| single-call callee | 512 | 1,024 |
| single-call budget per caller | 1,024 | 2,048 |
| late wave: leaf / hinted / other | 40 / 48 / 6 | 48 if no call |

In the late wave at `-O1` a one-block leaf uses 40 even when hinted. The
ordinary wave charges the larger of a callee's input size and its current
size. In both waves a loop-free multi-block callee whose blocks reachable
from the entry without a call include a return is also admitted when
those blocks hold at most 24 instructions (48 if hinted) and the callee at
most 96 (160 if hinted), unless its hot code calls an allocation function.
A callee with a loop and more than one call site is not admitted by the
size caps unless it is hinted (or, at `-O2`, call-free within 48).

The single-call policy, used by the ordinary wave when its caps reject a
call and by the single-call wave, takes a discardable callee (weak or
internal, no lifecycle role, not `object_root=yes`, not replaceable, not a
thread-local wrapper) with exactly one direct call and no other reference;
pruning then removes its definition. At `-O2` it also takes an internal
callee whose calls all lie in one function, when the copies together fit
its cap.

Three unit budgets bound growth, each the input's instruction count but at
least 10,240: one for single-call copies, one for copies of
`force_inline=yes` callees, and one for all other copies.

In the ordinary wave a callee with EH instructions (at most 256
instructions; with a loop only if hinted) that receives a constant
argument, an integer literal or the address of a readonly non-replaceable
global, may still fold away: a private copy receives the constants and runs
the per-function pipeline, and it replaces the call when no EH instruction
remains, it has at most 40 instructions (80 if hinted), it fits a budget of
2,048 such instructions per caller, and in handler code it cannot unwind.
A callee whose copies fail 16 times is not copied again in that wave.

At `-O2` and above the late wave also makes partial copies: for a callee
kept out of line (scalar or no result, no object or indirect-result
parameter), its side-effect-free entry and fast returning paths, calling
the callee where work begins (at most 24 instructions; 40 if hinted, 48
with at most two call sites, 8 if its hot code allocates). A loop-free,
EH-free function of at most 64 instructions that calls itself at least
twice takes copies of its own body at those calls, two levels deep at most
and 320 instructions in all.

Further rules adjust admission by call site and caller. For example, at
every level a small caller (at most 32 instructions) with a call-free path
to a return takes no callee larger than 16 except forced ones and the
function-pointer loops below, so that it stays inlinable. At `-O2` and
above a call in a block from which every path ends in a `noreturn` call or
`unreachable` takes only forced callees and callees of at most 4
instructions; callers the unit cannot inline elsewhere accept somewhat
larger callees with few call sites; and a callee whose loop calls through
a function the site passes (at most 96 instructions), or a loop around one
call (at most 12), may be copied.

Pruning follows each wave. The roots are every function definition that
is not discardable and every symbol named in global data; definitions
reachable from them through any symbol operand stay, and the other
discardable function definitions are removed.

### Argument Propagation (`-O2`, `-O3`)

After the waves, an internal function that is called, has parameters, no
reference other than direct calls (no address, data reference, lifecycle
role or object root), no recursion and fixed arity takes a parameter's
value when every call passes the same literal or symbol for it, then runs
the pipeline again. Parameters its body does not
read leave the signature and every call, whose callers run the pipeline
again; the boundary is internal, so no other unit sees the change. A call
with a stated signature or another argument count disqualifies the
function. Pruning follows.

### Final Pass (`-O2`, `-O3`)

After argument propagation every function gets these rewrites; if any of
them changed it, branch compares are computed again beside their branches:

- `i128` values become pairs of `i64` halves for logic, addition,
  subtraction, shifts, extensions, truncations, compares, selects, phis and
  whole loads and stores; other uses reassemble the value.
- A branch to a diamond or triangle whose arms hold at most two
  speculatable integer instructions each (no load, division or floating
  point) becomes `select`s, at most two per merge, for at most three rounds.
- Loads move out of loops where nothing but `store`, `copyobj` and
  `zeroinit` may write memory (no writing call, atomic operation,
  `va_start`, `va_arg` or EH instruction) and no load is volatile. A load
  moves to the preheader when no write in the loop may overlap it and it
  either runs on every entry (in the header, before any call) or reads a
  slot, a global, or a parameter's `object_bytes` extent; a loop moves at
  most 10 loads less the values already live through it (its header phis
  and the outside values it uses). Invariant integer arithmetic, one of
  whose operands is used nowhere else, moves too.
- A fill loop of a run-time value whose bytes all repeat calls `memset`
  for runs of at least 128 bytes.
- Narrow loop counters that cannot pass their bound are marked `nowrap`.

### What `-O3` Adds

Full unrolling, at most 16 loops per run of the pass:

- A loop with a straight-line body (a chain of jump-ended blocks), one
  preheader, one latch and its exit test in the header, whose trip count
  is known without wrapping and at most 4, and whose body times the trip
  count plus one is at most 64 instructions, becomes one block.
- An innermost loop with a branching body, an integer induction and a
  literal start, step and bound giving 1 to 16 passes is copied pass by
  pass: up to 16 passes within 256 cloned instructions when it reads
  constant data or branches on values computed from the induction, or
  calls nothing and touches no frame slot; otherwise up to 8 passes within
  160 when some path through the body back to the header calls nothing.
- A loop containing an EH instruction, `stack_alloc` or `va_start`, or one
  that exits into a handler block, is not unrolled.

Value numbering across diamonds: when a diamond's merge heads a second
diamond on the same condition, and none of the arms or the first merge
writes memory, a phi of the second whose arm inputs compute what a phi of
the first takes from the same sides (compared to depth 4) becomes that phi.
At most 1,024 phi pairs are compared per pair of diamonds.

### Safety Rules

These are the facts that stop a transformation in the course solution:

- Volatile accesses are never removed, reused, merged, hoisted or
  promoted; a slot with one stays in memory.
- A call whose boundary may write memory, an atomic operation, `va_start`
  and `va_arg` are barriers for every memory fact except those of private
  slots. Only calls that are `readnone` or `readonly`, `unwind=no` and not
  `noreturn` are removed when unused.
- A slot whose address escapes (stored as a value, passed to a call, used
  as a phi input or behind a variable index) is treated as reachable
  through any pointer and is neither promoted nor split (splitting does
  accept a call's indirect-result argument).
- Handler edges leave from the opener's block, so dominance alone does not
  say what a handler has run. Value reuse in code a handler reaches never
  takes a value computed after the first opener of its block, and slot
  promotion keeps a slot in memory when the value a handler's entry would
  see might differ from the slot's value at the throw point (for example,
  one stored inside an active region and live at a handler's entry).
  Memory forwarding starts every handler block empty, no phi is placed in
  a handler block, and handler blocks are never bypassed or merged.
- `replaceable=yes` and `no_inline=yes` callees stay calls; a replaceable
  global is never read as a constant; only discardable definitions are
  pruned.
- A moved instruction loses facts that may not hold where it now runs: a
  load placed where it did not always run loses `typed`, and arithmetic
  moved out of a loop loses `nowrap` and `exact`.
- A scalar temporary passed to a `by_address` parameter stands for storage
  holding its value. Before any pass at `-O1` and above it becomes that
  storage, a slot stored just before the call whose address the call
  passes, so propagation never passes the value itself as the pointer.

### Design Notes

Give function definitions dense indices, keep call edges in adjacency
arrays, and compute strongly connected components once for recursion and a
callee-first order; compare symbols by typed id, not by spelling. Keep
object roots for address, lifecycle and ABI requirements, not for ordinary
calls, so a definition can disappear when inlining removes its last call.
Since cloned calls only reach functions already visited, one callee-first
traversal per wave needs no whole-program retries. Keep each inlining
policy's budget separate, so exhausting one does not suppress another.

Carry the active-region stack as one interned value per block. Key a memory
location by base, offset and type, and undo facts when the dominator walk
leaves a subtree instead of copying tables per block. Share CFG and
dominator facts among passes until one changes the graph. Serialized LowIR
needs every definition before its uses in layout; when value reuse or block
merging breaks that, reorder the blocks (reverse postorder, or a stable
topological order of definition blocks when that would break an order the
input relied on).

The optimizer's own speed matters once it compiles the compiler. In the
course solution, each function body carries a version that changes only
when a pass changes the body, and a pass reports a change only when it made
one: the report decides whether another round runs, so a pass that reports
work it did not do buys a whole extra round of every pass. A pass that ran
at the current version and found nothing is not run again. Analyses (the
CFG, dominators, block maps, use counts) are kept between passes at the same
version, and a checking mode (`--verify-passes`) validates the program after
every pass and checks that the kept block maps and use counts are what a
fresh computation gives. The per-function containers of the larger
passes live in pools reused from one function to the next. No per-function
pass scans the whole program: calls, callers and symbols are indexed once
per wave. None of this changes the output; together it removed a large
share of the optimizer's own work on the compiler's sources.

### Diagnostic controls

`lowiropt` and `cppgm++` (the compile, link and `--emit-lowir` modes)
accept repeatable `--inline-limit <name>=<positive-integer>` options. The
names are `caller-budget` (default 1,536), `once-cap` (the single-call cap:
512, or 1,024 at `-O2` and above), `once-caller-budget` (1,024, or 2,048)
and `hint-late-cap` (the late wave's cap for a hinted callee, 48). A value
given replaces the default at every level; a zero, malformed or unknown
value is rejected. Only that number changes: recursion, EH, signature,
`no_inline` and growth checks still apply, and `-O0` ignores the options.

`lowiropt --stats` (and `cppgm++ --emit-lowir -O1` or higher with
`--stats`) writes one `pa32_opt_stats` record to standard error with the
fields `functions` (definitions optimized; 0 at `-O0`), `instructions_in`,
`instructions_out`, `blocks_in`, `blocks_out`, `folded`, `propagated`,
`reused`, `dead`, `branches_folded`, `blocks_removed`, `blocks_merged`,
`slots_promoted`, `phis_inserted`, `calls_inlined`, `inlined_instructions`,
`functions_pruned`, `loads_reused`, `stores_removed`, `loops_removed`,
`loops_unrolled`, `specialized` and `optimize_ms`; `lowiropt` appends
`parse_ms`, `validate_ms`, `write_ms` and `peak_rss_kb`. The counts are
observational only.
