# PA35 release measurements

`make -C pa35 ref-test` runs the course and student builds, enforces release
margins and generates `pa35/calibration.tsv` and `calibration-course.tsv`.
These are dated measurements tied to their recorded source, machine and
compiler configuration; the acceptance limits are defined in `gates.txt`.

The existing snapshots predate the 0.50 compile-time limits and matching
optimized speed levels. Their O3 compile-time and memory ratios used host
O2. Current speed measurements use O3 on both sides. Regenerate the speed
group when updating a run; a fresh speed measurement is not a regeneration
of the full release calibration. Preserve the historical snapshots' original
values and provenance until a complete release check replaces them.
