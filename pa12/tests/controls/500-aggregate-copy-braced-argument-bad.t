// expect reject
// A parenthesized braced argument must rank constructors before initialization.
struct A { int n; };
void f(A& source) { A object({source}); }
