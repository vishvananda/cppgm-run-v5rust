namespace boost { namespace mp11 {
template<bool B> struct bool_ { static const bool value = B; };
template<class... T> struct list { static const int size = sizeof...(T); };
template<class> struct first_impl;
template<template<class...> class L, class T, class... U>
struct first_impl<L<T, U...> > { using type = T; };
template<class L> using first = typename first_impl<L>::type;
template<class M, class K> struct contains : bool_<false> {};
template<class K, class V>
struct contains<list<list<K, V> >, K> : bool_<true> {};
template<class L, class T> using append = list<L, T>;
template<bool, class T, class E> struct select { using type = T; };
template<class T, class E> struct select<false, T, E> { using type = E; };
template<class C, class T, class... E>
using mp_if = typename select<C::value, T, E...>::type;
template<class M, class T> struct update {
  using type = mp_if<contains<M, first<T> >, M, append<M, T> >;
};
}}
using pair = boost::mp11::list<char, int>;
using map = boost::mp11::list<pair>;
static_assert(boost::mp11::update<map, pair>::type::size == 1, "");
int main() { return 0; }
