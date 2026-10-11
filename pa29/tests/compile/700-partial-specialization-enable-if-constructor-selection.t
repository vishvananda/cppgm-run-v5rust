template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B>
struct bool_constant {
  static const bool value = B;
};

typedef bool_constant<true> true_type;
typedef bool_constant<false> false_type;

template<class T>
struct remove_reference { typedef T type; };

template<class T>
struct remove_reference<T&> { typedef T type; };

template<class T>
struct remove_reference<T&&> { typedef T type; };

template<class T>
struct remove_cv { typedef T type; };

template<class T>
struct remove_cv<const T> { typedef T type; };

template<class T>
struct remove_cv<volatile T> { typedef T type; };

template<class T>
struct remove_cv<const volatile T> { typedef T type; };

template<class T>
struct remove_cvref {
  typedef typename remove_cv<typename remove_reference<T>::type>::type type;
};

template<class... T>
struct wrap;

template<class Other, class DecayedOther, class = void>
struct enable_from_wrap : false_type {};

template<class Other, class... U>
struct enable_from_wrap<Other, wrap<U...>, typename enable_if<sizeof...(U) == 1>::type>
    : true_type {};

template<class... U>
struct is_this_wrap : false_type {};

template<class U>
struct is_this_wrap<wrap<U>> : true_type {};

template<class... U>
struct first;

template<class U>
struct first<U> { typedef U type; };

template<class From, class To>
struct is_convertible {
  static const bool value = __is_convertible(From, To);
};

template<class T>
struct wrap<T> {
  T t;

  template<class... U,
           typename enable_if<sizeof...(U) == 1 &&
                              !is_this_wrap<typename remove_cvref<typename first<U...>::type>::type>::value &&
                              is_convertible<typename first<U...>::type, T>::value,
                              int>::type = 0>
  wrap(U&&... u) : t(static_cast<typename first<U...>::type&&>(u)...) {}

  template<class... U,
           typename enable_if<enable_from_wrap<wrap<U...>&&, typename remove_cvref<wrap<U...>&&>::type>::value,
                              int>::type = 0>
  wrap(wrap<U...>&& other) : t(static_cast<typename first<U...>::type&&>(other.t)) {}
};

int main() {
  int x = 7;
  wrap<int const&> w(wrap<int&&>(static_cast<int&&>(x)));
  return &w.t == &x ? 0 : 1;
}
