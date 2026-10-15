// N3485 13.3.1.4p1: copy-list initialization does not admit explicit
// conversion functions for the copy constructor's reference; expect reject
struct A { int n; A(); };
struct S { explicit operator A() const; };
int main() { S s; A a = {s}; }
