// HHC-156
template <bool, class T = void>
struct enable_if {};

template <class T>
struct enable_if<true, T> { typedef T type; };

template <bool B, class T = void>
using __enable_if_t = typename enable_if<B, T>::type;

template<class T1, class T2>
struct pair {
  template <class U1,
            class U2,
            __enable_if_t<__is_same(U1, U1) && __is_same(U2, U2)> = 0>
  pair(U1, U2) {}
};
