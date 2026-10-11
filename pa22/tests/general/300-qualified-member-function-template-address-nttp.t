// VALIDATION: compile-pass
struct A { template<class T> int f(T); };
template<int (A::*)(int)> struct helper {};
helper<&A::template f<int> > value;
