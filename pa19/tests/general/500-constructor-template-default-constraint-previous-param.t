// VALIDATION: compile-pass
// Reduced from Boost.Asio's mutable_buffer span constructor. An instantiated
// constructor template must preserve earlier function parameters while checking
// dependent defaulted SFINAE constraints.

namespace meta {
template<bool B, class T>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

struct defaulted_constraint {};

template<bool B, class T>
using constraint_t = typename enable_if<B, T>::type;

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

template<class T>
struct is_const
{
  static const bool value = false;
};

template<class T>
struct is_const<const T>
{
  static const bool value = true;
};
}

typedef decltype(sizeof(0)) size_t;

class mutable_buffer {
public:
  mutable_buffer() {}

  mutable_buffer(void *, size_t) {}

  template<template<class, size_t> class Span, class T, size_t Extent>
  mutable_buffer(
      const Span<T, Extent> &span,
      meta::constraint_t<!meta::is_const<T>::value,
                         meta::defaulted_constraint> =
          meta::defaulted_constraint(),
      meta::constraint_t<sizeof(T) == 1,
                         meta::defaulted_constraint> =
          meta::defaulted_constraint(),
      meta::constraint_t<
          meta::is_same<decltype(span.subspan(0, 0)),
                        Span<T, static_cast<size_t>(-1)> >::value,
          meta::defaulted_constraint> =
          meta::defaulted_constraint())
  {
  }
};

template<class T, size_t Extent>
struct span {
  T *data() const { return 0; }

  size_t size() const { return 0; }

  span<T, static_cast<size_t>(-1)> subspan(size_t, size_t = 0) const
  {
    return span<T, static_cast<size_t>(-1)>();
  }
};

int main()
{
  span<char, 1> s;
  mutable_buffer b(s);
  return 0;
}
