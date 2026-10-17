// expect exit 0
// Retained value operands bind class and member-alias arguments at separate depths.
template<bool, class T, class F> struct select { typedef T type; };
template<class T, class F> struct select<false, T, F> { typedef F type; };
template<class C, class T, class F>
using choose = typename select<C::value, T, F>::type;

struct yes { static const bool value = true; };
struct no { static const bool value = false; };
template<class> struct trait { static const bool value = true; };

template<class C> struct outer {
  template<class F> struct middle {
    template<class T> using pick = choose<C, T, F>;
    typedef pick<int> type;
  };
};
template<template<class> class P> struct named_outer {
  template<class T> using pick = choose<P<T>, T, void>;
  typedef pick<int> type;
};
template<class...> struct list {};
template<class C> struct packed_outer {
  template<class... T> using type = list<choose<C, T, void>...>;
};
template<class, class> struct same { static const bool value = false; };
template<class T> struct same<T, T> { static const bool value = true; };

static_assert(same<outer<yes>::middle<char>::type, int>::value, "outer true");
static_assert(same<outer<no>::middle<char>::type, char>::value, "outer false");
static_assert(same<named_outer<trait>::type, int>::value, "template name");
static_assert(same<packed_outer<yes>::type<int, char>, list<int, char>>::value, "packs");
int main() { return 0; }
