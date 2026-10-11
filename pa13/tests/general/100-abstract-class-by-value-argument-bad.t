struct A { virtual void f() = 0; }; void take(A); void use(A &a) { take(a); }
