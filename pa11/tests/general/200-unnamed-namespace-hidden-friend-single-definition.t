namespace { struct X { friend void f(const X &) {} }; X x; }
