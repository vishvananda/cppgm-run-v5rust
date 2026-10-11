// VALIDATION: compile-fail
// N3485 focus: 15.4 [except.spec] paragraph 8; 15.3 [except.handle] paragraph 3.
// A nonconst pointer reference handler cannot use a pointer conversion.
struct Error {};
struct DerivedError : Error {};
struct Base { virtual void f() throw(Error*&); };
struct Derived : Base { void f() throw(DerivedError*) override; };
int main() { return 0; }
