// N3485 focus: 7.1.3 [dcl.typedef] alias-declaration
using Y = const int*;
Y f(Y x) { Y y; { Y z; } }
