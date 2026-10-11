## CPPGM Programming Assignment 7 (`cppgm++ --emit-semantics`)

### Overview

Extend `cppgm++` with the PA7 call-semantics dump mode:

```sh
cppgm++ --emit-semantics -o <outfile> <srcfile1> [<srcfile2> ...]
```

The program reads one or more C++ source files, runs translation phases 1
through 7, parses them using the PA5 syntax boundary, applies the PA6
scope/type model, and writes a deterministic semantic dump for the procedural
expression, statement, conversion, and non-template call subset.

PA7 builds on PA5 and PA6. The earlier `--emit-ast` and `--emit-types` modes
remain required, and PA7 adds `--emit-semantics`.

### Prerequisites

Complete PA6 before starting PA7. You should expect to reuse:

- the PA1-PA4 preprocessing and tokenization pipeline
- the PA5 AST
- PA6 scope formation and lookup
- PA6 declarator-derived type construction
- the canonical type spelling used by the earlier semantic assignments

PA7 is the first call-semantics milestone. It is deliberately limited to the
procedural, non-template, non-class-aware subset. Class-aware calls,
constructors, user-defined conversions, overloaded operators, and template
functions are assigned later.

### Starter Kit

The PA7 starter kit contains:

- `README.md`, this assignment handout
- `Makefile`, which builds `cppgm++` and runs the PA7 tests
- `cppgm++.cpp`, a link to the editable `dev/cppgm++.cpp` entry point
- the grammar for this assignment called `pa7.gram`
- an HTML grammar explorer of `pa7.gram` in the sub-directory `grammar/`
- `scripts/run_all_tests.pl` and `scripts/compare_results.pl`
- `tests/spec/`, clause-anchored call/conversion/control-flow tests
- `tests/general/`, broader call-semantics tests
- checked-in `.ref` and `.ref.exit_status` files used as the oracle

Your main editable file is `dev/cppgm++.cpp`. You may add or change other
implementation files under `dev/` as needed. Preserve the supplied test inputs,
harness scripts and grammar. Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

The starter `dev/cppgm++.cpp` is the same long-lived `cppgm++` dispatcher used
from PA5 onward. For PA7, extend it so `--emit-semantics` runs your resolved
semantic-analysis and dump path.

Use `cppgm++-ref` to inspect example output. Tests run your compiler against the checked-in contract fixtures.

### Build And Test Commands

From the `pa7/` directory:

```sh
make
make test
```

`make` builds `cppgm++`. `make test` runs the local PA7 suite.

### Required Driver Surface

Previously required:

- `--emit-ast`
- `--emit-types`
- `-o <outfile>`

New in PA7:

- `--emit-semantics`

No new compile or link driver flags are introduced in PA7. Behavior is
undefined unless the command line has this form:

```sh
cppgm++ --emit-semantics -o <outfile> <srcfile1> [<srcfile2> ...]
```

### Input Contract

The authoritative source syntax is the shared `cppgm++` source grammar, exposed
for this assignment as `pa7.gram`. The grammar defines accepted syntax only;
the PA7 procedural semantic requirements are defined by the Required Features
and Out Of Scope sections below.

Passing PA5 and PA6 is necessary but not sufficient for PA7: a program may
parse and form declarations successfully while still relying on call or
expression semantics outside this assignment.

Behavior is undefined for input that:

- does not match the PA7 grammar
- requires PA7 semantic features outside the assignment boundary below
- is ill formed in a way PA7 is not required to diagnose

If this README and `pa7.gram` disagree about accepted source syntax, use
`pa7.gram`. If they disagree about the PA7 semantic slice, use this README.

### Output Format

On success, `cppgm++` writes the PA7 semantic dump to `<outfile>`.

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

The dump's lines are the kinds below and no others. `<type>` is PA6's
canonical type spelling; `<vc>` is a value category, `lvalue`, `prvalue`
or `xvalue` (the PA7 tests primarily exercise the first two); `<token>` is
a token's kind and spelling joined by a colon, as in PA5 (`OP_PLUS:+`).

Declarations:

```text
namespace-definition <name>
type-alias <name> <type>
variable <name> <type>
function-declaration <name> <type>
function-definition <name> <type>
parameter <name> <type>
constructor-action <class>::<class>
```

