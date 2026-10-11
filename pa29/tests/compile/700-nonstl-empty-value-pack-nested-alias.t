typedef unsigned long size_t;

template<class... T>
struct tuple {};

template<class... T>
struct types {};

template<size_t... I>
struct indices {};

template<class From>
struct copy_ref {
  template<class To>
  using apply = To;
};

template<class From, class To>
using copy_ref_t = typename copy_ref<From>::template apply<To>;

template<class TupleTypes, class Indices>
struct make_flat;

template<template<class...> class Tuple, class... Types, size_t... Idx>
struct make_flat<Tuple<Types...>, indices<Idx...> > {
  template<class Tp>
  using apply_quals = types<copy_ref_t<Tp, __type_pack_element<Idx, Types...> >...>;
};

struct S {};

int main()
{
  typedef typename make_flat<tuple<S&&>, indices<> >::template apply_quals<tuple<S&&> > selected;
  selected value;
  (void)value;
  return 0;
}
