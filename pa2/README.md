## CPPGM Programming Assignment 2 (posttoken)

Adapted from the original CPPGM material; revised for cppgm-extended.
See [NOTICE](../NOTICE) for attribution.

### Overview

Write a C++ application called `posttoken` that accepts a _C++ Source File_ on standard input that does NOT include any...

 - preprocessing directives
 - pre-defined macro names
 - the pragma operator

...executes phases 1, 2, 3, 4, 5, 6 and the tokenization part of 7 of the _Phases of Translation_, and describes the resulting sequence of analyzed and categorized `tokens` to standard output in the specified format.

Notice that by virtue of the restrictions, phase 4 (preprocessing) is a _no-op_ (does nothing).

### Prerequisites

You should complete Programming Assignment 1 before starting this assignment.

### Starter Kit

Use the cumulative student repository. Implement `posttoken` in
`../dev/posttoken.cpp` and reusable helpers under `../dev/src/`.
The assignment provides tests, references and a `posttoken-ref` wrapper;
see [Testing and references](../TESTING_AND_REFERENCES.md).

Reuse the implementation from PA1; do not start it over.

### Input Format

The C++ Source File shall be read from standard input `std::cin` in UTF-8 format as per PA1.

### Error Reporting

If an error occurs in phases 1, 2 or 3 `main` should `return EXIT_FAILURE` as per the behaviour of `pptoken` in PA1.

If an error occurs while converting a `preprocessing-token` to a `token` than you should output an `invalid` token (see below) and continue.

Preprocessing-tokens `#`, `##`, `%:`, `%:%:`, `non-whitespace-characters`, and `header-names`, should be output as `invalids`.

If a `preprocessing-token` contains a pre-defined macro names or the pragma operator you may treat them as identifiers.

### Implementation

Extend your existing compiler code under `dev/` and `dev/src/`.

### Output Format

`posttoken` shall write to standard output the following in UTF-8 format:

For each token in the sequence, one line shall be printed consisting of space-character separated elements.

The first element is the _Token Type_, one of:

    simple
    identifier
    literal
    user-defined-literal
    invalid

The second element is the UTF-8 encoded _source_ from.  The _source_ is the _PA1 Token Data_.  (In particular it should have `universal-character-names` decoded, but `escape-sequences` are not decoded).

In the case of string literal token concatenation, multiple `preprocessing-tokens` are used to form a single `token`.  The source for such a token is a space-seperated list of the `preprocessing-token` sources.  If string literal concatenation fails than output an `invalid` that also has a source of the same space seperated list.

The output should end with a `eof` on line by itself.

Depending on the Token Type the remaining elements are as follows:

#### simple

`simple` represent `keywords`, `operators` and `punctuators`.

This includes `true`, `false`, `nullptr`, `new` and `delete` - for this assignment we will treat them as `keywords` and not `literals` or `operators`.

They are output as

    simple <source> <TERMNAME>

Where `<TERMNAME>` is given in the list below under Simple Token Types (defined below).

For example, for an input of:

    auto &&

the output is:

    simple auto KW_AUTO
    simple && OP_LAND

#### identifier (in output context)

`identifiers` are output as

    identifier <source>

Where `<source>` is the identifier in UTF-8, for example for an input of:

    foo

the output is:

    identifier foo

#### literal

    literal <source> <type> <hexdump>

Where `<type>` is the type of the literal - either a Fundamental Type (defined below), or an `array of <n> <fundamental_type>`.

For example:

    char
    long long int
    array of 42 char16_t

And `<hexdump>` is the hexadecimal representation of the data in memory in the Linux x86-64 ABI (defined below).

> Important: the hexdump of a `floating-literal` is the IEEE-754 value the C library's decimal conversion produces (`strtof`, `strtod` and `strtold` semantics: round to nearest, ties to even).  The `PA2Decode_float`, `PA2Decode_double` and `PA2Decode_long_double` helpers in the starter code give exactly that; a decoder of your own must agree with them bit for bit on the fixtures.

For example, for an input of:

    1000000 'A' "ABC" 3.2

the output is:

	literal 1000000 int 40420F00
	literal 'A' char 41
	literal "ABC" array of 4 char 41424300
	literal 3.2 double 9A99999999990940

