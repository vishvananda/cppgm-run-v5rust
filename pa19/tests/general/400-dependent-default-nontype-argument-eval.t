template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<class T>
struct is_integral : bool_constant<false> {};

template<>
struct is_integral<long> : bool_constant<true> {};

template<class T>
struct is_enum : bool_constant<false> {};

template<class T>
struct remove_cv {
  typedef T type;
};

template<class T>
using remove_cv_t = typename remove_cv<T>::type;

template<class T, bool = is_integral<T>::value || is_enum<T>::value>
struct make_unsigned {};

template<class T>
struct make_unsigned<T, true> {
  typedef unsigned long type;
};

template<class T>
struct holder {
  typedef typename make_unsigned<remove_cv_t<T> >::type type;
};

int main() {
  holder<long>::type value = 7;
  return value == 7 ? 0 : 1;
}
