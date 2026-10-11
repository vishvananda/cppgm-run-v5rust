// HHC-269
struct S { int a; S(int x) : a(x) {} };
S f() { return S{3}; }
