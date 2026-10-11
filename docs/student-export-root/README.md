# cppgm assignments

Build a C++11 compiler in 35 cumulative assignments, from preprocessing to
native code, optimization and self-hosting. Later handouts identify the
additional language and hosted-library extensions they require. Start with
[PA1](pa1/README.md) and extend the same implementation in `dev/` and
`dev/src/` throughout.

This student repository contains scaffolds, handouts, tests, references and
wrappers that download pinned reference tools. The complete course solution
is maintained in [cppgm-extended](https://github.com/vishvananda/cppgm-extended).

## Environment and workflow

Use Linux x86-64 with a C++11 compiler (normally `g++`), GNU make, Bash, Perl,
Python 3, `curl` or `wget`, and standard archive/checksum tools. Later native
and debug checks need binutils and a debugger. Generated programs execute
locally; use an isolated environment for autonomous agents.

For assignment N, read `paN/README.md`, then:

```sh
make build
make test-paN
make test-report-through-paN
```

Replace N with the assignment number. A clean through-milestone report is the
exit criterion for PA1–PA33. [PA34](pa34/README.md) runs the self-host ladder
and ends with `make inception`, comparing the self and inception generations.
[PA35](pa35/README.md) makes the compiler fast and ends with
`make -C pa35 test`, its own gates on generated code and compile speed.
Later milestones extend your existing compiler; do not reset to the initial
scaffolds.

Reference tools download when needed. `make reference-binaries` fetches them
ahead of time. Personal test inputs and harnesses can live in `student.tests/`.

See [PROJECT_LAYOUT.md](PROJECT_LAYOUT.md) for the assignment map,
[TESTING_AND_REFERENCES.md](TESTING_AND_REFERENCES.md) for test and reference
policy, and [AGENTS.md](AGENTS.md) for coding-agent instructions. The C++11
reference text is `doc/n3485.txt`.

## Origins and license

This is an independently maintained adaptation of the original C++ Grandmaster
Certification course. The cppgm-extended authors substantially edited,
combined, renumbered and extended the inherited handouts, tests and starter
code. Current PA numbers and reference tools belong to this 35-assignment
version of the course.

Copyright 2026 The cppgm-extended Authors. Project-maintained code and materials
use the Apache License 2.0; inherited material retains its original notices.
See [LICENSE](LICENSE), [NOTICE](NOTICE) and [AUTHORS](AUTHORS).
