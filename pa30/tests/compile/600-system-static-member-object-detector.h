namespace std {
template<bool B> struct bool_value { static const bool value = B; };
template<class A, class P, class = void>
struct __has_destroy : bool_value<false> {};
template<class A, class P>
struct __has_destroy<A, P,
    decltype((void)((A*)0)->destroy((P)0))> : bool_value<true> {};
template<bool, class T = int> struct enable_if_destroy {};
template<class T> struct enable_if_destroy<true, T> { typedef T type; };
template<class A> struct destroy_traits {
  template<class T, typename enable_if_destroy<__has_destroy<A, T*>::value>::type = 0>
  static char select(A&, T*);
  template<class T, typename enable_if_destroy<!__has_destroy<A, T*>::value>::type = 0>
  static long select(A&, T*);
};
}
