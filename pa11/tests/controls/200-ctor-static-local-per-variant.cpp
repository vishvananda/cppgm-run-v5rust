// N3485 6.7/4, 7.1.2/4: a static local of a constructor is one object; the
// complete-object (C1) and base-object (C2) constructor entries must share it
// (Itanium names it _ZZN4BaseC4EvE1n); expect exit 0
struct Base { int id; Base() { static int n = 0; id = ++n; } };
struct Derived : Base { Derived() {} };
int main() { Derived d; Base b; return d.id == 1 && b.id == 2 ? 0 : 1; }
