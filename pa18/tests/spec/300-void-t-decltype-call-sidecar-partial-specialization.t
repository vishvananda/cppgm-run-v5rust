// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.6.2 [temp.dep],
// 14.8 [temp.fct.spec]
// A dependent decltype expression retained under a template-id argument must
// be reevaluated after the enclosing partial specialization deduces its pack.

template<class T> T rvalue();
template<class T> T& lvalue();

template<int> struct requirements {};

struct error {};

template<class T> T declval();

template<class...> struct make_void { typedef void type; };

template<class... T>
using void_t = typename make_void<T...>::type;

template<class Marker, class... Args>
struct invoke_result_impl {};

template<class... Args>
struct invoke_result_impl<
    void_t<decltype(__builtin_invoke(declval<Args>()...))>, Args...>
{
  typedef decltype(__builtin_invoke(declval<Args>()...)) type;
};

template<class... Args>
using invoke_result_t =
    typename invoke_result_impl<void, Args...>::type;

template<class F, class Bound>
struct bind_return
{
  typedef invoke_result_t<F&, Bound&> type;
};

template<class> struct result_of;

template<class F, class... Args>
struct result_of<F(Args...)>
    : invoke_result_impl<void, F, Args...> {};

template<class F>
using result_of_t = typename result_of<F>::type;

template<class F, class Bound>
struct target
{
  template<class Arg>
  typename bind_return<F, Bound>::type operator()(Arg&&);
};

template<class T>
struct binder
{
  template<class... Args>
  result_of_t<T(Args...)> operator()(Args&&...) &&;
};

typedef target<void (*)(int*), int*> target_type;
typedef binder<target_type> handler_type;
typedef requirements<
    sizeof(rvalue<handler_type>()(lvalue<const error>()), char(0))>
    type_check;

static_assert(sizeof(type_check) == 1, "");

int main()
{
  return 0;
}
