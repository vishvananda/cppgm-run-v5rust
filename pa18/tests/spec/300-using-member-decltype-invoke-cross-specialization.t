// VALIDATION: compile-pass
// N3485 focus: 7.1.6.2 [dcl.type.simple], 14.5.5 [temp.class.spec],
// 14.8 [temp.fct.spec]
// Resolving a using-declaration member with a dependent decltype operand must
// select the callable specialization for the current class-template pack.

template<class...>
struct make_void
{
  typedef void type;
};

template<class... T>
using void_t = typename make_void<T...>::type;

template<class T>
T&& declval();

struct identity
{
  template<class T>
  T&& operator()(T&&) const;
};

template<class, class... Args>
struct invoke_result_impl
{
};

template<class... Args>
struct invoke_result_impl<
    void_t<decltype(__builtin_invoke(declval<Args>()...))>, Args...>
{
  using type = decltype(__builtin_invoke(declval<Args>()...));
};

template<class... Args>
using invoke_result_t = typename invoke_result_impl<void, Args...>::type;

template<class A, class B>
struct is_same
{
  static const bool value = false;
};

template<class A>
struct is_same<A, A>
{
  static const bool value = true;
};

typedef invoke_result_t<identity&, const char&> char_result;
typedef invoke_result_t<identity&, const int&> int_result;

static_assert(is_same<char_result, const char&>::value, "char result");
static_assert(is_same<int_result, const int&>::value, "int result");

int main()
{
  return 0;
}