In the case of string literal concatenation:

    "abc" /* concat */ "def"

The output should have the sources joined space separated:

    literal "abc" "def" array of 7 char 61626364656600

#### user-defined-literal

One of:

    user-defined-literal <source> <ud-suffix> integer <prefix>
    user-defined-literal <source> <ud-suffix> floating <prefix>
    user-defined-literal <source> <ud-suffix> character <type> <hexdump>
    user-defined-literal <source> <ud-suffix> string <type> <hexdump>

Where `<ud-suffix>` is the `ud-suffix`

In the case of `integer` and `floating`, `<prefix>` is the source without the `ud-suffix`

In the case of `string` and `character`, `<type> <hexdump>` have the same meaning as in `literal` (above).

A `ud-suffix` that does not begin with `_` is reserved (17.6.4.3.5); the course defines such a token as `invalid` (for example `123yyy` and `"abc"yyy`).

Keep the phase-3 maximal-munch token even immediately after `operator`:
`operator""sv` produces the keyword followed by `invalid ""sv`, and
`operator""_valid` produces the keyword followed by a user-defined string
literal with suffix `_valid`. A space before the suffix changes tokenization:
`operator "" sv` has an ordinary empty string followed by identifier `sv`.
Interpreting a literal-operator declaration belongs to the later parser, not
this standalone post-token output.

For example:

    123_foo 4.2_bar 0x3_baz "abc"_qux 'a'_quux

outputs:

	user-defined-literal 123_foo _foo integer 123
	user-defined-literal 4.2_bar _bar floating 4.2
	user-defined-literal 0x3_baz _baz integer 0x3
	user-defined-literal "abc"_qux _qux string array of 4 char 61626300
	user-defined-literal 'a'_quux _quux character char 61

#### invalid

For any valid `preprocessing-token` that does not posttokenize output:

    invalid <source>

For example:

    # 123abc 1..e

Should output:

    invalid #
    invalid 123abc
    invalid 1..e

Also as stated above invalid string literal concatenation should output one invalid.  For example:

    u8"abc" u"def" U"ghi"

Should output:

    invalid u8"abc" u"def" U"ghi"

### Features

First you will need to apply the same functionality from pptoken to get the stream of `processing-tokens`.

`whitespace-sequence` and `new-line` are ignored.  (They are for preprocessing, which we shall implement in a later assignment)

Any occurences of `non-whitespace-character` or `header-name` output as an `invalid`.

The remaining `preprocessing-tokens` are:

    identifier
    preprocessing-op-or-punc
    pp-number
    character-literal
    user-defined-character-literal
    string-literal
    user-defined-string-literal

We will discuss each of them:

#### identifier (in preprocessing-token context)

If it is a member of the keyword list, output it as a keyword, otherwise output it as an identifier.

#### preprocessing-op-or-punc

If you encounter the operators `#`, `##`, `%:`, `%:%:` output them as `invalid`.

For the remaining `operators` and `punctuation` you should map them to the appropriate `simples` given in the table below, or output them as `invalid` if not found.

#### pp-number

You need to analyze the string and classify it as matching the `integer-literal`, `floating-literal`, `user-defined-integer-literal` or `user-defined-floating-literal` grammar.

If it doesn't match any of these grammars, output it as `invalid`.

In the case of `user-defined-integer-literal` and `user-defined-floating-literal` you need to split the `ud_suffix` off and then output the prefix and `ud-suffix` as a string.

In the case of `integer-literal` and `floating-literal` you will then need to calculate its appropriate type (see 2.14.2 and 2.14.4 in the standard).  If no appropriate type is found output it as an `invalid`.

Once the appropriate type is found you will then scan it into that type.

For `floating-literals`, once you have confirmed that they match the appropriate grammar, scan them to the value the C library's decimal conversion gives (as `PA2Decode_float`, `PA2Decode_double` and `PA2Decode_long_double` in the starter code do), so that the hexdump matches the references bit for bit.

You may _optionally_ check that `floating-literals` are in the range of their types:

> If the scaled value is not in the range of representable values for its type, the program is ill-formed. (course defined optional)

but this is not required for PA2, nor will it be tested for PA2.

