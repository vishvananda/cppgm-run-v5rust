// N3485 7.3.3p17, 11.2: using R::get names a member template of A, a private
// base of R, which is inaccessible in H (the non-template form is rejected).
// Must reject.
struct A { template<int I> int get() { return 7 + I; } };
struct R : private A { };
struct H : private R { using R::get; };
int main() { H h; return h.get<0>(); }
