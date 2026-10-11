## CPPGM Programming Assignment 9 (`abimangle`)

### Overview

Write one C++ application called `abimangle`.

`abimangle` takes normalized ABI fact files as input and writes Itanium C++
ABI mangled names. Each input case describes the semantic facts for one ABI
name: the entity being named, owner scopes, type structure, template
parameters, template arguments, dependent expressions, local contexts, ABI
tags, and special ABI-name forms.

The input is not C++ source. This assignment is about ABI name construction
only. It does not require C++ parsing, semantic analysis, LowIR generation,
object emission, linking, or runtime behavior.

### Prerequisites

Complete PA8 before starting this assignment.

You will want to reuse:

- the shared tool structure from the earlier assignments
- the Itanium C++ ABI mangling rules in `../doc/itanium-mangling.txt`

The normalized facts intentionally describe language features that later
assignments have not implemented yet. Those facts are the design vocabulary
for the encoder; the corresponding source-language milestones are not
prerequisites.

The tests assume a POSIX-like shell environment with `make`, `bash`, `perl`,
and a working host C++ compiler for building `abimangle`.

### Starter Kit

The starter kit provides:

- `dev/abimangle.cpp`, populated with command-line handling for `abimangle`
- `pa9/abimangle.cpp`, a wrapper that builds the editable tool source from
  `../dev/abimangle.cpp`
- `pa9/Makefile`
- `pa9/scripts/`, the ABI fact test harness
- `pa9/tests/abi/`, the checked-in ABI fact tests and reference files
- shared support sources and headers under `dev/src/`
- the ABI vocabulary in `dev/src/abi/itanium/abi_mangle.h`: the Itanium code
  tables and the fact-file words as enums

Put code changes in `dev/`, especially `dev/abimangle.cpp` and reusable
helpers under `dev/src/`. Do not edit generated `.my` files. Preserve supplied test inputs.
Corrections to reference outputs follow the
[reference policy](../TESTING_AND_REFERENCES.md).

`abi_mangle.h` gives the vocabulary; the typed ABI model, the fact reader and
the encoder are yours. From PA10 on, the compiler's symbols must equal what
`abimangle` produces for the same facts; how the compiler reaches its encoder
is yours. One encoder under `dev/src/` serving both, with the line-oriented
fact reader kept at the `abimangle` tool boundary, is the direct way to keep
them equal; converting semantic state to fact text and parsing it back is a
slow path to avoid.

The code tables in `abi_mangle.h` cover builtin types, standard substitutions,
terminals and operators. Substitution, the structure of each encoding,
compiler semantic lowering and other implementation logic are your work,
outside the PA9 wrapper and test harness.

Use `abimangle-ref` to inspect example output.
Tests run your implementation against the checked-in contract fixtures.

### Command-Line Contract

Required form:

```sh
abimangle -o <outfile> <abi-facts-file>...
```

`abimangle` shall read all input fact files in command-line order and write one
mangled name for each input case to `<outfile>`.

Each output name is written on its own line:

```text
_ZN2ns1fEiPc
```

If an input file contains multiple cases, the output preserves the case order
from that file before moving to the next input file.

### ABI Fact Files

ABI fact files are line-oriented. The checked-in tests use normalized facts of
the forms described here.

Simple cases can be one line:

```text
function f
function path ns::f
variable ns::g
type ptr:const:int
typeinfo ns::C
vtable ns::C
```

A named or class-template type with one or more ABI tags uses a typed wrapper:

```text
let-type Tagged tagged named:C tag
typeinfo Tagged
```

The tags belong to the type's unqualified ABI name component and therefore
also appear in its typeinfo, typeinfo-name, vtable, and VTT special symbols.

The normalized builtin word `float128` denotes GNU `__float128` and uses the
Itanium builtin type code `g`. This is an ABI fact spelling, not a requirement
to parse `__float128` as C++ source in PA9.

The normalized builtin words `complex-float`, `complex-double`, and
`complex-longdouble` denote GNU complex floating types and use the Itanium type
encodings `Cf`, `Cd`, and `Ce`. These are typed ABI facts: the fact file never
spells a mangled fragment, and the encoder produces the encoding.

Integral `value` facts are interpreted according to their typed value. Signed
negative values use the Itanium `n`-plus-magnitude spelling, including the
minimum value of a signed type. A negative stored value for an unsigned builtin
type denotes that type's modulo bit pattern, so `value uint -1` and
`value ulong -1` are emitted as the maximum values for their respective target
widths rather than as negative ABI literals.

