// VALIDATION: compile-pass
// A pack expansion in a nested template-id pattern matches each actual argument.

template<class... T>
struct tuple
{
};

template<class T>
struct slot
{
};

template<class T>
struct unwrap;

template<template<class> class S, class... E>
struct unwrap<tuple<S<E>...> >
{
  typedef tuple<E...> type;
};

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class T>
struct is_same<T, T>
{
  static const bool value = true;
};

typedef typename unwrap<tuple<slot<char>, slot<long> > >::type result;

int main()
{
  return is_same<result, tuple<char, long> >::value ? 0 : 1;
}
