// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraph 2.
struct Base { virtual void f() throw(int&&); };
int main() { return 0; }
