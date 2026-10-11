# Directives and preprocessing state

Adapted from the CPPGM Foundation preprocessor handout (2013). These rules
complete [the preprocessor assignment](README.md) after
[macro replacement](macros.md).


Logically each source file is processed in turn with no shared state between them.

The following preprocessing directives must be implemented:

    - conditional inclusion (`#if`...`#endif`)  (using PA3)
    - source file inclusion (`#include`)
    - macro replacement ([macros.md](macros.md))
    - line control (`#line`)
    - a course-defined list of pre-defined macros
    - a course-defined list of pragmas (`#pragma`)
    - error directive (`#error`)
    - null directive and non-directive

Also you must implement the `_Pragma()` operator

#### Preprocessing Directive List

    #if
    #ifdef
    #ifndef
    #elif
    #else
    #endif
    #include
    #define
    #undef
    #line
    #error
    #pragma

If no tokens follow `#` on a logical line, it is a null directive and is always ignored.

If the first token after `#` is not an identifier, or not one of the identifiers in the above list, the logical line is a `non-directive`. It is an error if in an active `#if` section, or ignored if it is in an inactive one.

#### Pre-defined Macros

The following pre-defined macros shall be implemented:

    #define __CPPGM__ 201303L               // CPPGM course run version
    #define __cplusplus 201103L             // C++ version
    #define __STDC_HOSTED__ 1              // hosted implementation

The above are fixed values.

    #define __CPPGM_AUTHOR__ "Your Name"   // A chosen author label

Choose any author string; no fixture prints it.

    #define __FILE__ "foo"                 // current presumed source file name
    #define __LINE__ 123                   // current presumed source line number

Behaviour specified below.

    #define __DATE__ "Mmm dd yyyy"         // build date from asctime
    #define __TIME__ "hh:mm:ss"            // build time from asctime

`__DATE__` is `"Mmm dd yyyy"` and `__TIME__` is `"hh:mm:ss"`, the fields `asctime` prints.  Take the time once per run, so that every source file of one invocation sees the same values.

You may conditionally-implement any other pre-defined macros provided they start with an underscore and then a capital (`_Foo`) or another underscore (`__foo`).

#### Pragmas

The following pragmas shall be supported:

    #pragma once

The following pragma shall NOT be supported (and so should be ignored):

    #pragma cppgm_mock_unknown pptokens

You may conditionally-implement any additional pragmas.

The course-defined treatment of the _Pragma operator is as follows:

> A `_Pragma(string-literal)` shall be recognized only in a `text-sequence`, and only _after_ all macro replacement.  Any occurrence of the identifier `_Pragma` must be followed by `( string-literal )` or it is an error.  The pragma operator invocation tokens shall be removed from the `text-sequence` after it is executed.

#### Source File Inclusion and `__FILE__`

The course-defined handling of source file inclusion in the default case shall be as follows:

Below, `__FILE__` denotes the presumed name of the current file, however you represent it.  Initially it is the command-line argument.

When a `#include` directive is encountered it will match this form:

    #include pptokens

where `pptokens` is any sequences of `preprocessing-tokens`.

`pptokens` shall be macro replaced as specified in [macros.md](macros.md).  If the resulting sequence of tokens is not a `header-name` or an ordinary `string-literal`, behaviour is undefined.  (Notice that a `header-name` can only be the result if it was already there, before macro replacement, as per PA1)

Both `header-name` types (`<foo>` and `"foo"`) are treated the same.  The delimiters are stripped and the resulting code points are converted into a UTF-8 string, we shall call `nextf`.

In the case of an ordinary `string-literal` it shall be post-tokenized into a UTF-8 string, and likewise we shall call the string `nextf`.

If `__FILE__` contains a `/` character, a new string `pathrel` is formed by concatenating (A) the sub-string of `__FILE__` up to and including the last `/`; and (B) the string `nextf`

In pseudo-code:

    __FILE__ = "foo/bar/baz"

    #include "qux"

    nextf = "qux"

    pathrel = "foo/bar/" + "qux" = "foo/bar/qux"

Once the two strings `nextf` and `pathrel` are identified they are searched as follows:

1. If `pathrel` is defined and a file exists of that path relative to the current working directory (or absolute if it starts with `/`), it shall be the include file.
2. Otherwise, if a file exists of the path `nextf` relative to the current working directory (or absolute if it starts with a `/`) shall be the include file.
3. Otherwise, it is an error and `EXIT_FAILURE` should be returned.

The new value of `__FILE__` is whichever one of 1 or 2 succeeded.

