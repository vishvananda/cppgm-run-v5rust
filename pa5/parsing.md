# Building the parser you will keep

This guide accompanies [the parser/AST assignment](README.md) and explains
the parsing approach for the compiler's structured AST. The assignment
grammar and AST contract are in the main handout.

## Work in useful groups

1. Connect the completed preprocessor to a token cursor. Parse empty units,
   simple declarations, identifiers, literals, and function bodies.
2. Add expression precedence, associativity, statements, and declarators.
   Distinguish a failed parse from a successful optional empty production.
3. Add namespaces, classes, enums, template parameter declarations, and the
   corresponding name categories. Preserve template arguments as syntax.
4. Resolve the required declaration/expression and angle-bracket ambiguities.
   Inspect trees as well as whether a source was accepted.

The 100, 200, and 300 groups give focused feedback without adding graded
milestones or executables. Run individual cases with `make check TEST=...`.
All groups feed the same parser and AST. Template syntax in a class body can
be parsed before that class's semantics or generated code is implemented.

## Grammar notation and token context

The authoritative grammar is `pa5.gram`, a link to `shared/source.gram`.
Indented lines give alternatives. `foo?` means zero or one, `foo*` zero or
more, and `foo+` one or more. Parentheses group grammar elements. Backslash
line splices are ignored. The HTML explorer lists productions and their
`USED`, `FIRST`, and `FOLLOW` relationships. These sets help make predictive
choices; they are not a requirement to generate a parser table.

The token vocabulary includes these contextual forms:

| Grammar token | Meaning |
| --- | --- |
| `TT_IDENTIFIER` | An ordinary identifier |
| `TT_LITERAL` | A literal, including a user-defined literal |
| `ST_EMPTYSTR` | The literal with spelling `""`; it also matches `TT_LITERAL` |
| `ST_EOF` | End of this translation unit |
| `ST_FINAL`, `ST_OVERRIDE` | Identifiers treated specially in their class-member contexts |
| `ST_RSHIFT_1`, `ST_RSHIFT_2` | The two logical halves of one `>>` token |

`final` and `override` remain identifiers outside those special contexts.
You can represent a split `>>` with cursor state or two linked token pieces.
Keep their original adjacency available so a shift expression still sees one
operator. The grammar's notation does not dictate your token storage layout.

## Parse functions, rollback, and trees

A recursive-descent implementation can use one function per useful grammar
production, with common prefixes factored into helpers. A successful function
returns a structured node and leaves the cursor after its input. A failed
speculative alternative restores the cursor, partial nodes, and any name
facts it introduced. A checkpoint must cover all of that state; restoring
only the token position can make a later alternative see declarations that
never happened.

Use prediction to avoid trying every production everywhere. Expression
parsing can use precedence climbing or another approach that preserves the
required tree. Give assignment and conditional expressions their right
associativity, while ordinary arithmetic groups according to its precedence
and associativity. Parentheses affect grouping; preserve the distinctions
that the AST dump exposes.

Keep parse failure distinct from a successful empty parameter list or an
optional missing initializer. Make progress in repetition loops and stop at
an explicit terminator. A debugging trace can show the input position,
production, and success/failure, but its format is not graded.

Build the structured AST from the start. The assignment's nodes express
actual declarations, declarators, and expressions that semantic analysis will
consume. A disposable generic recognizer tree would create a second
representation to replace later. The AST text is a view for tests, not the
data structure the next assignment should parse back in.

## The name-category boundary

C++ syntax sometimes needs to know what an identifier denotes. In a block,
`x * y;` declares a pointer if `x` names a type; if `x` is a value, it is a
multiplication expression. Record enough category information from declarations
to distinguish types, templates, and values, including scope entry/exit and
parameter shadowing. This is smaller than full semantic lookup: there are no
canonical types, overload sets, deduction, or instantiation to compute here.

For unresolved names in syntax-only fixtures, the course uses lexical hints:
an identifier containing `C`, `Y`, or `E` can stand for a type; `T` marks a
potential template/type name. These are fallback hints. A known value or
non-type parameter overrides them. Namespace components and dependent names
remain structured names for later lookup, rather than requiring an invented
naming restriction for every identifier.

Prefer real declarations in new examples. For example:

```cpp
int T1 = 1;
int result = T1 < 2;
```

The initializer is a relational expression. Its `T` does not make it an
incomplete template-id. Likewise, a new template parameter declaration must
replace any previous category associated with its spelling. Explicit
`typename` and `template` provide syntax information in dependent contexts.
Later semantic lookup replaces lexical fallback while preserving the parser
and its AST.

## Declaration versus expression

When both parses are possible in the required C++11 grammar, apply the
language's declaration preference (N3485 6.8). Given a type `C`:

```cpp
C(a)->m = 7;  // expression: construction, member access, assignment
C(a)++;       // expression: construction, postfix increment
C(*d)(int);   // declaration of a pointer to a function returning C
C(e)[5];      // declaration of an array
C(a);         // declaration of an object named a
```

These examples illustrate independent contexts, not a single valid block
with repeated declarations. The grouped case
`tests/general/300-declaration-expression-intake.t` supplies explicit contexts.
Its AST distinguishes the three expression statements from declarations;
acceptance alone would not show that distinction.

N3485 8.2 also distinguishes functions from initialized objects:

```cpp
C w(int(a));  // function: a is a parenthesized parameter name
C x(int());   // function taking a parameter adjusted from function type
C y((int)a);  // object initialized by a cast expression
C z = int(a); // object initialized by a conversion expression
```

If the inner name instead denotes a type, the parameter can itself have a
function type: after `typedef int value_type;`, `int f(int(value_type));`
declares `f` with a function parameter, later adjusted to a pointer. This
choice depends on the name category. Parentheses around an ordinary parameter
name do not turn that name into a type.

## Angle brackets and operators

Inside a template argument list, a closing `>` at that delimiter level ends
the argument list. Parenthesized subexpressions can contain comparisons and
shifts without ending it. For example:

```cpp
template<int n> struct leaf {};
template<class T> struct outer {};
leaf<(1 > 2)> comparison;
outer<leaf<1>> nested;
outer<leaf<(6 >> 1)>> shifted;
```

The final line contains both a shift and nested closing brackets. Track the
relevant delimiter context, including parentheses, brackets, and braces, as
you parse. Do not globally replace every `>` with a close token independent
of its expression context.

Operator names also have template-id syntax, such as `operator+<int>(a,b)`
and `operator "" _size<'1'>()`. Preserve the operator name and its argument
syntax before parsing any following relational operator. The explicit
contexts in `tests/general/300-operator-template-calls.t` exercise this
boundary without relying on an unresolved mock name.

## What syntax acceptance means

Syntax fixtures may use unresolved names or omit semantic definitions because
type checking is a later stage. The shared grammar and handout define the
supported boundary; a few rejection cases test it.
