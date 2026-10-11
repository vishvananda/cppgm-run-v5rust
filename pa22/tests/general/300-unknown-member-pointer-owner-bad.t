// VALIDATION: compile-fail
template<class> struct X {};
X<void (Missing::*)()> x;
