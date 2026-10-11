template<bool B>
struct integral_constant { static constexpr bool value = B; };
using true_type = integral_constant<true>;
using false_type = integral_constant<false>;

template<class T>
T&& declval(int);

template<class T>
T declval(long);

template<class...>
using __void_t = void;

template <bool, class T = void>
struct enable_if {};

template <class T>
struct enable_if<true, T> { typedef T type; };

template <bool B, class T = void>
using __enable_if_t = typename enable_if<B, T>::type;

template<class T, class U, class = void>
struct less_than_comparable : false_type {};

template<class T, class U>
struct less_than_comparable<T, U, __void_t<decltype(declval<T>(0) < declval<U>(0))> > :
  true_type {};

template<class T, class U, __enable_if_t<!less_than_comparable<const T*, const U*>::value, int> = 0>
bool f(const T*, const T*, const U*) { return true; }

template<class T, class U, __enable_if_t<less_than_comparable<const T*, const U*>::value, int> = 0>
bool g(const T*, const T*, const U*) { return true; }