A variable of class type is followed by its `constructor-action`, whose
child is the `call-expression` of the constructor; that call's `callee` is
`<class>::<class>`, a function whose first parameter is the object pointer.

Statements, without payload:

```text
compound-statement
simple-declaration
expression-statement
return-statement
if-statement
condition
condition-declaration
then
else
while-statement
do-statement
for-statement
for-init-statement
iteration
switch-statement
case-statement
default-statement
break-statement
continue-statement
```

Expressions:

```text
id-expression <vc> <type> <name>
literal <vc> <type> <spelling>
call-expression <vc> <type>
callee <name> <type>
unary-expression <vc> <type> <token>
postfix-expression <vc> <type> <token>
binary-expression <vc> <type> <token>
assignment-expression <vc> <type> <token>
member-expression <vc> <type> <token-kind>:<member>
conditional-expression <vc> <type>
subscript-expression <vc> <type>
sizeof-expression <vc> <type>
cast-expression <vc> <type> [<token>]
braced-init-list <vc> <type>
```

`callee` is a `call-expression`'s first child and names the function
overload resolution selected, qualified by its class or namespace (`A::g`);
it carries no value category. A `member-expression` ends with the
operator's kind and the member's name, `OP_DOT:value` or `OP_ARROW:right`;
an anonymous union's member, reached without an operator, ends with the
member's name alone. A `cast-expression` with a payload is a cast keyword
or, for a C-style cast, `OP_LPAREN:`; one without is a functional-notation
cast or a conversion the analysis inserted, such as a derived-to-base
conversion.

Unnamed entities have generated names. An unnamed class declared with a
declarator (`struct { int value; } first;`) is `__local_type<N>`, N
counting from 1 in order of definition within the translation unit; an
anonymous union is `__anonymous_union_type__<k>_<e>` as PA6 defines, and
its unnamed object (9.5p5), through which its members are reached, is the
variable `__anonymous_union_storage__<k>_<e>` of that type; an unnamed
enumeration is `__anonymous_enum<N>`. Only the dump is compared: the
semantic graph in memory may take any shape that produces it.

Namespace aliases, using directives, and using declarations affect lookup, but
they do not necessarily have dedicated output lines. Their effect is visible in
the resolved declarations and expression subtrees.

Standard output and standard error are ignored by the automated PA7 tests.

### Error Handling

If preprocessing, tokenization, parsing, or PA7 semantic analysis fails,
`cppgm++` must exit with `EXIT_FAILURE`.

The contents of `<outfile>` are unspecified on failure. For failing tests, the
harness compares only the named exit status, not diagnostic text and not the
output file.

### Required Features

PA7 must support:

- namespace-scope simple declarations, alias declarations, function
  declarations, and function definitions
- named, inline, and unnamed namespace definitions, namespace aliases, using
  directives, and using declarations, with same-scope namespace/ordinary-name
  conflicts rejected
- type aliases used by the PA7 slice
- fundamental, pointer, reference, array, and function types
- function parameter scopes, nested block scopes, and the separate scopes of
  unbraced selection/iteration substatements
- local simple declarations
- block-scope using declarations and using directives
- supported ordinary anonymous-union local declarations
- unqualified and qualified lookup of namespace-scope non-template functions
- unqualified lookup extended by using directives, using declarations, namespace
  aliases, and unnamed-namespace visibility
- calls through function names, function references, and function pointers
- target-directed resolution of overloaded function names in contexts such as
  function-pointer initialization and function-pointer arguments
- overload resolution using the assignment's limited standard-conversion subset:
  identity, lvalue-to-rvalue, top-level cv stripping for by-value arguments,
  array-to-pointer, function-to-pointer, common integral promotions and
  conversions, pointer-to-bool, `nullptr_t` to pointer, pointer qualification,
  object pointer to cv-qualified `void*`, and the supported lvalue-reference
  bindings
- function redeclaration matching after top-level parameter cv normalization,
  with conflicting return types and duplicate definitions rejected
- recursive pointer-qualification conversion checks, including rejection when
  the intermediate const qualification required by a deep conversion is absent
- copy-initialization for local variables, condition declarations, and returns
  using that same conversion subset