Compact member-pointer types use
`memberptr:<owner>:<member-type>`, where the compact owner is a bare qualified
name or type-definition identifier. Scope separators do not delimit the two
operands, so both `memberptr:ns::C:int` and `memberptr:C:ptr:int` are valid.
Use the multiword `member-pointer <owner> <member-type>` form when the owner
itself needs a constructor prefix such as `named:`; the canonical fact
serializer uses this unambiguous form. A `function-type` fact contains a result
type followed by zero or more parameter types; an empty parameter list is
encoded with the Itanium `v` marker.

Adjacent `const` and `volatile` type wrappers describe one canonical
cv-qualified type. Their source order does not create distinct types or
substitution keys, and the encoder emits the canonical Itanium qualifier order.

Structured cases introduce reusable facts before the final target:

```text
let-type Char template-param 0
let-arg Char_arg type Char
let-type Traits template std::char_traits Char_arg
let-arg Traits_arg type Traits
let-type Alloc template std::allocator Char_arg
let-arg Alloc_arg type Alloc
let-type String template std::__cxx11::basic_string Char_arg Traits_arg Alloc_arg
function path std::getline Char_arg
param ref String
```

Definition forms:

`decltype-id <expr-ref>` preserves an unparenthesized id-expression or class
member access and emits `Dt`; `decltype <expr-ref>` emits `DT`. The dependent
expression form `source-name <identifier>` retains an unqualified name, such as
the callee of `decltype(selected(value))`.

`member <type-ref> <yes|no> <identifier> [<argument-ref>...]` retains a
qualified dependent name, including a member template's argument list. Use
`yes` for a written qualifier level closed by `E`; use `no` for an unresolved
type owner such as `T_`. A qualified call wraps this expression in `call`.

- `let-type <id> ...`: a type fact
- `let-arg <id> ...`: a template-argument fact
- `let-expr <id> ...`: a dependent-expression fact
- `let-context <id> function ...`: a local or lambda context named by a
  function target
- `let-context <id> raw <context-fragment>`: a local or lambda context already
  normalized as an Itanium local-name context fragment
- `let-entity <id> ...`: an entity fact used by entity-valued template
  arguments and dependent expressions

Definition identifiers are file-local binders. Their spelling does not
participate in the ABI name; use a short descriptive identifier and refer to it
consistently. Whether two uses refer to one definition or to separately
defined structural facts can still matter to the case being described. One
identifier may be defined only once in a case, across all `let-*` forms;
redefining it is an invalid fact file rather than an overwrite.

Template-parameter and other ABI indices are nonnegative decimal integers.
Negative or otherwise malformed index spellings are invalid facts and must be
rejected.

Target forms:

- `type ...`
- `function ...` with optional following terminal and `param ...` lines
- `variable ...`
- `typeinfo ...`
- `typeinfo-name ...`
- `vtable ...`
- `vtt ...`
- `construction-vtable ...`
- `tls-wrapper variable ...`
- `thunk ... function ...`
- `virtual-base-thunk ... function ...`

The records that describe a function (terminals, `function-qualifier`,
`result`, `param` and the other component lines) follow only a target that
names a function: `function ...`, or the `function ...` of a thunk. After any
other target they make the fact file invalid. A `function-qualifier` record
names at most one of `lvalue-ref` and `rvalue-ref`.

The thunk target uses `thunk <this-adjust> function ...` when only `this`
needs adjustment and `thunk <this-adjust> <result-adjust> function ...` for a
fixed covariant-result adjustment.  A covariant result reached through a
virtual base uses the typed form
`thunk <this-adjust> virtual-result <fixed-adjust> <vcall-offset> function ...`.
The fixed component is applied after loading the dynamic adjustment from the
returned object's vtable at the supplied vcall-offset slot.  The compiler's
thunk symbols must equal what `abimangle` produces for the same facts; the
text form is the fact file's spelling of the target.

Function operator terminals use semantic names, not raw Itanium terminal
fragments:

```text
function path C::operator
operator-terminal plus
param int

function path operator
operator-terminal literal _digits
param ulonglong

function path C::operator
conversion-terminal int
```