Recall that you can optionally implement command-line switches which alter or extend this behaviour.  You may wish to implement a `-I <path>` switch to add additional paths, and/or a `--stdinc` switch which also searches `/usr/include`, and so on.  However, exactly the two search paths specified (`__FILE__` relative, and current working directory relative) must be the `preproc` default behaviour.

#### Line Control and `__LINE__`

    #line pptokens

where `pptokens` is any sequences of `preprocessing-tokens`.  `preprocessing-tokens` are macro-replaced.

After macro replacement `#line` will match one of the following two forms:

    #line ppnumber
    #line ppnumber string-literal

The `ppnumber` should post-tokenize to a positive integer.  The `string-literal` if present shall be an ordinary `string-literal`.  If this is not the case behaviour is undefined.

The integer shall set the current value of `__LINE__`, the string shall set the current value of `__FILE__` (notice that this will impact future `#include` behaviour).

#### Pragma Once

The course-defined `#pragma once` handling is specified as follows:

A file that has executed `#pragma once` is not entered again in the same
translation unit, however its path is spelled: a file's identity is the
host's (device and inode), so distinct path spellings of one file share the
once state. Reset that state for each primary source file.

The preprocessor scaffold supplies a helper for that identity, which uses the
host `stat` interface and which you may use without implementing system calls:

    bool GetPreprocessorFileId(const string& path, PreprocessorFileId& out_fileid)

It takes a file `path` and an out parameter of type `PreprocessorFileId`, and
returns `true` on success (or `false` when the file does not exist or cannot
be read).  One way to use it: keep, per primary source file, a set of the ids
of the files that have executed `#pragma once`; add the current file's id when
you process the directive; and before entering an included file, look up its
id and skip the inclusion when the id is in the set.

### Reading

You should read the parts of clause _16 Preprocessing directives_ that you have not already read.

### Design Notes (Optional)

You will need to start with PA1 `pptoken` code as usual, however you will need to add source file name and physical line tracking to support `__FILE__` and `__LINE__`.

One way to do this is before the entry to `PPTokenizer` count physical new line characters _before_ they enter the tokenizer, and then as `preprocessing-tokens` are emitted, store the current source file name and line in each token.  So you are not storing a string in each token (although they are usually copy-on-write anyway), you may want to store filenames in a table and use an `int` index.  After macro invocation assign the source file and line of each produced token to be that of the head.  When you invoke a `__FILE__` or `__LINE__` predefined macro, simply replace it with its own file or line.

Now we have a stream of `pptokens` with file and line numbers marked.

The next step is to accumulate preprocessing directives and text sequences.  You will need to do this in a stream, as some preprocessing directives will change the system state that will effect how later directives are handled.  You can delimit preprocessing tokens as previously.  A `new-line #` signifies the start of a preprocessing directive and a `new-line` ends one.  This can be done with a little DFA or state machine.

Each time a preprocessing directive is encountered you will need to take a certain action.

Conditional inclusion is a bit more complicated.  A `#if`, `#ifdef` or `#ifndef` will start an if group, a matching `#endif` will close it.  In between there can be one of more `#elifs` and an optional `#else`.  Depending on the corresponding truth values of the controlling expressions, each section is either active or inactive.

Whether in an inactive or active group, a nested `#if` group must be ordered correctly (with respect to `#elifs`, `#else` and `#endif`), however inside an inactive group all preprocessing directives (aside from their name) are ignored.  For example, the following is valid:

    #if 0
    #if foo bar baz
    #elif foo bar baz ... @
    #else @@@@@
    #endif @@@@@
    #else
    ok
    #endif

This is because the nested `#if` group is in an inactive section.  Any sequence of tokens can come after the directive names in this case.  However the following is invalid:

    #if 0
    #if foo bar baz
    #else @@@@@
    #elif foo bar baz ... @
    #endif @@@@@
    #else
    ok
    #endif

The `#else` and `#elif` directives are in the wrong order.  This order must be correct even in an inactive section.

This can be implemented with a stack of states.  When you enter an `#if` group or `#include`, push a new state onto the stack.  As you proceed through an `#if` group update the state.  When you exit an `#if` group or `#include` pop the state off the stack.  The state can keep track of whether or not you are in an `#if` group, where in the `#if` group you are, and whether or not you are currently active.  As the different directives are encountered they alter the state on the top of the stack or push or pop states from it.


## Scope and handoff

The default include search above is the graded contract. Searching system
headers is optional here. Host include discovery and nonstandard header
compatibility are introduced later; they are not prerequisites for completing
the preprocessor. Retain a callable token pipeline with source locations so
the parser can consume tokens directly without reparsing this textual dump.
