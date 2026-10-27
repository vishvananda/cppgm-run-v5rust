struct A { virtual int f() { return 1; } };
struct B : A { int f() { return 2; } };
struct V : B, A {};
struct D : virtual V {};
int call(A* value) { return value->f(); }
int main() { D object; B* base = &object; return call(base) - 2; }
