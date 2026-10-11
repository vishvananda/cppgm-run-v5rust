// Templated lambdas (C++20 [expr.prim.lambda], P0428; GCC and Clang accept
// them in C++11 mode): a capture-default captures what the body names, a named
// constant too where the body odr-uses it (&n, P0588R1), through a nested
// templated lambda's body as well