Complex function encodings may also be written as a `function encoding` target
followed by normalized component lines. Template-id components use
`name-template ... <arg-ref>...`; function-template arguments use
`function-template-arg <arg-ref>`, with `function-template-prefix <key>` when
the function-template prefix is substitutable; local entities use
`local-context ...` or `lambda-context ...` followed by the same terminal,
qualifier, result, and parameter lines as ordinary functions. A local entity's
discriminator (the occurrence operand of `local-context`) follows its whole
entity name, after a member terminal: the destructor of a function's third
local class `Local` is `...EN5LocalD1E_1v`, not `...EN5Local_1D1Ev`. A member
template used as a template-template argument is one substitution candidate,
entered after its owner specialization, so a repeated use is that substitution.

Namespace-scope lambda closure types use
`type namespace-lambda <source-name> [namespace-qualifier...]`. Their call
operators may be written either as
`function namespace-lambda <source-name> <terminal> [namespace-qualifier...]`
or, in a `function encoding` case, as
`namespace-lambda-context <source-name> [namespace-qualifier...]` followed by
ordinary terminal, qualifier, result, and parameter lines. The source name is
the ABI source-name component, such as `$_0`.

`operator-terminal <name>` names the C++ operator semantically. Supported names
include `plus`, `minus`, `address-of`, `deref`, `new`, `new-array`,
`delete`, `delete-array`, `multiply`, `divide`, `remainder`, `bit-or`,
`bit-xor`, assignment operators, shifts, comparisons, logical operators,
`increment`, `decrement`, `comma`, `member-pointer`, `arrow`, `call`, and
`index`. For operators whose Itanium terminal depends on unary versus binary
use, the encoder chooses from the parameter count and member/non-member shape;
explicit names such as `unary-plus`, `binary-plus`, `unary-minus`,
`binary-minus`, `bit-and`, and `multiply` may be used when the shape should be
unambiguous.

Inherited constructors use `terminal constructor-inherited-complete <base-type>`
or `terminal constructor-inherited-base <base-type>`. The base type identifies
where the constructor was originally declared, including through multiple
levels of inheritance. Encode its class name, retaining substitutions for
name prefixes and template arguments without adding a complete-type entry.

Literal operators are written as `operator-terminal literal <suffix>`, where
`<suffix>` is the unencoded suffix source name such as `_digits`. Conversion
operators remain separate `conversion-terminal <type>` facts. The conversion
type participates in ordinary substitution ordering and is also the function's
encoded result; a separate `result` record is not emitted for a conversion
function. Local and lambda call-operator contexts continue to use
`operator-call` as a semantic terminal marker, not as an Itanium code.

Thunks, wrappers, typeinfo, and vtable names are described as ABI facts instead
of already-mangled names.

Raw external symbols may be carried with `let-entity <id> symbol <mangled-name>`
when a template argument or dependent expression names an entity that is already
known by ABI symbol rather than by a source-level qualified name.
Namespace-scope variables with internal linkage use
`let-entity <id> internal-variable <qualified-name>`; this keeps the qualified
entity and linkage typed until the encoder inserts the Itanium local-name marker.

Template-template arguments may name either a namespace-scope template with
`let-arg <id> template-entity <qualified-name>` or a member template of an
already-structured owner type with
`let-arg <id> member-template-entity <owner-type> <member-name> <substitution>`.
Type facts may also spell a class-template specialization whose template name
is a template-template parameter using `type template-param-template <index> <arg-ref>...`.
Member type facts use the same structured owner rule, so `type member <owner>
<name>` may be rooted in a dependent template specialization or builtin
transform type such as `__remove_const<T>`.

The fact format is deliberately small, but it is still an ABI entity graph. It
should not become a second C++ parser.

### Required ABI Coverage

The checked-in tests are numbered from simpler names toward more complete ABI
situations:

- `100-*`: basic functions, variables, named types, builtin types, pointers,
  arrays, member pointers, typeinfo, vtables, VTTs, variadic forms, and
  malformed fact files
- `200-*`: ABI tags, local entities, local and namespace-scope lambdas,
  ABI-tagged special type names, member-function qualifiers, operators,
  conversion terminals, TLS wrappers, and thunks
- `300-*`: entity-valued template arguments, template-template arguments,
  standard substitutions, construction vtables, and dependent integral values
- `400-*`: dependent aliases and dependent member/owner types
- `500-*`: dependent expressions, casts, calls, type traits, `sizeof(type)`,
  packs, and substitution of equivalent dependent expressions
- `600-*`: nested owner contexts and standard-library-adjacent inline namespace
  cases

