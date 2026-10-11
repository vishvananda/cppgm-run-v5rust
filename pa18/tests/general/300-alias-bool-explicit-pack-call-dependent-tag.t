// VALIDATION: compile-pass

namespace traits
{

template<bool Value>
struct bool_constant
{
  static const bool value = Value;
};

template<bool Value>
using bool_constant_t = bool_constant<Value>;

struct invoke_other
{
};

struct invoke_memfun_ref
{
};

struct invoke_memfun_deref
{
};

struct invoke_memobj_ref
{
};

struct invoke_memobj_deref
{
};

template<class T>
T&& declval();

template<class Type, class Tag>
struct result_success
{
  typedef Tag invoke_type;
};

struct result_failure
{
};

struct result_other_impl
{
  template<class Function, class... Args>
  static result_success<
      decltype(declval<Function>()(declval<Args>()...)), invoke_other>
  test(int);

  template<class...>
  static result_failure test(...);
};

template<bool IsMemberObject, bool IsMemberFunction,
         class Function, class... Args>
struct result_impl
{
  typedef result_failure type;
};

template<class Function, class... Args>
struct result_impl<false, false, Function, Args...> : private result_other_impl
{
  typedef decltype(test<Function, Args...>(0)) type;
};

template<class Function, class... Args>
struct invoke_result
  : result_impl<false, false, Function, Args...>::type
{
};

template<class Function, class... Args>
constexpr bool call_is_nt(invoke_other)
{
  return noexcept(declval<Function>()(declval<Args>()...));
}

template<class Function, class Object, class... Args>
constexpr bool call_is_nt(invoke_memfun_ref)
{
  return true;
}

template<class Function, class Object, class... Args>
constexpr bool call_is_nt(invoke_memfun_deref)
{
  return true;
}

template<class Function, class Object>
constexpr bool call_is_nt(invoke_memobj_ref)
{
  return true;
}

template<class Function, class Object>
constexpr bool call_is_nt(invoke_memobj_deref)
{
  return true;
}

template<class Result, class Function, class... Args>
struct call_is_nothrow
  : bool_constant_t<
      traits::call_is_nt<Function, Args...>(typename Result::invoke_type{})>
{
};

template<class Function, class... Args>
using call_is_nothrow_t =
    call_is_nothrow<invoke_result<Function, Args...>, Function, Args...>;

template<class Function, class... Args>
struct is_nothrow_invocable : call_is_nothrow_t<Function, Args...>
{
};

}

struct callable
{
  template<class... Args>
  void operator()(Args&&...);
};

int main()
{
  static_assert(!traits::is_nothrow_invocable<
                    callable&, int&, double&>::value,
                "dependent tag call");
  return 0;
}
