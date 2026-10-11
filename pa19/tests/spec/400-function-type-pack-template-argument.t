// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic], 14.8.2.5 [temp.deduct.type]
// A function type argument formed from a bound result type and a bound
// parameter pack must be resolved structurally when matching a partial
// specialization.

#include "../support.h"

template<class T>
struct function_traits;

template<class R, class... P>
struct function_traits<R(P...)>
{
  typedef R result_type;
  static const int arity = sizeof...(P);
};

template<class R, class... P>
struct make_function_type
{
  typedef typename function_traits<R(P...)>::result_type result_type;
  static const int arity = function_traits<R(P...)>::arity;
};

template<class, class, class...>
struct call_impl;

template<class F, class... A, class Head, class... Tail>
struct call_impl<F, void(A...), Head, Tail...>
    : call_impl<F, void(A..., Head), Tail...>
{
};

template<class F, class... A, class Context>
struct call_impl<F, void(A...), Context>
{
  template<class T>
  struct nested_result
  {
    typedef T type;
  };

  typedef typename nested_result<int>::type type;

  static type call(F, A..., Context)
  {
    return sizeof...(A);
  }
};

template<class Signature>
struct call_result;

template<class This, class F, class... A>
struct call_result<This(F, A...)>
    : call_impl<F, void(), A...>
{
};

typedef call_result<void(int &, bool &, char &)> concrete_call_result;
typedef decltype(&concrete_call_result::call) concrete_call_pointer;

static_assert(sizeof(concrete_call_result) == 1,
              "the recursive result class must be complete");
static_assert(is_same<concrete_call_pointer,
                      int (*)(int &, bool &, char &)>::value,
              "the inherited static call must survive full collection");

int main()
{
  typedef make_function_type<int, char, long> traits;
  return is_same<traits::result_type, int>::value && traits::arity == 2 ? 0 : 1;
}
