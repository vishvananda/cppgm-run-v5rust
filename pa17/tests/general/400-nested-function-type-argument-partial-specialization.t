// A nested function type used as a template argument must keep its parameter
// type before the outer function parameter adjustment. Reduced from
// Boost.Proto callable transform signatures such as Fun(A0(B0)).

struct value {};
struct get_value {};
struct push {};

template<class T>
struct nested_arg_is_function_pointer
{
  static const int value = 0;
};

template<class R, class A0>
struct nested_arg_is_function_pointer<R (*)(A0)>
{
  static const int value = 1;
};

template<class T>
struct call
{
  static const int value = 0;
};

template<class Fun, class A0>
struct call<Fun(A0)>
{
  static const int value = nested_arg_is_function_pointer<A0>::value;
};

static_assert(call<push(get_value(value))>::value == 1, "");

int main()
{
  return call<push(get_value(value))>::value == 1 ? 0 : 1;
}
