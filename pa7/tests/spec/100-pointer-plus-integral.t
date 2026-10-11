// N3485 focus: 5.7 [expr.add] pointer arithmetic
// HHC-160
typedef unsigned long size_t;

char16_t* f(char16_t* s, size_t n) { return s + n; }
