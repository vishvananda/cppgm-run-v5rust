// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], 14.5.3 [temp.variadic]
// A forwarded empty pack must remain empty when a concrete pack-only template
// is decomposed to select a partial specialization.

namespace meta
{
  template<class... T>
  struct type_list
  {
  };

  template<class... L>
  struct append_impl;

  template<>
  struct append_impl<>
  {
    typedef type_list<> type;
  };

  template<template<class...> class L, class... T>
  struct append_impl<L<T...> >
  {
    typedef L<T...> type;
  };

  template<class... L>
  using append = typename append_impl<L...>::type;

  template<class L>
  struct unique_impl;

  template<class... T>
  struct unique_impl<type_list<T...> >
  {
    typedef type_list<T...> type;
  };

  template<class L>
  using unique = typename unique_impl<L>::type;
}

namespace detail
{
  template<class T>
  struct traits
  {
    typedef meta::type_list<T> types;
  };

  template<class L>
  struct deduce_list;

  template<template<class...> class L, class... T>
  struct deduce_list<L<T...> >
  {
    typedef meta::unique<
        meta::append<typename traits<T>::types...>
    > type;
  };

  template<class L>
  struct tuple_impl;

  template<template<class...> class L, class... T>
  struct tuple_impl<L<T...> >
  {
    typedef meta::type_list<T...> type;
  };

  template<class... E>
  using tuple_for = typename tuple_impl<
      typename deduce_list<meta::type_list<E...> >::type
  >::type;
}

template<class... E>
struct context
{
  typedef detail::tuple_for<E...> tuple;
  tuple value;
};

int main()
{
  context<> empty;
  context<int> one;
  (void)empty;
  (void)one;
}
