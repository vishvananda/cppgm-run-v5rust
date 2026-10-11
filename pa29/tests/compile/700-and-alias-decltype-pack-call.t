// HHC-157
template<class T, T V>
struct integral_constant {
  typedef integral_constant type;
  static constexpr T value = V;
};

template<class A, class B>
struct is_assignable : integral_constant<bool, __is_assignable(A, B)> {};

template<class T>
struct outer {
  typedef typename is_assignable<T&, const T&>::type type;
};

outer<char>::type x;
