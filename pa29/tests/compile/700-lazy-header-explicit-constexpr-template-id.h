#pragma once

namespace lazy_header_explicit_constexpr {

template<bool B, class T = int>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = int>
using enable_if_t = typename enable_if<B, T>::type;

template<class T1, class T2>
struct construction_check {
  template<class U1, class U2>
  static constexpr bool viable()
  {
    return __is_constructible(T1, U1) &&
           __is_constructible(T2, U2);
  }
};

template<class T1, class T2>
struct pair_like {
  T1 first;
  T2 second;

  template<class Check = construction_check<T1, T2>,
           enable_if_t<Check::template viable<T1 const &, T2 const &>(),
                       int> = 0>
  pair_like(T1 const & first_in, T2 const & second_in)
    : first(first_in),
      second(second_in)
  {
  }

  template<class U1,
           class U2,
           enable_if_t<construction_check<T1, T2>::template viable<U1, U2>(),
                       int> = 0>
  pair_like(U1 && first_in, U2 && second_in)
    : first(static_cast<U1 &&>(first_in)),
      second(static_cast<U2 &&>(second_in))
  {
  }
};

template<class T1, class T2>
pair_like<int *, int *> make_pair_like(T1 && first_in, T2 && second_in)
{
  return pair_like<int *, int *>(static_cast<T1 &&>(first_in),
                                 static_cast<T2 &&>(second_in));
}

}
