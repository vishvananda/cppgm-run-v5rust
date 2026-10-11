// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], 14.5.7 [temp.alias]

template<bool B, class T = void>
struct enable_if
{
  typedef T type;
};

template<class T>
struct enable_if<false, T>
{
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

template<bool B>
struct bool_constant
{
  static const bool value = B;
};

typedef bool_constant<true> true_type;
typedef bool_constant<false> false_type;

template<class... T>
using expand_to_true = true_type;

template<class... Pred>
expand_to_true<enable_if_t<Pred::value>...> and_helper(int);

template<class...>
false_type and_helper(...);

template<class... Pred>
using And = decltype(and_helper<Pred...>(0));

template<class T>
struct wrap
{
  typedef And<T> type;
};

int main()
{
  return 0;
}