For `integer-literals` you need to check that they fit into the maximal type based on their `integer-suffix`, if they do not output them as `invalid`.  Calculate the appropriate type to place them in and scan them into that type for output.  This is explained in 2.14.2.

These are the C++11 grammars. PA1 through PA4 follow them strictly, so
binary integer literals (`0b101`), hexadecimal floating literals (`0x1p3`)
and the extended floating suffixes (`1.0Q`, `1.0f128`) are `invalid` here.
From PA29 on, the compiler accepts them through the same tokenizer, because
the host's headers use them; take the dialect as a parameter of the literal
classifier now rather than write a second classifier later.

#### character-literal

Escape sequences (2.14.3.3 Table 7) need to be decoded into their code point.

We shall course define the limits of character literals as follows:

> All character literals must contain one code point in the valid Unicode range.  `[0 .. 0xD800)` and `[0xE000 .. 0x110000)`

> If an ordinary character literals code point is less than or equal to 127 (it is in the ASCII range and representable as one UTF-8 code unit), its type is `char`, otherwise its type is `int`.  In both cases its value is its code point.

> For the remaining character literals types (u, U, L) the type is as per the standard (`char16_t`, `char32_t`, `wchar_t`), and ill-formed if the code point doesn't fit in a single code unit.

`L`/`wchar_t` is defined by the ABI (defined below) as 32 bits (UTF-32, same as `U`)

#### user-defined-character-literal

The same as for character-literal except extract the `ud-suffix` first

#### string-literal

First determine the encoding of the complete maximal sequence as described
below. Source characters and `universal-character-names` in each non-raw
`string-literal` are encoded into code units using that encoding. Simple
escape sequences contribute the code unit for the character they name.

Octal and hexadecimal escape sequences are numeric escape sequences. A
numeric escape sequence contributes exactly one code unit whose value is the
value of the escape; it is not interpreted as a Unicode code point and then
re-encoded. The value must be representable in the unsigned integer type
corresponding to the sequence's element type, otherwise the complete
`string-literal` sequence is ill-formed and must be output as `invalid`. Under
the course ABI, the code-unit widths are 8 bits for `char`, 16 bits for
`char16_t`, and 32 bits for `char32_t` and `wchar_t`.

For example, `"\x3C0"` is invalid because `0x3C0` does not fit in one
`char` code unit, while `u"\x3C0"`, `U"\x3C0"`, and `L"\x3C0"` are
valid. `"\u03C0"` is also valid because a `universal-character-name` is
encoded rather than treated as a numeric escape.

You then need to consider maximal consequtive sequences of both kinds (user-defined and non-user-defined) together and apply the phase 6 rules about string concatenation.

A `string-literal` without an encoding prefix is called an _ordinary string literal_.

There are 4 `encoding-prefixes`: `u8` (UTF-8), `u` (UTF-16), `U` (UTF-32), `L` (UTF-32).

We shall course define the rule for concatenating string literals as follows:

> - If a sequence of `string-literals` contains no encoding prefixes, the entire sequence is an treated as an ordinary string literal
> - If a sequence of `string-literals` contains one of the four `encoding-prefixes`, the entire sequence shall use that prefix.
> - If a sequence of `string-literals` contains two or more different types of the four `encoding-prefixes`, the program is ill-formed, output as `invalid`.

Read 2.14.5.13 for clarification of this.

Concatenate the sequences and append a terminating 0 code point.

You will then need to UTF-8, UTF-16 or UTF-32 encode them as appropriate depending on the encoding prefix.  As our execution character set is UTF-8, ordinary string literals are UTF-8 (same as `u8`).

The type is `array of <n> <ch>`, where `<n>` is the number of __code units__ (not code points), including terminating 0 code point - and `<ch>` is the appropriate type (`char`, `char16_t`, `char32_t`, `wchar_t`)

#### user-defined-string-literal

The rules are similiar to `string-literal`, and in fact `user-defined-string-literals` and combined with normal `string-literals` as part of the maximal sequence.

The rule about combining `ud-suffixes` is similiar to the one about `encoding-prefixes`.  The total number of `ud-suffix` types must be 0 or 1, or the program is ill-formed.  If it is 0 than it is a normal `string-literal`.  If it is 1 than it is a `user-defined-string-literal` of that type.