An implementation should handle Itanium substitution ordering, nested names,
local-name contexts, template parameter references, template arguments,
dependent expressions, ABI tags, special names, and every target form covered
by the tests. Ordering remains deterministic when substitutions arise inside
dependent expressions, qualified member-template owners, and local-name
contexts. Multiple ABI tags use canonical order, and local entities support the
same special-member terminals as their nonlocal counterparts. A local lambda
used as a function-template argument retains the enclosing function as its
local-name context; it is not represented as a named class under its call
operator.

Reference:

- Local copy of Itanium C++ ABI, Chapter 5.1 "External Names (a.k.a.
  Mangling)": [`../doc/itanium-mangling.txt`](../doc/itanium-mangling.txt)

### Output Format

The output file contains one mangled name per target, followed by a newline.

For successful test cases, standard output and standard error are ignored. You
may use them for diagnostics.

### Error Handling

If command-line parsing, input reading, fact parsing, or name construction
fails, `abimangle` shall exit with failure.

For negative tests, exact diagnostics are not the grading contract. The harness
compares exit status first. If the reference path fails, stdout and stderr are
diagnostic side effects rather than required output.

### Testing

Run the ABI naming suite with:

```sh
make test
```

To run one test through the shared check target:

```sh
make check TEST=tests/abi/100-global-function.t
```

For each test case `x.t`:

- `abimangle` is executed to produce `x.my`
- the exit status is recorded in `x.my.exit_status`
- `x.my` is compared against `x.ref`
- `x.my.exit_status` is compared against `x.ref.exit_status`

The checked-in references are the oracle. Your tests should not invoke the host
compiler, `nm`, `readelf`, `objdump`, or a demangler as a live ABI-name oracle,
because host compiler and standard-library version differences can create
noise around ABI tags, inline namespaces, and local entity numbering.

### Assignment Boundary

This assignment owns standalone ABI name construction from normalized ABI fact
files.

To complete this assignment, implement this behavior:

1. Parse normalized ABI fact files.
2. Represent the ABI facts with enough typed structure to apply the Itanium C++
   ABI mangling grammar.
3. Encode the supported fact records into deterministic mangled names.
4. Implement substitution-table behavior in the order the Itanium ABI gives,
   as clang implements it, for the tested cases.

If `abimangle` accepts a fact file and writes a different ABI name from the
checked-in reference, the issue belongs in this assignment.

### Out Of Scope

The following are out of scope for this assignment:

- C++ source input
- C++ source parsing or semantic analysis
- LowIR generation
- relocatable object generation or host linking
- ELF, Mach-O, COFF, archives, shared libraries, or relocation records
- vtable layout, RTTI object layout, exception handling, unwind metadata, or
  host runtime behavior beyond naming the corresponding ABI entities
- demangling
- using host object tools or host compiler output as compiler input

### Design Notes (Non-Normative)

A simple implementation strategy is to keep three concerns separate:

- fact-file parsing into typed records
- ABI name encoding from those typed records
- substitution-table state for one mangled name

The `abi_mangle.h` scaffold supplies the words and codes for that shape: the
typed records store its enums, not fact words. The standalone tool and later
compiler stages should share one structured encoder implementation.

Substitution is part of the ABI grammar, not just text de-duplication. The
encoder should record substitutions in the order required by the Itanium ABI
and should compare structured facts when deciding whether a component can reuse
an existing slot. Structural comparison must retain encoding-significant facts
such as array bounds, integral-expression values, and type-trait operands while
still recognizing equivalent value arguments and canonical spellings of the
same named type.

Avoid building names by assembling large ad hoc strings that are later
reparsed. Some ABI facts contain source spellings, but type structure,
template arguments, dependent expressions, and local contexts should remain
structured until the encoder emits the final mangled name.

The compiler of later assignments mangles every symbol it emits, thousands
per translation unit once the standard library headers are in it. If each
ABI entity (a name, a type, a template argument list) is built once and
shared, a substitution check is an identity comparison and the table maps
identities to indices; comparing structures or spellings for every
candidate repeats that work on every name. A mangled name is output: produce
it when the symbol is emitted, not as a key for finding entities.

### Stage Handoff

The next stage is PA10, where `cppgm++ --emit-lowir` first emits compiler-owned
symbols. From then on the compiler's symbols must equal what `abimangle`
produces for the same entities; one encoder serving both is the direct way to
keep them equal, and later class, template, native-object, exception, and
hosted-runtime assignments extend the same facts.
