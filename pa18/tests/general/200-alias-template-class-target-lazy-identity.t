template<class T> struct tuple {
  char storage[sizeof(T)];
};

template<class...> struct list {
};

template<class L1, class L2> struct assign;

template<template<class...> class L1, class... T,
         template<class...> class L2, class... U>
struct assign<L1<T...>, L2<U...> > {
  typedef L1<U...> type;
};

template<class L1, class L2>
using assign_t = typename assign<L1, L2>::type;

template<class L, class R> struct same_type {
  static const bool value = false;
};

template<class T> struct same_type<T, T> {
  static const bool value = true;
};

int main() {
  typedef assign_t<tuple<int>, list<int[]> > rebound;
  static_assert(same_type<rebound, tuple<int[]> >::value, "");
  return 0;
}
