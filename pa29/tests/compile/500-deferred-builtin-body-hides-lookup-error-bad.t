// PA29 README (hosted inline bodies): a body deferred for an unimplemented
// reserved builtin must still diagnose ordinary lookup errors (undeclared_y);
// N3485 3.4.1; expect compile REJECT
struct S { void f() { __builtin_nonexistent_x(); undeclared_y(); } };
int main() { return 0; }
