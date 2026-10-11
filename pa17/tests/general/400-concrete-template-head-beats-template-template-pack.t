// VALIDATION: compile-pass
// A concrete template-id partial is more specialized than a
// template-template-parameter partial whose trailing pack can also match it.

namespace tuples {
template<class Front, class Tail>
struct cons {};
}

template<class T>
struct refs;

template<template<class...> class Tuple, class... Iterators>
struct refs<Tuple<Iterators...> >
{
  static const int value = 1;
};

template<class Front, class Tail>
struct refs<tuples::cons<Front, Tail> >
{
  static const int value = 2;
};

typedef tuples::cons<int, tuples::cons<long, void> > cons_type;

static_assert(refs<cons_type>::value == 2, "concrete head partial");

int main()
{
  return refs<cons_type>::value == 2 ? 0 : 1;
}
