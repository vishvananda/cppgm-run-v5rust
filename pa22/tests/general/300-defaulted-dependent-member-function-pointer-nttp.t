// A defaulted member-function pointer argument is resolved after substituting
// the preceding type parameters into both its type and its initializer.
template<class E> struct N { void f(E const &); };
template<class E, class C = N<E>, void (C::*P)(E const &) = &N<E>::f> struct T {};
static_assert(sizeof(T<int>) == 1, "");
