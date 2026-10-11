// N3485 9.4.1/2: a static member function shall not be declared const,
// volatile, or const volatile; must be rejected
struct A { static int f() const; };
