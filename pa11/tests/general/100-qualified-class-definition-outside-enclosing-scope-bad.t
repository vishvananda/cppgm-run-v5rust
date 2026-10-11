// N3485 9/11: a class-specifier with a nested-name-specifier shall appear in a
// namespace enclosing the previous declaration; inside struct C it is
// ill-formed; must be rejected
struct A { struct B; };
struct C { struct A::B { int x; }; };
