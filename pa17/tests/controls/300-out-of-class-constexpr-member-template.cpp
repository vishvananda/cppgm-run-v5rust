// N3485 5.19p2, 14.7.1: a constexpr member function template defined out of
// class (before use) is usable in another template's constant initializer;
// expect exit 0
template<class E> struct T { template<class U> static constexpr int f(); static const int a = sizeof(E); };
template<class E> template<class U> constexpr int T<E>::f() { return sizeof(U) + a; }
template<class E> struct W { static const int v = T<E>::template f<E>(); };
int main() { return W<int>::v == 8 ? 0 : 1; }
