// N3485 focus: 14.8.2 substitution of default template arguments.
// A defaulted non-type SFINAE parameter whose type depends on
// is_constructible<T, U>::value must evaluate after function-template
// deduction binds both T and U, including reference targets.
template<class A, class B>
struct is_not_same {
  static const bool value = !__is_same(A, B);
};

template<bool B>
struct bool_constant {
  static const bool value = B;
};

template<class T, class U>
struct is_constructible : bool_constant<__is_constructible(T, U)> {};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<class T>
using remove_cvref_t = __remove_cvref(T);

struct true_type {
  static const bool value = true;
};

struct false_type {
  static const bool value = false;
};

template<class...>
using expand_to_true = true_type;

template<class... Pred>
expand_to_true<enable_if_t<Pred::value>...> and_helper(int);

template<class...>
false_type and_helper(...);

template<class... Pred>
using and_t = decltype(and_helper<Pred...>(0));

template<class T>
struct leaf {
  template<class U,
           enable_if_t<and_t<is_not_same<remove_cvref_t<U>, leaf>,
                             is_constructible<T, U> >::value, int> = 0>
  explicit leaf(U&&) {}
};

struct X {};

int main()
{
  X x;
  leaf<X&&> value(static_cast<X&&>(x));
  (void)value;
  return 0;
}
