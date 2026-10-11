template<class... T>
struct type_list {
};

template<class Set, class... T>
struct set_push_back {
};

template<class Set, class Value>
struct contains {
  static constexpr bool value = false;
};

template<bool Condition, class True, class False>
struct select;

template<class True, class False>
struct select<true, True, False> {
  using type = True;
};

template<class True, class False>
struct select<false, True, False> {
  using type = False;
};

template<class Condition, class True, class False>
using select_t = typename select<Condition::value, True, False>::type;

template<template<class...> class List, class... U>
struct set_push_back<List<U...> > {
  using type = List<U...>;
};

template<template<class...> class List,
         class... U,
         class First,
         class... Rest>
struct set_push_back<List<U...>, First, Rest...> {
  using next = select_t<contains<List<U...>, First>,
                        List<U...>,
                        List<U..., First> >;
  using type = typename set_push_back<next, Rest...>::type;
};

namespace tags {
struct first {
};

struct second {
};
}

template<class A, class B>
struct same {
  static constexpr bool value = false;
};

template<class A>
struct same<A, A> {
  static constexpr bool value = true;
};

using result = set_push_back<type_list<>, tags::first, tags::second>::type;

int main() {
  static_assert(same<result, type_list<tags::first, tags::second> >::value, "");
  return 0;
}
