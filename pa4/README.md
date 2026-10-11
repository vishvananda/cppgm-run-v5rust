# CPPGM Programming Assignment 4: Complete preprocessing (`preproc`)

Adapted and combined from the original CPPGM preprocessing assignments.
See [NOTICE](../NOTICE) for attribution.

Build a preprocessor using the tokenizer, post-token converter, and
preprocessing-expression evaluator from PA1–PA3. It performs translation
phases 1 through 6 and the tokenization part of phase 7 under the course
rules below. Its reusable token pipeline becomes the parser's input.

Implement one program, `preproc`. Macro replacement and directive processing
are groups within this assignment, with the same file interface throughout.

## Input and output

```sh
preproc -o <outfile> <source1> [<source2> ...]
```

Read each UTF-8 source relative to the working directory. At least one source
is required. Reset macro definitions, conditional state, and pragma-once
state between primary sources. Included files share the state of their
including translation unit.

Create or replace the output file with:

```text
preproc <number-of-primary-sources>
sof <source1>
<PA2 posttoken records>
eof
sof <source2>
<PA2 posttoken records>
eof
```

The `sof` spelling is the command-line source path. There is one `eof` for
each primary source; included files do not introduce `sof` or `eof` records.
PA2 defines token spelling, literal data, and string-literal concatenation.
This text is a test observation. Share structured tokens with later stages;
you do not need to turn the dump into an internal transport format.

For example, if `first.cpp` contains `#define F(x) x x` followed by `F(3)`,
and `second.cpp` contains `F`, then:

```sh
preproc -o /tmp/tokens first.cpp second.cpp
```

produces:

```text
preproc 2
sof first.cpp
literal 3 int 03000000
literal 3 int 03000000
eof
sof second.cpp
identifier F
eof
```

Return `EXIT_SUCCESS` on success. Return `EXIT_FAILURE` on a preprocessing
error, an active `#error`, an erroneous controlling expression, a missing
include, or an invalid phase-7 token. Output-file contents after failure
are unspecified. Standard output and standard error are ignored. As in
PA1, unsupported inputs outside the documented course contract do not create
additional requirements. Arguments beginning with `-` other than the initial
`-o` are outside this interface.

## Implementation order

1. Implement [macro replacement](macros.md): definitions, arguments,
   stringizing, token pasting, rescanning, and recursion suppression.
2. Implement conditional inclusion using the PA3 evaluator, including nested
   inactive groups and macro-expanded controlling expressions.
3. Add [file inclusion and directives](directives.md): include search, source
   locations, line control, predefined macros, pragmas, and `_Pragma`.
4. Check integration: macro definitions from headers, locations through
   expansions, pragma-once file identity, and translation-unit state reset.

The macro group needs only ordinary source files containing macro directives
and text. You can run it before supporting includes or conditionals. Preserve
the course-defined recursion rules in `macros.md`; a host preprocessor is not
an exact oracle for those rules. The directives handout specifies the default
include search and predefined values. Hosted-system-header compatibility is
later work: PA29 adds the GNU and Clang extensions that the host's headers
use, such as `__COUNTER__`, the `__has_*` probes and the GNU comma paste
`, ## __VA_ARGS__`.

## Starter and tests

Continue in the shared `dev/` tree. The student export installs
`dev/preproc.cpp` from the preprocessor scaffold, including a host file-identity
helper for `#pragma once`. Reuse your earlier token and expression machinery.
There is no additional starter-kit checkout.

Run these commands from this assignment directory:

```sh
make check TEST='tests/macros/*.t'
make check TEST='tests/directives/200-*.t'
make check TEST='tests/directives/400-*.t tests/directives/800-*.t'
make test
make run INPUT='tests/directives/400-multiple-source-files.t*'
```

`make check` runs selected cases and compares their outputs. `make test` runs
both groups. The wrappers arrange the working directory needed for relative
includes. For a case named `name.t`, any numbered companions such as `name.t2`
are additional primary source files for that invocation, in lexical order.
References compare the output file and exit status exactly; no fixture
prints the values of `__DATE__`, `__TIME__` or `__CPPGM_AUTHOR__`, and the
comparator does not normalize them. Diagnostic text is not graded.

Use `../dev/preproc-ref` to investigate a test and the root testing guide for
reference policy. You may run the tool directly with temporary files while
developing; no separate stdin implementation or macro-only executable is
required. Finish with the cumulative through-PA4 report from the root.

## Later scale (optional)

The macro and directive guides describe the clearest way to get every rule
right, and it is a fine way to finish PA4. Later assignments preprocess the
host's standard library headers for every translation unit, and PA34
compiles the compiler itself: hundreds of thousands of tokens per compile,
most of them from macros, include guards and groups that are skipped. At
that scale the representation decides the cost, not the rules. One representation
that scales, and the one the course solution uses:

- Find a macro through the identifier's interned entry (a field that says
  whether, and as what, the name is defined), not by probing a table keyed by
  spelling on every rescan.
- Keep each token's set of unavailable macro names as a small interned id,
  so that a produced token copies one number instead of a set of names.
- Expand by pulling tokens from a stack of active expansions, with arguments
  kept as ranges of the invocation's tokens. Memory then follows the nesting
  depth, not the length of the output, and nothing rewrites a long token
  sequence in place.
- Record leading whitespace and the start of a line as flags on the next
  token instead of keeping whitespace and new-line tokens.
- Give each file one range in a single location space, so a token's position
  is one number; work out a file and line only when a diagnostic, `__LINE__`
  or a debug location asks.
- Remember a header whose whole text sits inside one `#ifndef` guard, and
  skip it on a later inclusion while the guard macro stays defined. Lex an
  inactive group only far enough to find its directives, without interning
  its identifiers or looking up macros.

None of this changes PA4's output, so it can come later, but it is easier
to leave room for it now. Any design that meets the output here and the
speed gates later is acceptable.

## What carries forward

Keep token kind, source spelling, decoded literal data, and source location
available to the parser. Keep preprocessing state local to a translation unit
and expose a reusable preprocessing entry point. The next lesson builds a
structured AST over that token stream; preprocessing remains a completed
stage and its tests remain in the cumulative report.

The detailed macro and directive explanations retain material from the
CPPGM Foundation's 2013 handouts.
