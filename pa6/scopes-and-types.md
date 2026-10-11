# Scopes, declarations, and types

This guide accompanies [the first semantic assignment](README.md) and
builds directly on the compiler AST.

## Implementation order

1. Introduce scope, entity, binding, and type records with identities that can
   survive later expression analysis. Walk the AST to create scopes and
   register declarations in source order.
2. Build fundamental and declarator-derived types. Add aliases, compatible
   declaration matching, function parameter normalization, and array completion.
3. Add unqualified and qualified lookup, namespaces, namespace aliases, using
   declarations/directives, inline namespaces, and their scope interactions.
4. Add the class/enum scopes and small constant-expression subset needed by
   the handout. Check bounds and `static_assert` results, then inspect the
   complete scope/type dump.

The numbered test groups are development checkpoints within one assignment.
They do not prescribe a separate namespace parser or intermediate program.
PA7 will add expression typing and calls to this same representation.

## Separate the concepts

A scope describes where a name is visible. An entity is the object, function,
type, or namespace that a declaration denotes. A binding connects a name in
a scope to an entity or type. Reopening a namespace extends its existing scope;
a namespace alias supplies another way to name that scope. A type alias names
a type without creating a distinct underlying type.

A name is not a sufficient entity identity: different namespaces can both
contain `x`, and multiple declarations can denote one object. Give records
stable identity, using indices, stable objects, or another suitable ownership
scheme. Interning structurally equal types is useful but is not a requirement
to reproduce the reference compiler's physical layout.

Process translation units independently for lexical lookup. A declaration
in one file cannot make a name visible in another file simply because both
files appear on the command line. Later linking associates externally linked
entities across units without merging their lexical scopes.

## Build types from declarators

Fundamental spellings are:

```text
signed char                 unsigned char
short int                   unsigned short int
int                         unsigned int
long int                    unsigned long int
long long int               unsigned long long int
char                        wchar_t
char16_t                    char32_t
bool                        float
double                      long double
void                        nullptr_t
```

Compound types use these recursive forms, where `T`, `R`, and `P` are types:

```text
const T
volatile T
const volatile T
pointer to T
lvalue-reference to T
rvalue-reference to T
array of 0 T
array of N T
function of (P1, P2) returning R
function of (P1, P2, ...) returning R
```

An array bound printed as `0` means unknown bound in this assignment; an
explicit zero bound is outside its valid bound rules.

For instance, `const int *p` is `pointer to const int`, whereas
`int *const p = nullptr` is `const pointer to int`. Traverse the declarator's
structure, including parentheses, to compose those operations. Flattening
the token sequence loses distinctions such as a pointer to a function versus
a function returning a pointer.

An alias preserves the type it denotes. Qualifying an array alias qualifies
its element type. A later compatible declaration can complete an incomplete
array: `extern int a[]; int a[10];` denotes one object whose completed type
has bound 10. Earlier visible declarations can therefore print the completed
bound in the scope dump.

References through aliases follow reference collapsing: a combination with an
lvalue reference yields an lvalue reference; combining only rvalue references
yields an rvalue reference. Cv qualification on a reference alias does not
qualify its referred-to type. Keep these type operations independent from
later expression-level reference binding and temporary lifetime rules.

## Source parameter types and canonical signatures

The type/scope dump preserves declared function parameter types. Thus these
can print different source forms:

```cpp
void f(const int);
void f(volatile int);
void g(void callback(int), int* data);
void g(void (*)(int), int data[3]);
```

For function identity, both `f` declarations have one `int` parameter. Both
`g` declarations have a pointer-to-function parameter and a pointer-to-int
parameter. Array and function parameter types adjust to pointers, and
top-level cv qualifiers are removed from the signature. Cv qualifiers below
a pointer remain significant. A sole `void` parameter is the empty list.

Retain the distinction between source parameter types, parameter object types,
and the normalized signature. A `const int` parameter in a definition is still
a const local parameter even though its function signature matches `int`.
The scope dump shows the source view; PA7's call-resolution tests check the
canonical function identity as well.

Unnamed function parameters also have declarators. In `int f(int());`, the
parameter is a function returning `int`, adjusted to a pointer in the
signature. It must not silently become an ordinary `int` parameter because
its AST child is an abstract declarator. The parser guide explains the
related distinction between `int(a)` with a parameter name and `int(T)` with
a known type name.

## Lookup and scope changes

Unqualified lookup starts in the relevant current scope and follows enclosing
scopes under the course rules. Qualified lookup first resolves the qualifier,
then searches the named scope. A declaration's point matters: later aliases
must not retroactively change an earlier resolved binding.

Using declarations introduce selected names. Using directives add namespace
lookup relationships; they do not copy a namespace's current contents into
a permanent flat map. Later additions to a nominated namespace must become
visible where the rules require them. The tests distinguish nested and sibling
namespace relationships, transitive directives, and qualified lookup through
inline namespaces. Keep namespace-target lookup distinguishable from ordinary
value lookup so a value with the same spelling does not always hide a valid
namespace target in a namespace-only context.

A straightforward lookup walk can be correct without caching. If you add a
cache, changes to declarations or using relationships must invalidate affected
results. `200-lookup-after-using-graph-change.t` asserts the lookup results;
it does not require a cache or any particular search algorithm.

Class scopes add complete-class contexts: a member function body can see a
member type declared later in that class. Keep that syntax/lookup interaction
consistent with the parser, using the structured declarations already stored.
Do not create a second parser for semantic analysis.

## Constants at this boundary

Array bounds, enumerators, and `static_assert` need the supported integral
constant evaluator described in the handout. Observe values through the
resulting bound, enumerator value, or assertion outcome. Merely printing the
type of a constant object does not test its initialized value.

Implement only this slice now. Full floating and pointer constant evaluation,
initialized storage bytes, runtime object identity, and cross-file definition
resolution arrive at their owning later assignments.

Useful standard reading is N3485 3.4 (lookup), 7.1.6 (type specifiers and cv
qualification), 7.3 (namespaces), and 8.3 (declarators), alongside the concrete
course subset. The handout and this guide supply the prerequisites.