- integer literals, `true`, `false`, and `nullptr`
- id-expressions for parameters, locals, and supported globals
- parenthesized expressions
- unary `+`, `-`, `!`, `~`, `&`, `*`, prefix `++`, and prefix `--`
- postfix `++` and postfix `--`
- built-in arithmetic, bitwise, shift, logical, comparison, equality,
  conditional, comma, assignment, and compound-assignment expressions over the
  supported operand categories
- conditional-expression typing and value-category selection for the supported
  scalar cases, including mixed `bool` lvalue/prvalue operands
- pointer arithmetic and pointer comparisons in the ordinary object-pointer
  cases required by the tests
- built-in subscript expressions on arrays and pointers
- explicit casts over the supported integral, enum, pointer, and `nullptr`
  subset
- `sizeof(expr)` and `sizeof(type-id)`
- compound statements, `if` / `else`, `switch`, `while`, `do`, `for`, `break`,
  and `continue`
- expression conditions and declaration conditions of the form `T x = expr`
- supported integral `constexpr` complete objects, enumerator constants, the
  course-supported `__builtin_constant_p` query over propagated integral
  expressions, and semantic recognition of a zero-argument `__builtin_abort`
  call (without requiring its later control-flow lowering); passing arguments
  to `__builtin_abort` is rejected
- rejection of type, call-arity, and control-flow violations within this
  supported slice, including mismatched indirect-call arity, nonconstant case
  labels, `break` or `continue` outside a permitted statement, `default`
  outside a switch, a value returned from a `void` function, invalid
  scoped-enum conditions, and invalid pointer/integer equality or pointer
  multiplication
- deterministic resolved-expression output

Your semantic model should retain enough information for later assignments to
add class-aware conversion ranking and richer overload resolution without
reparsing the source.

### Out Of Scope

PA7 does not require:

- class-aware call resolution
- member function calls or implicit object parameters
- overloaded operators
- constructor selection
- user-defined conversions
- reference binding beyond the basic cases listed above
- full standard conversion ranking
- template functions or template-aware overload resolution
- floating-point, string, or user-defined literals
- general callable-object semantics beyond plain functions and function
  pointers
- statement forms beyond the supported control-flow subset, including `goto`,
  `throw`, and `try`
- semantic support for classes, enums, templates, or `decltype` beyond what is
  needed by this assignment

Inputs that rely on those features have undefined behavior for PA7.

### Testing And Grading Contract

The PA7 harness discovers every `.t` file under the requested test root.
For each test case `x.t`, it runs:

```sh
cppgm++ --emit-semantics -o x.my x.t
```

and records `x.my.exit_status`.

Comparison rules:

- `x.my.exit_status` must match `x.ref.exit_status`.
- If the reference status is `EXIT_FAILURE`, the test passes after the exit
  status comparison.
- If the reference status is `EXIT_SUCCESS`, `x.my` must match `x.ref` exactly.
- Standard output and standard error are not compared.

The local suite is split by role:

- `tests/spec/` contains small tests tied to specific C++11 calls,
  conversions, initialization, overload-resolution, or control-flow clauses.
  These files begin with an `N3485 focus` comment.
- `tests/general/` contains broader PA7 call-semantics tests,
  cross-feature semantic combinations, and useful intake cases that are not a
  single-clause oracle.

### Design Notes (Non-Normative)

A good PA7 design keeps these pieces separate:

- PA6 scope/type analysis
- expression analysis
- conversion classification
- overload candidate collection
- overload ranking for the limited PA7 subset
- statement-scope construction
- deterministic semantic printing

Treat the PA7 call layer as a base that later class and template assignments
will extend. Avoid hard-coding assumptions that only work before member
functions, constructors, user-defined conversions, or templates are introduced.

Keep compiler-generated identities separate from ordinary source lookup.
Anonymous entities should receive stable typed identities rather than names
that are re-parsed or inserted into the source identifier namespace.

Overload resolution runs for every call, operator and initialization, which
is thousands of times per translation unit once the standard library headers
are in it. Keep an expression's analysis result small: the node, its type's
identity and its value category, never a copy of a subtree. Record the
selected function and each operand's conversion on the operands, as small
records over type identities, so that later assignments lower what PA7
decided instead of classifying the conversion again.

### Multi-file test groups

For a root `name.t`, companions such as `name.t2` are additional primary
source files in the same invocation, in lexical order. They remain separate
translation units; the harness counts the group once. Headers and other
support files stay with
the owning case and are not concatenated into one source file.
