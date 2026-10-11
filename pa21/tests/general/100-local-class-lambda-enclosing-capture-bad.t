void f() { int x=1; struct A { int get() { return [&] { return x; }(); } }; }
