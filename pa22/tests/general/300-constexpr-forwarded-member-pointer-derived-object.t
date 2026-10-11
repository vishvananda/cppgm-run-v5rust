// VALIDATION: compile-pass
// N3485 focus: 5.2.9 [expr.static.cast], 5.5 [expr.mptr.oper], and
// 7.1.5 [dcl.constexpr]. A member pointer remains a constant value while it is
// stored in a wrapper and forwarded to base and derived object pointers.

template<class T>
struct remove_reference
{
  typedef T type;
};

template<class T>
struct remove_reference<T &>
{
  typedef T type;
};

template<class T>
struct remove_reference<T &&>
{
  typedef T type;
};

template<class T>
constexpr T && forward(typename remove_reference<T>::type & value) noexcept
{
  return static_cast<T &&>(value);
}

struct X
{
  int member = -1;

  constexpr int function() const
  {
    return member;
  }
};

struct Y : X
{
};

template<class M, class T>
struct data_wrapper
{
  M T::* pointer;

  template<class U>
  constexpr auto operator()(U && object) const
      -> decltype((*forward<U>(object)).*pointer)
  {
    return (*forward<U>(object)).*pointer;
  }
};

template<class M, class T>
constexpr data_wrapper<M, T> make_data_wrapper(M T::* pointer) noexcept
{
  return data_wrapper<M, T>{pointer};
}

template<class M, class T, class A>
constexpr auto invoke_data(M T::* pointer, A && object)
    -> decltype(make_data_wrapper(pointer)(forward<A>(object)))
{
  return make_data_wrapper(pointer)(forward<A>(object));
}

template<class R, class T>
struct function_wrapper
{
  R (T::* pointer)() const;

  template<class U>
  constexpr R operator()(U && object) const
  {
    return ((*forward<U>(object)).*pointer)();
  }
};

template<class R, class T>
constexpr function_wrapper<R, T>
make_function_wrapper(R (T::* pointer)() const) noexcept
{
  return function_wrapper<R, T>{pointer};
}

template<class R, class T, class A>
constexpr R invoke_function(R (T::* pointer)() const, A && object)
{
  return make_function_wrapper(pointer)(forward<A>(object));
}

int main()
{
  constexpr X base = {};
  constexpr Y derived = {};

  static_assert(invoke_data(&X::member, &base) == -1,
                "data member through base pointer");
  static_assert(invoke_data(&X::member, &derived) == -1,
                "data member through derived pointer after base specialization");

  static_assert(invoke_function(&X::function, &base) == -1,
                "member function through base pointer");
  static_assert(invoke_function(&X::function, &derived) == -1,
                "member function through derived pointer after base specialization");
  return 0;
}
