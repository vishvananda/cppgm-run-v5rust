# CPPGM Programming Assignment 34: Self-hosting (inception)

Complete PA33, then use your compiler to build itself. PA34 adds no language
feature or output format: it checks that the implementation you have built
can compile its own sources correctly and reproducibly.

## Completion criterion

From the repository root:

```sh
make inception CXX=g++ CPPGM_HOST_CXX=g++
```

Keep the same host compiler and standard-library flags throughout the build.
The target performs three builds:

| Generation | Built by | Output |
| --- | --- | --- |
| Seed | The host C++ compiler | `dev/cppgm++` |
| Self | The seed compiler | `pa34/cppgm++-self` |
| Inception | The self-built compiler | `pa34/cppgm++-inception` |

The self and inception binaries must match byte for byte. The seed is built
by a different compiler and is not the binary comparison target. A successful
run prints `MATCH cppgm++` and exits successfully.

The host compiler still links the generated object files and supplies the
hosted configuration. Your compiler must perform the C++ compilation and
native object generation itself. Keep LowIR and MIR as the interfaces defined
by the earlier assignments.

## Work through the ladder

The ladder helps locate failures before the final comparison. Run these
commands from the repository root:

```sh
make build CXX=g++ CPPGM_HOST_CXX=g++
make -C pa34 test-through-pa4 CPPGM_HOST_CXX=g++
make -C pa34 test-through-pa5 CPPGM_HOST_CXX=g++
make -C pa34 test-through-pa8 CPPGM_HOST_CXX=g++
make -C pa34 test-through-pa33 CPPGM_HOST_CXX=g++
make inception CXX=g++ CPPGM_HOST_CXX=g++
```

Within `pa34/`, `CXX` defaults to `../dev/cppgm++`. Keep
`CPPGM_HOST_CXX` set to a real host compiler. Do not pass the host compiler
as PA34's `CXX`, since that would bypass self-compilation.

`test-through-paN` builds the needed self-hosted tools and runs PA1 through
PA N with them, in assignment order. `test-paN` reruns just one stage. These
are the existing assignment contracts with a different compiler binary; the
expected outputs do not change.

| Assignment tests | Self-built tool |
| --- | --- |
| PA1 | `pptoken-self` |
| PA2 | `posttoken-self` |
| PA3 | `ppexpr-self` |
| PA4 | `preproc-self` |
| PA5–PA7, PA10–PA23, PA25–PA31 | `cppgm++-self` |
| PA8 | `lowir-self` |
| PA9 | `abimangle-self` |
| PA24, PA33 | `lowir2native-self` |
| PA32 | `lowiropt-self`, with `cppgm++-self` for driver checks |

PA1–PA4 rebuild the smaller preprocessing tools. PA5 must build the complete
`cppgm++` implementation even though its tests use only `--emit-ast`; later
source-language rungs reuse that binary in the appropriate mode. The ladder
is a sequence of checks, not a sequence of partial implementations of your
compiler.

PA8 still uses the supplied native backend to execute the LowIR constructed
by `lowir-self`; PA10–PA23 use it for their self-checking controls on C++
lowering. PA24 and PA33 test your self-built native backend.

The source sets come from `dev/frontend_source_sets.mk`, including the files
you added during earlier assignments. Keep those lists current for every tool
that uses a shared module.

## Focused builds and checks

From the repository root:

```sh
make -C pa34 cppgm++-self CPPGM_HOST_CXX=g++
make -C pa34 test-pa12 CPPGM_HOST_CXX=g++
make -C pa34 compare-cppgm++-inception CPPGM_HOST_CXX=g++
```

The last command performs the final comparison without rebuilding the seed
first. After changing compiler sources, run root `make build` before these
focused commands so the seed includes your fix.

Use `INCEPTION_OBJ_ROOT_BASE=../obj/pa34-experiment` on PA34 commands to keep
an experiment's objects separate. Canonical objects depend on the compiler
that produced them; changing that compiler correctly invalidates those objects.
Do not combine object generations for the final comparison.

For a quick single-source diagnosis:

```sh
make -C pa34 probe-self-object SOURCE=../dev/cppgm++.cpp CPPGM_HOST_CXX=g++
```

This uses the self-host flags but writes a scratch object under
`obj/pa34/probe/`. It is a debugging aid; finish with the normal ladder and
inception targets.

## Diagnose a failure

1. Find the first failing source file or assignment test.
2. Reduce it to a small input in `student.tests/` and compare the seed and
   self-built compiler on that input.
3. Fix the underlying parser, semantic, lowering, optimizer or native-code
   behavior. Preserve the earlier assignment contracts.
4. Rebuild the seed, rerun the failing rung, then continue the ladder and final
   comparison.

A byte mismatch can come from unstable symbol or instruction order, embedded
paths, timestamps, host configuration or linker behavior. PA34 also compares
corresponding self and inception objects as they are built, so the first
object mismatch can identify the responsible source file.

A self-build slowdown can expose a miscompilation as well as an inefficient
algorithm. Check for repeated semantic work, lost cache results and divergent
control flow before increasing resource limits.

Each self-build source compile has a 900-second timeout; each inception
compile has a 3,600-second timeout. The runner also caps a command at 8 GiB
RSS. Inception compilation uses at most eight jobs by default; lower
`INCEPTION_BUILD_JOBS` if concurrent compiler processes exhaust memory.
The timeout knobs are `INCEPTION_SELFHOST_COMPILE_TIMEOUT_SEC` and
`INCEPTION_INCEPTION_COMPILE_TIMEOUT_SEC`; the memory knob is
`CPPGM_RUN_MAX_RSS_KB`. Use them to diagnose a reduced case, then validate the
complete build under the normal limits.

## Measuring the self-built compiler

Nothing in PA34 is timed, but the self-built compiler is the largest program
your optimizer and backend compile, and the natural one to measure them on.
A few habits make such measurements mean something:

- Freeze a seed and a self build of one source, and keep them unchanged
  while you measure. A rebuilt binary is a different program.
- Link the seed and the self build the same way, with the same `malloc`. An
  allocator difference can move a compiler's time by a few percent and hide
  the difference that comes from generated code.
- Count instructions on both levels of one workload (`-O0` mostly runs the
  front end and instruction selection, `-O3` mostly the optimizer), and use
  paired timing to confirm. When a change touches the optimizer, a self
  build from the new source does different work as well as running
  different code; to see the code alone, build one frozen source with the
  old and the new compiler and compare those two binaries, which do the
  same work.

[PA35](../pa35/README.md) turns these measurements into gates: its scripts
freeze the seed and the self build, link them alike and count both levels.
