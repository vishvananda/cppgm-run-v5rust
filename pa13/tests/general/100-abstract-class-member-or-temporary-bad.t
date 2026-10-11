// N3485 10.4p2-p3: no object of an abstract class can be created except as a
// base subobject: a member of abstract type (and a converting-constructor
// temporary of it) is ill-formed; must be rejected
struct A { virtual void f() = 0; };
struct H { A a; };
int main() { return 0; }
