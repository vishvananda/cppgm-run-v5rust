# Approaching machine optimization

Start with the MIR and native backend you built in PA24. The
[assignment](README.md) defines the required behavior and code-quality bounds;
these suggestions describe one route to them, and what the course solution
does.

First establish which values and physical registers each instruction reads,
writes or clobbers. Include call arguments, fixed-register operations, both
parts of indexed addresses, and values live across control-flow and exception
edges. Incorrect use information makes even a redundant-copy removal unsafe.

For O1, small local rewrites are useful: remove identity copies, forward a
value into its next use, fold a cheap address or immediate, and eliminate a
jump to the next block. Stop propagation at a conflicting definition or
clobber. Preserve volatile effects, exception boundaries and debug locations
when replacing instructions.

For O2, compute liveness across blocks and calls, and give registers to the
values that live across blocks. The bounds can be met by a small allocator,
but PA34 runs this backend on the whole compiler, where functions are large,
most have cleanups and calls sit inside loops. These choices are the ones
that paid there in the course solution:

- Weigh each use by an estimate of how often its block runs: a loop runs
  several times per entry, and a path that only throws or ends in a call that
  does not return weighs nothing. A frame home's store and reload cost more
  time than their instruction count suggests.
- Keep a value in one register over its whole life where possible. When that
  fails, split it: keep the register outside a loop it crosses without a use,
  or outside just the blocks that clobber or crowd it, rather than giving the
  value a frame home everywhere.
- Let a phi's sources and result share a register when their ranges do not
  overlap, so its edge transfers move nothing. A transfer on a conditional
  edge is executed in that edge's block, not at the end of the predecessor.
  Modeling it there is what lets the web share.
- Give a value a callee-saved register only when its weight pays for the
  save and restore on every entry to the function. Rematerialize constants
  and frame addresses instead of keeping them live.
- Treat a landing pad as a successor of every call that can unwind into it,
  so values in functions with cleanups can stay in registers where the
  exception model allows.
- Lay out the frame last: reuse slots whose lifetimes do not overlap, put the
  most accessed objects nearest the frame register, and give each return its
  own epilogue.
- Chain blocks along their likely edges, put blocks that only throw or exit
  last, and rotate a loop whose test is at its top so the test also ends the
  body.

When a change does less than expected, look for a constraint stricter than
emission needs before tuning a heuristic. A phi transfer modeled at the
predecessor instead of on its edge is one such constraint, and removing it
is worth more than any threshold. Raising thresholds and caps usually nets
little.

The course solution meets the bounds with a liveness-based allocator of this
kind for values that cross blocks. It is computed before instruction
selection. Values used inside one block get registers as they are selected,
and the selector keeps two general and two vector registers back as scratch.
Under its course-model runtime, a function with exception regions keeps its
cross-block values in memory, because that runtime's handler records restore
callee-saved registers to their values at the push. Those are limits of one
design, not advice. Selecting into virtual registers and assigning them
afterwards, with a coloring or linear-scan allocator, meets the same bounds,
and later work has found it easier to extend.

Keep each improvement small enough to validate with an existing fixture.
Compare your own O0 and optimized dumps to understand the redundant work, then
read the failing expectation to see the required bound. Reference dumps are
examples of one successful answer. They are not pass recipes.

For performance investigation beyond the assignment, use programs that run
long enough to measure and depend on runtime inputs. Keep their results equal,
build outside the timed region, and compare immutable executable versions in
wall-time ABBA order. Repeat A/A runs to estimate noise before interpreting a
small difference. Instruction counts and simulated cache and branch events
are deterministic and resolve smaller differences than time does. Such
measurements help choose profitable changes; the assignment's completion
checks remain deterministic.
