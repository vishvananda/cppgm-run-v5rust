// N3485 10.3p7-p8, Itanium 2.5.3: an overrider returning S* for R* where R is
// a virtual base of S needs a result-adjusting thunk through the returned
// object's vtable; expect exit 0
struct R { int r; R() : r(4) {} };
struct S : virtual R { };
struct B { virtual R* get() { return 0; } };
struct D : B { S s; S* get() { return &s; } };
int main() { D d; B& b = d; return d.get()->r == 4 && b.get()->r == 4 ? 0 : 1; }
