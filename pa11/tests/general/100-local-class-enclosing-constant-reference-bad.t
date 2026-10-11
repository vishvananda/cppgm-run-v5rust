void f() { const int x=1; struct A { const int& get() { return x; } }; }
