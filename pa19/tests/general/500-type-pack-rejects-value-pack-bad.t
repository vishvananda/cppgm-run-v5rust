// VALIDATION: compile-fail

template<class... T>
struct all {
  static const bool value = true;
};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class T>
struct trait {
  static const bool value = true;
};

template<class Type, class>
using helper_imp = Type;

template<class Type, class... Keys>
using helper =
    helper_imp<Type,
               typename enable_if<all<trait<Keys>::value...>::value>::type>;

helper<int, int> x;

int main()
{
  return 0;
}
