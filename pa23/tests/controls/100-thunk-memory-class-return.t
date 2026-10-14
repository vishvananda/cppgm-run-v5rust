// Itanium ABI 2.5.5/3.2 + SysV psABI 3.2.3: the this-adjusting thunk for
// C::label (overriding the second base B) must forward a memory-class (sret)
// return value; expect exit 0
struct R { long a, b, c; };
struct A { virtual ~A() {} virtual int x() { return 0; } };
struct B { virtual ~B() {} virtual R label() const { R r = {1, 2, 3}; return r; } };
struct C : A, B { R label() const override { R r = {4, 5, 6}; return r; } };
int main() { C c; B* b = &c; return b->label().c == 6 ? 0 : 1; }
