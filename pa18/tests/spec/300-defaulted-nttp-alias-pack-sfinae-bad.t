// N3485 focus: 14.5.7 [temp.alias], 14.8.2 [temp.deduct]

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class...>
struct and_ {
  static const bool value = true;
};

template<class Head, class... Tail>
struct and_<Head, Tail...> {
  static const bool value = Head::value && and_<Tail...>::value;
};

template<class T>
struct not_ {
  static const bool value = !T::value;
};

template<class From, class To>
struct is_convertible {
  static const bool value = false;
};

template<class T>
struct is_convertible<T, T&&> {
  static const bool value = true;
};

template<class T>
struct is_convertible<T, T> {
  static const bool value = true;
};

template<class To, class From>
struct is_constructible {
  static const bool value = false;
};

template<class T>
struct is_constructible<T&&, T> {
  static const bool value = true;
};

template<class T>
struct is_constructible<T, T> {
  static const bool value = true;
};

template<bool, class... Types>
struct tuple_constraints {
  template<class... UTypes>
  using constructible = and_<is_constructible<Types, UTypes>...>;

  template<class... UTypes>
  using convertible = and_<is_convertible<UTypes, Types>...>;

  template<class... UTypes>
  static constexpr bool is_implicitly_constructible() {
    return and_<constructible<UTypes...>, convertible<UTypes...> >::value;
  }

  template<class... UTypes>
  static constexpr bool is_explicitly_constructible() {
    return and_<constructible<UTypes...>, not_<convertible<UTypes...> > >::value;
  }
};

template<class... Elements>
struct tuple_like {
  template<class... UElements>
  static constexpr bool valid_args() {
    return sizeof...(Elements) == sizeof...(UElements);
  }

  template<bool Cond>
  using constraints = tuple_constraints<Cond, Elements...>;

  template<bool Cond, class... Args>
  using explicit_ctor = enable_if_t<
      constraints<Cond>::template is_explicitly_constructible<Args...>(), bool>;

  template<class UElement, class... UElements,
           bool Valid = valid_args<UElement, UElements...>(),
           explicit_ctor<Valid, UElement, UElements...> = false>
  explicit tuple_like(UElement&&, UElements&&...) {}
};

struct S {};

int main() {
  tuple_like<S> value((S()));
  (void)value;
  return 0;
}
