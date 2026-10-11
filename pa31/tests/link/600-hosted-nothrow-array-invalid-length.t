// An array new-expression whose count is invalid (negative, or a size beyond
// std::size_t's range: 5.3.4p7) through a non-throwing allocation function
// yields null without calling it (CWG 1992, C++17 [expr.new]p8; g++ and clang
// give null for these); the throwing form throws std::bad_array_new_length (a
// std::bad_alloc: the hosts' is one too)
