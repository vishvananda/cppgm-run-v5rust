struct X { void f() const; };
template<class R, class C, class... P, class O, class... T>
int pick(R (C::*)(P...) const, O const&, T const&...);
X x; int selected = pick(&X::f, x);
