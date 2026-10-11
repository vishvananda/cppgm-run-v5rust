## CPPGM Programming Assignment 6 (`cppgm++ --emit-types`)

### Overview

Extend `cppgm++` with the PA6 type/scope dump mode:

```sh
cppgm++ --emit-types -o <outfile> <srcfile1> [<srcfile2> ...]
```

The program reads one or more C++ source files, runs translation phases 1
through 7, parses them using the PA5 syntax boundary, and writes a
deterministic description of the first semantic layer: scopes, declarations,
bindings, and canonical types.

PA6 builds on PA5. The `--emit-ast` mode remains required, and PA6 adds
`--emit-types`.

### Prerequisites

Complete PA5. Reuse the PA1–PA4 preprocessing pipeline and the PA5 AST.
This is the first semantic assignment: build scopes, declarations, entity
identity, lookup, and types directly on that AST. Read
[scopes-and-types.md](scopes-and-types.md) for the implementation order and
type notation.

The representation you create here is extended by PA7's expression analysis
and later lowering. Initialization bytes, full constant evaluation, and
cross-translation-unit linking remain at their existing later stages.

### Starter Kit

The PA6 starter kit contains:

- `README.md`, this assignment handout
- `Makefile`, which builds `cppgm++` and runs the PA6 tests
- `cppgm++.cpp`, a link to the editable `dev/cppgm++.cpp` entry point
- the grammar for this assignment called `pa6.gram`
- an HTML grammar explorer of `pa6.gram` in the sub-directory `grammar/`
- `scripts/run_all_tests.pl` and `scripts/compare_results.pl`
- `tests/spec/`, clause-anchored scope/type tests
- `tests/general/`, broader scope/type tests
- checked-in `.ref` and `.ref.exit_status` files used as the oracle

Your main editable file is `dev/cppgm++.cpp`. You may add or change other
implementation files under `dev/` as needed. Preserve the supplied test inputs,
harness scripts and grammar. Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

The starter `dev/cppgm++.cpp` is the same long-lived `cppgm++` dispatcher used
from PA5 onward. For PA6, extend it so `--emit-types` runs your scope/type
analysis and dump path.

The checked-in reference files under `tests/` are the grading oracle. The
`cppgm++-ref` wrapper is available for investigation under the root reference
policy; tests run your implementation.

### Build And Test Commands

From the `pa6/` directory:

```sh
make
make check TEST='tests/spec/100-*.t tests/general/100-*.t'
make check TEST='tests/general/200-*declarator*.t tests/general/200-*array*.t'
make check TEST='tests/general/200-*using*.t tests/general/200-*namespace*.t'
make test
```

`make` builds `cppgm++`. `make test` runs the local PA6 suite.

### Required Driver Surface

Previously required:

- `--emit-ast`
- `-o <outfile>`

New in PA6:

- `--emit-types`

No new compile or link driver flags are introduced in PA6. Behavior is
undefined unless the command line has this form:

```sh
cppgm++ --emit-types -o <outfile> <srcfile1> [<srcfile2> ...]
```

