#ifndef CPPGM_PA27_OWNER_DEPENDENT_RESULT_MEMBER_TEMPLATE_NTTP_HELPER_H
#define CPPGM_PA27_OWNER_DEPENDENT_RESULT_MEMBER_TEMPLATE_NTTP_HELPER_H

namespace meta {

template<class T>
struct single_view {};

template<class Head, class Tail>
struct joint_view {};

template<class T0, class T1>
struct vector2 {};

template<class Seq, int Index>
struct at_c;

template<class T0, class T1>
struct at_c<vector2<T0, T1>, 0> {
  typedef T0 type;
};

template<class T0, class T1>
struct at_c<vector2<T0, T1>, 1> {
  typedef T1 type;
};

}  // namespace meta

namespace example {

template<class T, class Seq>
struct concat_view : meta::joint_view<meta::single_view<T>, Seq> {};

}  // namespace example

namespace boost { namespace function_types {

namespace detail {

template<long Flags, long CCID>
struct property_tag {};

}  // namespace detail

typedef detail::property_tag<1024, 1024> const_qualified;

template<class Seq, class Tag>
struct member_function_pointer;

template<class R, class C, class A0, class Tag>
struct member_function_pointer<
    ::example::concat_view<R, meta::vector2<C, A0> >, Tag> {
  typedef R (C::*type)(A0) const;
};

}}  // namespace boost::function_types

class interface_x
{
  struct vtable
  {
    void (*func0)(void *, int);
    void (*func1)(void *, long);

    template<class T = void *>
    struct inf0
    {
      typedef void result;
      typedef meta::vector2<T, int> params;
    };

    template<class T = void *>
    struct inf1
    {
      typedef void result;
      typedef meta::vector2<T, long> params;
    };
  };

  vtable const * ptr_vtable;
  void * ptr_that;

  template<class T>
  struct vtable_holder
  {
    static vtable const val_vtable;
  };

public:
  template<class T>
  interface_x(T & that)
      : ptr_vtable(&vtable_holder<T>::val_vtable), ptr_that(&that)
  {
  }

  vtable::inf0<>::result call(int value) const
  {
    return ptr_vtable->func0(ptr_that, value);
  }

  vtable::inf1<>::result call(long value) const
  {
    return ptr_vtable->func1(ptr_that, value);
  }
};

namespace example {

namespace ft = boost::function_types;

template<class Inf, class Tag>
struct member
{
  typedef typename ft::member_function_pointer<
      concat_view<typename Inf::result, typename Inf::params>,
      Tag>::type mem_func_ptr;
  typedef typename meta::at_c<typename Inf::params, 0>::type context;

  template<mem_func_ptr MemFuncPtr>
  static typename Inf::result wrap(void * c)
  {
    return (reinterpret_cast<context *>(c)->*MemFuncPtr)();
  }

  template<mem_func_ptr MemFuncPtr, class T0>
  static typename Inf::result wrap(void * c, T0 a0)
  {
    return (reinterpret_cast<context *>(c)->*MemFuncPtr)(a0);
  }
};

}  // namespace example

struct a_class
{
  static int seen;

  void a_func(int value) const
  {
    seen = value;
  }

  void a_func(long value) const
  {
    seen = static_cast<int>(value + 100);
  }
};

template<class T>
interface_x::vtable const interface_x::vtable_holder<T>::val_vtable = {
    &example::member<vtable::inf0<T>,
                     example::ft::const_qualified>::template wrap<&T::a_func>,
    &example::member<vtable::inf1<T>,
                     example::ft::const_qualified>::template wrap<&T::a_func>};

#endif  // CPPGM_PA27_OWNER_DEPENDENT_RESULT_MEMBER_TEMPLATE_NTTP_HELPER_H
