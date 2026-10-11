# Phase 6: the handouts against `main`

`git diff main -- 'pa*/README.md'` after Phase 5: 26 handouts differ, 2,651
lines added and 114 removed.  Every difference is one of three kinds, and
none removes or contradicts an instruction a student on `main` was given.

| kind | handouts | what changed |
| --- | --- | --- |
| lane wording | PA3, PA4, PA10, PA11, PA12, PA13, PA39 | the sentence about an optional `course/paN` suite is gone; `make test` runs the one suite under `paN/tests/`; PA39's workflow says where a reducer goes |
| contract text added | PA11, PA13, PA15, PA16, PA17, PA18, PA19, PA20, PA21, PA22, PA23, PA25, PA26, PA28, PA31, PA32, PA33, PA34 | rules the implementation already followed are now stated: rejected declaration forms (PA11); LowIR metadata spelled by omission, boolean presence flags, the `role` family (PA13, 103 lines); compact symbol identity and what the comparison absorbs and enforces (PA15); `__builtin_unreachable`, read-only string storage, `make test-seams` (PA16); `object_bytes` extents and `alias=noalias` on copy and move constructor parameters (PA17); nondependent template bases (PA19); dependent qualified types and template-name hiding (PA22); LSDA call-site coalescing, one terminate action, one resume route (PA31); the GNU and Clang concessions PA34 owns and the three host cells |
| specification restructured | PA29, PA37, PA38 | "How PAN Is Specified" separates the contract, the quality bar and one design (the course solution's); "Optimization Levels" became "One Design: The Course Solution's Optimization Levels" plus "Quality Bar" and "Design Notes"; the regression lane is named as the course solution's memory, outside the contract |

## The three restructured handouts

PA29 (+403), PA37 (+962) and PA38 (+513) are the large divergences.  They
came from the backend student review (`doc/backend-student-review.md`,
`docs/implemented/v3/PLAN-BACKEND-STUDENT-REVIEW.md`), whose question was
exactly the one this phase asks: can a student who reads only the handout
and the seam header build a different design that the suite accepts?  The
review's answer was a newcomer trial: a linear-scan allocator written from
the PA38 handout alone passes the course suite and the behaviour floor
(`make test-variants` runs it with the other designs).  So the added text is
what a student needs, and the separation of contract from design is what
keeps the course solution's shape out of the grade.

What a reader should still check by hand, section by section:

- PA29 "Quality Bar" and "Testing": the size envelopes (`x.ref.expect`) are
  the only place a fixture constrains instruction selection; the wording
  must not read as "match the reference MIR".
- PA37 "Validation Modes": repeats the two lists from `pa13/lowir.md`; if
  the LowIR comparison changes, both places change.
- PA38 "One Design": the placement decisions are labelled as one design
  throughout; any imperative sentence in that section ("must") is a
  contract sentence that belongs in "Quality Bar".

## Test-facing sentences

The only sentences that name test locations are the `make test` paragraphs
and the lane lists (PA29, PA37, PA38); all say `tests/...` and, where the
assignment has one, `tests/regression/...` and `tests/controls/...`.  No
handout names `cppgm.tests` or a course lane.

## Verdict

Nothing in the student-facing instructions moved away from what `main`
told a student to build; the additions state contracts the references
already enforced, and the three restructured handouts were validated by a
newcomer building against them.  Phase 8 (the assignment combination plan)
is where handouts will change shape deliberately.
