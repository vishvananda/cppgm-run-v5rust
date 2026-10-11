// VALIDATION: compile-pass
// Reference-member collection must not eagerly instantiate a dependent
// variadic member-template return type while looking up a nested bind template.

struct void_;

template<class... T>
struct list {};

template<class T, T V>
struct integral_constant {
  static const T value = V;
  typedef integral_constant type;
};

typedef integral_constant<bool, true> true_type;
typedef integral_constant<bool, false> false_type;

template<bool C, class T, class F>
struct if_c { typedef T type; };

template<class T, class F>
struct if_c<false, T, F> { typedef F type; };

template<class A, class B>
struct is_same : false_type {};

template<class A>
struct is_same<A, A> : true_type {};

template<class T>
struct identity { typedef T type; };

template<class L>
struct size;

template<template<class...> class L, class... T>
struct size<L<T...> > : integral_constant<unsigned long, sizeof...(T)> {};

template<class L>
struct empty : integral_constant<bool, size<L>::value == 0> {};

template<class L>
struct front;

template<template<class...> class L, class T, class... Rest>
struct front<L<T, Rest...> > { typedef T type; };

template<class L>
struct pop_front;

template<template<class...> class L, class T, class... Rest>
struct pop_front<L<T, Rest...> > { typedef L<Rest...> type; };

template<class Spec, class Arg, class Tail>
struct item {
  typedef Arg arg;
  typedef Tail tail;
};

template<class Spec, class Arg, class Tail>
struct make_item {
  typedef item<Spec, Arg, typename Tail::type> type;
};

template<class Spec, class Arg, class Tail>
using make_items = typename if_c<is_same<Arg, void_>::value,
                                 identity<void_>,
                                 make_item<Spec, Arg, Tail> >::type;

template<class SpecSeq, class... Args>
struct make_parameter_spec_items_helper;

template<class SpecSeq>
struct make_parameter_spec_items_helper<SpecSeq> {
  typedef void_ type;
};

template<class SpecSeq, class A0, class... Args>
struct make_parameter_spec_items_helper<SpecSeq, A0, Args...>
    : make_items<typename front<SpecSeq>::type,
                 A0,
                 typename if_c<empty<typename pop_front<SpecSeq>::type>::value,
                               identity<void_>,
                               make_parameter_spec_items_helper<
                                   typename pop_front<SpecSeq>::type,
                                   Args...> >::type> {};

template<class SpecSeq, class... Args>
using make_parameter_spec_items =
    typename if_c<empty<SpecSeq>::value,
                  identity<void_>,
                  make_parameter_spec_items_helper<SpecSeq, Args...> >::type;

template<class List>
struct make_arg_list0 {
  typedef typename List::arg argument;
  typedef list<argument> type;
};

template<class List>
struct make_arg_list
    : if_c<is_same<List, void_>::value,
           identity<list<> >,
           make_arg_list0<List> >::type {};

template<class... Spec>
struct parameters {
  typedef list<Spec...> parameter_spec;

  template<class A0, class... Args>
  typename make_arg_list<
      typename make_parameter_spec_items<parameter_spec, A0, Args...>::type>::type
  operator()(A0&&, Args&&...) const {
    return typename make_arg_list<
        typename make_parameter_spec_items<parameter_spec, A0, Args...>::type>::type();
  }
};

struct spec0 {};
struct spec1 {};
struct arg0 {};
struct base {};

struct parameters_with_bind : parameters<spec0, spec1> {
  template<class A0, class... Args>
  struct bind {
    typedef base type;
  };
};

typedef parameters_with_bind signature;

template<class T>
struct holder : signature::template bind<T>::type {
  T value;
};

int main()
{
  holder<arg0> h;
  (void)h.value;
  return 0;
}
