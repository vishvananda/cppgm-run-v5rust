struct yes { static const bool value = true; };
struct no { static const bool value = false; };
template<template<class...> class F, class... T> struct valid_impl {
  template<template<class...> class G, class = G<T...> > static yes check(int);
  template<template<class...> class> static no check(...);
  using type = decltype(check<F>(0));
};
template<template<class...> class F, class... T>
using valid = typename valid_impl<F, T...>::type;
template<bool, class T, class E> struct if_impl { using type = E; };
template<class T, class E> struct if_impl<true, T, E> { using type = T; };
template<class C, class T, class E> using if_ = typename if_impl<C::value, T, E>::type;
template<template<class...> class F, class... T> struct defer_impl { using type = F<T...>; };
struct no_type {};
template<template<class...> class F, class... T>
using defer = if_<valid<F, T...>, defer_impl<F, T...>, no_type>;
template<class V, template<class...> class F> struct q2 {
  template<class A, class B> using fn = F<F<V, A>, B>;
};
template<class V, class Q>
using fold_q = typename defer<q2<V, Q::template fn>::template fn, int, int>::type;
template<unsigned> struct size {};
struct Q { template<class N, class> using fn = N; };
static_assert(valid<fold_q, size<0>, Q>::value, "");
int main() { return 0; }
