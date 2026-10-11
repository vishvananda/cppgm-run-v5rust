// N3485 10.3p7-8 + Itanium 2.5.2-2.5.3: D::get (S*) overrides B::get (R*) with
// R a virtual base of S, so D's primary vtable gets its own slot returning S*
// and B's slot holds a result-adjusting thunk; pd->get() through D* yields the
// S*; expect exit 0
struct R { int r; R() : r(4) {} };
struct S1 : virtual R { }; struct S2 : virtual R { };
struct S : S1, S2 { };
struct B { virtual R* get() { return 0; } };
struct D : B { S s; S* get() { return &s; } };
int main() { D d; D* pd = &d; return pd->get()->r == 4 ? 0 : 1; }
