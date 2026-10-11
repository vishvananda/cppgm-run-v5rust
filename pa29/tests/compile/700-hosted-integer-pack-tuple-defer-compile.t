template<class T, T... I>
struct seq {};

template<template<class, int...> class BaseType, int N>
using make_seq = BaseType<int, __integer_pack(N)...>;

template<class... U>
struct tuple {};

template<class... U>
struct holder {
  typedef tuple<U...> type;
  typedef make_seq<seq, 3> seq_type;
};

typedef holder<int, long>::type tuple_type;
typedef holder<int, long>::seq_type seq_type;

int main() {
  return 0;
}