Read 2.14.8.8 for clarification.

## Testing and references

From the repository root:

```sh
make test-pa2
make test-report-through-pa2
```

Use `posttoken-ref` to investigate behavior. Required output and exit status
are compared with the checked-in sidecars; diagnostic text is not graded.
See [Testing and references](../TESTING_AND_REFERENCES.md) for local checks
and comparison rules. Preserve supplied test inputs; reference corrections
follow that policy.

## Definition: Fundamental Types

There are exactly 20 fundamental types in C++.

They are:

    signed char
	short int
	int
	long int
    long long int
    unsigned char
    unsigned short int
    unsigned int
    unsigned long int
    unsigned long long int
    wchar_t
    char
    char16_t
    char32_t
    bool
    float
    double
    long double
    void
    nullptr_t

Each fundamental type is different (distinct) from every other.

The C++ standard does not specify exactly the size, alignment and representation of each of these types.

This information is given in a separate document called the ABI (defined below).

> Side note: In some cases the ABI specifies that two fundamental types have the same representation.  For example `char` vs `signed char`.  Also `wchar_t` vs `char32_t`.  However the toolchain must still treat them as different types (as will your bootstrap compiler).

At this point please read the _C++ standard 3.9.1 Fundamental types_.

## Definition: Linux x86-64 ABI

In order for the program produced by your toolchain to make system calls to the kernel, the size, alignment and representation of types needs to be agreed upon.

The C/C++ standards do not specify exact representations of types, so a specification called the ABI (Application Binary Interface) is formed.

The Linux x86-64 kernel adheres to an ABI specification called the "System V AMD64 ABI".  It is documented here:

[System V ABI AMD64 Architecture](http://www.cs.tufts.edu/comp/40/readings/amd64-abi.pdf)

It is course defined that your toolchain shall implement this ABI.

In particular:

- 8-bit byte
- Multibyte types are stored byte-wise little-endian
- Signed integers are stored using 2s-compliment
- The size of the fundamental types is given in the Table 3.1 Scalar Types of the [ABI spec page 12](http://www.cs.tufts.edu/comp/40/readings/amd64-abi.pdf)

For this assignment this dictates the hexdump of literals.

## Definition: Simple Token Types

The course mapping for simple tokens is given below, and this has also been transcribed in the starter code:

	simple alignas KW_ALIGNAS
	simple alignof KW_ALIGNOF
	simple and_eq OP_BANDASS
	simple and OP_LAND
	simple asm KW_ASM
	simple auto KW_AUTO
	simple bitand OP_AMP
	simple bitor OP_BOR
	simple bool KW_BOOL
	simple break KW_BREAK
	simple case KW_CASE
	simple catch KW_CATCH
	simple char16_t KW_CHAR16_T
	simple char32_t KW_CHAR32_T
	simple char KW_CHAR
	simple class KW_CLASS
	simple compl OP_COMPL
	simple const_cast KW_CONST_CAST
	simple constexpr KW_CONSTEXPR
	simple const KW_CONST
	simple continue KW_CONTINUE
	simple decltype KW_DECLTYPE
	simple default KW_DEFAULT
	simple delete KW_DELETE
	simple do KW_DO
	simple double KW_DOUBLE
	simple dynamic_cast KW_DYNAMIC_CAST
	simple else KW_ELSE
	simple enum KW_ENUM
	simple explicit KW_EXPLICIT
	simple export KW_EXPORT
	simple extern KW_EXTERN
	simple false KW_FALSE
	simple float KW_FLOAT
	simple for KW_FOR
	simple friend KW_FRIEND
	simple goto KW_GOTO
	simple if KW_IF
	simple inline KW_INLINE
	simple int KW_INT
	simple long KW_LONG
	simple mutable KW_MUTABLE
	simple namespace KW_NAMESPACE
	simple new KW_NEW
	simple noexcept KW_NOEXCEPT
	simple not_eq OP_NE
	simple not OP_LNOT
	simple nullptr KW_NULLPTR
	simple & OP_AMP
	simple -> OP_ARROW
	simple ->* OP_ARROWSTAR
	simple = OP_ASS
	simple &= OP_BANDASS
	simple | OP_BOR
	simple |= OP_BORASS
	simple : OP_COLON
	simple :: OP_COLON2
	simple , OP_COMMA
	simple ~ OP_COMPL
	simple -- OP_DEC
	simple / OP_DIV
	simple /= OP_DIVASS
	simple . OP_DOT
	simple ... OP_DOTS
	simple .* OP_DOTSTAR
	simple == OP_EQ
	simple operator KW_OPERATOR
	simple >= OP_GE
	simple > OP_GT
	simple ++ OP_INC
	simple && OP_LAND
	simple <% OP_LBRACE
	simple { OP_LBRACE
	simple <= OP_LE
	simple ! OP_LNOT
	simple || OP_LOR
	simple ( OP_LPAREN
	simple << OP_LSHIFT
	simple <<= OP_LSHIFTASS
	simple <: OP_LSQUARE
	simple [ OP_LSQUARE
	simple < OP_LT
	simple - OP_MINUS
	simple -= OP_MINUSASS
	simple % OP_MOD
	simple %= OP_MODASS
	simple != OP_NE
	simple + OP_PLUS
	simple += OP_PLUSASS
	simple ? OP_QMARK
	simple } OP_RBRACE
	simple %> OP_RBRACE
	simple ) OP_RPAREN
	simple >> OP_RSHIFT
	simple >>= OP_RSHIFTASS
	simple :> OP_RSQUARE
	simple ] OP_RSQUARE
	simple ; OP_SEMICOLON
	simple * OP_STAR
	simple *= OP_STARASS
	simple ^ OP_XOR
	simple ^= OP_XORASS
	simple or_eq OP_BORASS
	simple or OP_LOR
	simple private KW_PRIVATE
	simple protected KW_PROTECTED
	simple public KW_PUBLIC
	simple register KW_REGISTER
	simple reinterpret_cast KW_REINTERPET_CAST
	simple return KW_RETURN
	simple short KW_SHORT
	simple signed KW_SIGNED
	simple sizeof KW_SIZEOF
	simple static_assert KW_STATIC_ASSERT
	simple static_cast KW_STATIC_CAST
	simple static KW_STATIC
	simple struct KW_STRUCT
	simple switch KW_SWITCH
	simple template KW_TEMPLATE
	simple this KW_THIS
	simple thread_local KW_THREAD_LOCAL
	simple throw KW_THROW
	simple true KW_TRUE
	simple try KW_TRY
	simple typedef KW_TYPEDEF
	simple typeid KW_TYPEID
	simple typename KW_TYPENAME
	simple union KW_UNION
	simple unsigned KW_UNSIGNED
	simple using KW_USING
	simple virtual KW_VIRTUAL
	simple void KW_VOID
	simple volatile KW_VOLATILE
	simple wchar_t KW_WCHAR_T
	simple while KW_WHILE
	simple xor_eq OP_XORASS
	simple xor OP_XOR

### Design Tips (Optional)

These design tips are optional, and only suggest one way to complete this assignment.

The tips below connect your PA1 code to the PA2 output. One design that
scales, and the one the course solution uses, reads post-tokens on a
different path: classify an
identifier as a keyword once, on its interned entry, and produce post-tokens
on demand as small records that refer to their spellings rather than copy
them. The `PA2Decode_*` helpers exist so that floating hexdumps match the
references bit for bit; the compiler can decode literals directly.

You can start by creating an implementation of `IPPTokenStream` from PA1, and connect up your PA1 code to it.

`emit_whitespace_sequence` and `emit_new_line` are no-ops.

Apart from `string-literals` and `user-defined-string-literals` each `preprocessing-token` (`IPPTokenStream`) outputs exactly one token (maybe `invalid`).

`string-literals` and `user-defined-string-literals` need to be collected up into a sequence, terminated when you see some other token type.  Once terminated they are processed as one token.  You can implement `string-literals` first as one to one (outputing one token like the others), and then refactor in this string concatenation feature after.

To output a token you can use the `DebugPostTokenOutputStream` class from the starter code.

You will need to write UTF-8 and UTF-16 decoders and encoders for string and character literals.  Recall that UTF-32 encoding/decoding is trivial (UTF-32 code units are one-to-one with code points).
