#include "300-host-parameter-pack-expansion-symbol-spelling.helper.h"

template<template<class...> class Tuple, class... A>
int count_types(Tuple<A...>);
template<class R, class... A>
int count_parameters(R (*)(A...));
template<class R, class C, class... A>
int count_member_parameters(R (C::*)(A...) const);
template<n::size_type... I>
int count_indices(indices<I...>);

long two(int, char) { return 2; }

int host_counts()
{
  return count_types(tuple<int, char, long>()) * 1000 +
    count_parameters(&two) * 100 + count_member_parameters(&X::m) * 10 +
    count_indices(indices<0, 1, 2, 3>());
}
