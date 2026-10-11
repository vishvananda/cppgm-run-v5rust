#pragma once

namespace lazy_header_sfinae {

template<class T, T v>
struct integral_constant {
  typedef integral_constant type;
  typedef T value_type;
  static const T value = v;
};

template<class T, class... Args>
struct is_constructible
    : integral_constant<bool, __is_constructible(T, Args...)> {};

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
    return is_constructible<T1, U1>::value &&
           is_constructible<T2, U2>::value;
  }
};

template<class T1, class T2>
struct pair_like {
  pair_like(pair_like const &);
  pair_like(pair_like &&);

  template<class U1,
           class U2,
           enable_if_t<construction_check<T1, T2>::template viable<U1, U2>(),
                       int> = 0>
  pair_like(U1 &&, U2 &&)
  {
  }
};

}