### Input Contract

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa6.gram`. The grammar defines accepted syntax only;
the PA6 scope/type requirements are defined by the Required Features and Out Of
Scope sections below.

Passing PA5 syntax is necessary but not sufficient for PA6: a program may
parse successfully and still require declaration, type, or constant-expression
behavior that this assignment does not define.

The parser guide explains the lexical hints used in isolated syntax tests.
Semantic lookup must resolve actual declarations and shadowing; a type-like
spelling is not a substitute for an existing type binding in this assignment.

Behavior is undefined for input that:

- does not match the PA6 grammar
- requires PA6 semantic features outside the assignment boundary below
- is ill formed in a way PA6 is not required to diagnose

If this README and `pa6.gram` disagree about accepted source syntax, use
`pa6.gram`. If they disagree about the PA6 semantic slice, use this README.

### Output Format

On success, `cppgm++` writes the PA6 semantic dump to `<outfile>`.

The first line is:

```text
<n> translation units
```

where `<n>` is the number of source files on the command line.

For each translation unit, in command-line order, the output contains:

```text
start translation unit <k>
...
end translation unit
```

where `<k>` is the 1-based translation-unit index.

Between those wrapper lines, write a deterministic semantic dump rooted at:

```text
translation-unit
```

The body of the dump is a scope tree. Scope lines include:

```text
scope namespace <name>
scope template-parameters
scope class <name>
scope enum <name>
scope function <name>
scope block
```

Bindings inside a scope are written one per line, using forms such as:

```text
type <name> <type>
type-alias <name> <type>
enumerator <name> <type> <value>
function <name> <type>
variable <name> <type>
parameter <name> <type>
```

`<type>` uses the recursive spelling defined in
[scopes-and-types.md](scopes-and-types.md). Function declaration lines preserve
the declared parameter types, including top-level cv qualifiers and array or
function forms. Canonical signatures separately apply parameter adjustment;
PA7's call tests observe that identity. The source view is deliberate and
does not require using unadjusted parameter types for function matching.

A using declaration that introduces a type name emits the binding the name
already has, in the current scope: `type` for a class or enumeration and
`type-alias` for a typedef name. Supported value-name using
declarations are emitted through the resulting `function`, `variable`,
`parameter`, or `enumerator` binding rather than through a separate
`using-declaration` line.

Namespace aliases and using directives affect lookup. They have no output
line of their own.

Unnamed types have generated names. An unnamed class or enumeration that a
typedef declares takes the typedef name (7.1.3p9) and is dumped under it,
with both its `type` binding and the `type-alias` (`type C class C`,
`type-alias C class C`). Otherwise an unnamed enumeration is
`__anonymous_enum<N>`, N counting from 1 in order of definition within the
translation unit, and an anonymous union is
`__anonymous_union_type__<k>_<e>`, where k and e are positions in the
translation unit's token sequence after preprocessing, counted from 0: k is
the `union` keyword's, and e is the token's after the closing brace, or the
token's after the `;` when the declaration begins with the keyword itself
(`union { ... };`; `static union { ... };` begins with `static`). PA7 shows
the union's unnamed object as `__anonymous_union_storage__<k>_<e>`.

Standard output and standard error are ignored by the automated PA6 tests.

### Error Handling

If preprocessing, tokenization, parsing, or PA6 semantic analysis fails,
`cppgm++` must exit with `EXIT_FAILURE`.

The contents of `<outfile>` are unspecified on failure. For failing tests, the
harness compares only the named exit status, not diagnostic text and not the
output file.

### Required Features

PA6 must support:

- namespace, template-parameter, class, enum, function, and block scopes
- named namespace reopening
- named class declarations and forward declarations, including compatible
  `struct` / `class` redeclarations of the same non-union class
- visibility of later-declared member types inside member function bodies,
  which are complete-class contexts (N3485 3.3.7/1)
- complete-class lookup and size queries in ordinary member exception
  specifications (C++11 defect resolution CWG 1330); an unused ordinary
  member's specification must still be valid
- named enum declarations and scoped opaque enum declarations
- elaborated class and enum type specifiers in supported declarations;
  elaborated class lookup may find a type hidden by an ordinary-name binding,
  while an elaborated enum specifier must name an existing enumeration
- namespace-scope anonymous class, union, and enum specifiers when the same
  declaration immediately introduces a usable type name
- namespace-scope anonymous-union declarations only when they include the
  required `static` specifier
- template declarations with type and template-template parameter scopes
- simple declarations and function definitions
- `typedef` and alias declarations
- namespace aliases
- using directives
- using declarations that introduce supported type or value names
- storage and function specifiers that do not change the PA6 type model:
  `extern`, `static`, `thread_local`, `inline`, `virtual`, and `constexpr`
- free-function declarators with supported exception specifications
- parameter type adjustment: arrays and functions become pointers, top-level
  cv qualifiers are removed for the signature, and a sole `void` means no
  parameters; retain the source parameter types for the dump
- compatible declaration matching, incomplete-array completion, cv-qualified
  array aliases, and reference collapsing through aliases
- unqualified lookup of visible type aliases, template type parameters,
  namespaces, and supported value names
- qualified lookup through namespaces, class scopes, and scoped enum scopes for
  the supported declaration forms; a nested-name-specifier naming an unscoped
  enumeration also qualifies its enumerators, which live in the enclosing scope
  (7.2p11)
- fundamental, class, enum, cv-qualified, pointer, reference, array, and
  function types
- array bounds formed from supported integral literals (including character
  literals), `sizeof(type-id)`, `alignof(type-id)`, and the documented integral
  constant-expression subset; a bound must convert to a positive size
- semantic disambiguation of `sizeof(T)` when lookup determines that `T` names a
  type
- enumerator values with the supported simple integral constant-evaluation
  subset, including short-circuit `&&` and `||` evaluation that does not
  evaluate an unselected operand
- `static_assert` over the supported integral constant-expression subset
- rejection of implicit conversion or comparison between a scoped-enum value
  and an integer, including a scoped enumerator used as the initializer of
  another enumeration's enumerator (7.2p5)
- `constexpr` object declarations treated as `const` objects for PA6 type and
  constant-value purposes
- the supported `decltype(...)` forms: an unparenthesized id-expression
  naming a variable, function, enumerator or static data member (the
  entity's declared type, 7.1.6.2p4), and a parenthesized lvalue such as
  `decltype((x))` (an lvalue reference to its type)
- deterministic scope-tree output

PA6 also rejects the following declaration forms:

- an object declared with type `void`, which is an incomplete type that can
  never be completed; `void` return types and pointers to `void` remain valid
- a reference declared without an initializer, except as a parameter, a
  function return type, a class member, or where `extern` is used explicitly
- a qualified definition of a namespace or class member written outside a
  scope that encloses the member's own scope
- a namespace-definition that names an existing namespace-alias, since an alias
  is another name for a namespace rather than a namespace that can be extended

A reference initialized with a constant expression is itself usable as a
constant expression, so reading through one may supply an array bound.

The PA6 output should preserve enough declaration and type information for PA7
to add expression and call semantics without reparsing the source.

### Out Of Scope

PA6 does not require:

- overload sets or overload resolution
- expression typing
- general `sizeof`, `alignof`, or `decltype` beyond the type-forming cases
  required by this assignment
- template-aware semantic disambiguation of PA5 syntax ambiguities
- non-type template parameter binding, template specialization modeling, or
  template instantiation semantics
- floating-point or pointer constant evaluation
- full constant-expression semantics
- opaque unscoped enum declarations such as `enum E;`
- semantic analysis of statements beyond creating nested block scopes

Inputs that rely on those features have undefined behavior for PA6.

### Testing And Grading Contract

The PA6 harness discovers every `.t` file under the requested test root.
For each test case `x.t`, it runs:

```sh
cppgm++ --emit-types -o x.my x.t
```

and records `x.my.exit_status`.

Comparison rules:

- `x.my.exit_status` must match `x.ref.exit_status`.
- If the reference status is `EXIT_FAILURE`, the test passes after the exit
  status comparison.
- If the reference status is `EXIT_SUCCESS`, `x.my` must match `x.ref` exactly.
- Standard output and standard error are not compared.

The local suite is split by role:

- `tests/spec/` contains small tests tied to specific C++11 scope, declaration,
  lookup, or type-formation clauses. Most begin with an `N3485 focus` comment
  naming the clause.
- `tests/general/` contains broader PA6 scope/type tests, cross-feature
  semantic combinations, and useful intake cases that are not a single-clause
  oracle.

### Design Notes (Non-Normative)

A good PA6 design keeps these pieces separate:

- AST traversal from PA5
- declaration collection
- scope ownership and parent/child relationships
- lookup
- declarator-derived type construction
- deterministic printing

Keep the semantic graph and analyzer reusable by later modes. The PA6 dump
can be a source-facing view over that shared graph: distinguish source
declarations from implicit implementation facts with typed metadata, and keep
lookup/type identity separate from presentation. Avoid rebuilding semantic
names by parsing rendered strings.

Keep native image construction and later initialization lowering outside the PA6
core. Those ideas become useful again later, but this assignment should leave
behind a reusable scope/type model.

Keep names structured and interned. Lookup runs for every name in every
template instantiation; a qualified name kept as its components' interned
identities is compared and hashed cheaply, while a name rebuilt as text and
hashed on each lookup repeats that work millions of times on real headers.
The same holds for types: give each canonical type a compact identity so
that type equality is an identity comparison.

Keep the cost of one lookup proportional to the scopes it actually
searches. A table per scope, or one table keyed by scope and interned name,
answers each step in one probe. A lookup that allocates or clears a
structure sized to every scope of the translation unit, or walks every
binding of an enclosing namespace, makes compile time grow with the square
of the input: each new class, function or namespace adds a scope that every
later lookup pays for. The same goes for constant values (enumerators,
array bounds, `static_assert` operands): keep them as small values, not as
records that own strings or maps and are copied with every binding and
expression.

### Multi-file test groups

For a root `name.t`, companions such as `name.t2` are additional primary
source files in the same invocation, in lexical order. They remain separate
translation units; the harness counts the group once. Headers and other
support files stay with
the owning case and are not concatenated into one source file.
