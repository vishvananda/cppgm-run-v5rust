// expect exit 0
// A bound pack keeps its canonical ABI identity when it occurs again outside
// the mixed expansion. Own template-name packs remain distinct from type packs.
template<class... T> struct list {};
template<class A, class B> struct pair {};
template<class T> struct first {};
template<class T> struct second {};

template<class... T> struct owner {
  template<class Tag> struct nested {
    template<class... U>
    static list<pair<T, U>...> zip(list<U...>, list<T...>) {
      return list<pair<T, U>...>();
    }
  };

  template<template<class> class... U>
  static list<U<T>...> apply(list<T...>) {
    return list<U<T>...>();
  }
};

int main() {
  owner<int, char>::nested<float>::zip(list<long, short>(), list<int, char>());
  owner<int, char>::apply<first, second>(list<int, char>());
  owner<>::nested<float>::zip(list<>(), list<>());
  owner<>::apply<>(list<>());
  return 0;
}
