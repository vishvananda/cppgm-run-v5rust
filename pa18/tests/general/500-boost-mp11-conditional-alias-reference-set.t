// VALIDATION: compile-pass
// Boost.MP11 conditional aliases used during reference-member collection must
// select their branch without leaking intermediate integral constants.

namespace std {
typedef decltype(sizeof(0)) size_t;

template<class T, T V>
struct integral_constant {
  static const T value = V;
  typedef integral_constant type;
};

template<bool B>
struct bool_constant : integral_constant<bool, B> {};

typedef bool_constant<true> true_type;
typedef bool_constant<false> false_type;

template<class A, class B>
struct is_same : false_type {};

template<class A>
struct is_same<A, A> : true_type {};
}

namespace boost {
namespace mp11 {

template<class... T>
struct mp_list {};

template<bool B>
using mp_bool = std::integral_constant<bool, B>;

typedef mp_bool<true> mp_true;
typedef mp_bool<false> mp_false;

template<class T>
struct mp_identity {
  typedef T type;
};

namespace detail {
template<class L>
struct mp_size_impl;

template<template<class...> class L, class... T>
struct mp_size_impl<L<T...> > {
  typedef std::integral_constant<std::size_t, sizeof...(T)> type;
};
}

template<class L>
using mp_size = typename detail::mp_size_impl<L>::type;

template<class L>
using mp_empty = mp_bool<mp_size<L>::value == 0>;

namespace detail {
template<bool C, class T, class... E>
struct mp_if_c_impl {};

template<class T, class... E>
struct mp_if_c_impl<true, T, E...> {
  typedef T type;
};

template<class T, class E>
struct mp_if_c_impl<false, T, E> {
  typedef E type;
};
}

template<bool C, class T, class... E>
using mp_if_c = typename detail::mp_if_c_impl<C, T, E...>::type;

template<class C, class T, class... E>
using mp_if = typename detail::mp_if_c_impl<static_cast<bool>(C::value), T, E...>::type;

namespace detail {
template<class L>
struct mp_front_impl;

template<template<class...> class L, class T1, class... T>
struct mp_front_impl<L<T1, T...> > {
  typedef T1 type;
};

template<class L>
struct mp_pop_front_impl;

template<template<class...> class L, class T1, class... T>
struct mp_pop_front_impl<L<T1, T...> > {
  typedef L<T...> type;
};
}

template<class L>
using mp_front = typename detail::mp_front_impl<L>::type;

template<class L>
using mp_pop_front = typename detail::mp_pop_front_impl<L>::type;

namespace detail {
template<class L, std::size_t N>
struct mp_take_c_impl;

template<template<class...> class L, class... T>
struct mp_take_c_impl<L<T...>, 0> {
  typedef L<> type;
};

template<class L, std::size_t N>
struct mp_drop_c_impl;

template<template<class...> class L, class... T>
struct mp_drop_c_impl<L<T...>, 0> {
  typedef L<T...> type;
};

template<class L, class... T>
struct mp_push_front_impl;

template<template<class...> class L, class... U, class... T>
struct mp_push_front_impl<L<U...>, T...> {
  typedef L<T..., U...> type;
};

template<class L>
using mp_is_value_list = mp_false;

template<class L, template<class...> class P>
struct mp_count_if_impl;

template<template<class...> class L, class... T, template<class...> class P>
struct mp_count_if_impl<L<T...>, P> {
  typedef std::integral_constant<std::size_t, 0> type;
};

template<class... L>
struct mp_append_impl;

template<template<class...> class L1, class... T1, template<class...> class L2, class... T2>
struct mp_append_impl<L1<T1...>, L2<T2...> > {
  typedef L1<T1..., T2...> type;
};

struct append_value_lists {};

struct append_type_lists {
  template<class... L>
  using fn = typename mp_append_impl<L...>::type;
};
}

template<class L, std::size_t N>
using mp_take_c = typename detail::mp_take_c_impl<L, N>::type;

template<class L, std::size_t N>
using mp_drop_c = typename detail::mp_drop_c_impl<L, N>::type;

template<class L, class... T>
using mp_push_front = typename detail::mp_push_front_impl<L, T...>::type;

template<class L, template<class...> class P>
using mp_count_if = typename detail::mp_count_if_impl<L, P>::type;

template<class... L>
using mp_append = typename mp_if_c<
    (sizeof...(L) > 0 && sizeof...(L) == mp_count_if<mp_list<L...>, detail::mp_is_value_list>::value),
    detail::append_value_lists,
    detail::append_type_lists>::template fn<L...>;

template<class L, std::size_t I, class... T>
using mp_insert_c = mp_append<mp_take_c<L, I>, mp_push_front<mp_drop_c<L, I>, T...> >;

}
}

struct void_ {};

template<class Tag>
struct optional {};

template<class Spec, class Arg, class Tail = void_>
struct item {
  typedef Spec spec;
  typedef Arg arg;
  typedef Tail tail;
};

template<class Spec, class Arg, class Tail>
struct make_item {
  typedef item<Spec, Arg, typename Tail::type> type;
};

template<class Spec, class Arg, class Tail>
using make_items = boost::mp11::mp_if<std::is_same<Arg, void_>,
                                      boost::mp11::mp_identity<void_>,
                                      make_item<Spec, Arg, Tail> >;

template<class SpecSeq, class... Args>
struct make_parameter_spec_items_helper;

template<class SpecSeq>
struct make_parameter_spec_items_helper<SpecSeq> {
  typedef void_ type;
};

template<class SpecSeq, class... Args>
using make_parameter_spec_items =
    boost::mp11::mp_if<boost::mp11::mp_empty<SpecSeq>,
                       boost::mp11::mp_identity<void_>,
                       make_parameter_spec_items_helper<SpecSeq, Args...> >;

template<class SpecSeq, class A0, class... Args>
struct make_parameter_spec_items_helper<SpecSeq, A0, Args...>
    : make_items<boost::mp11::mp_front<SpecSeq>,
                 A0,
                 make_parameter_spec_items<boost::mp11::mp_pop_front<SpecSeq>, Args...> > {};

template<class S, class K>
struct insert_ {
  typedef boost::mp11::mp_insert_c<S, 0, K> type;
};

template<class List, class UsedArgs>
struct make_arg_list0 {
  typedef typename List::arg argument;
  typedef typename insert_<UsedArgs, typename List::spec>::type used_args;
  typedef typename make_arg_list0<typename List::tail, used_args>::type type;
};

template<class UsedArgs>
struct make_arg_list0<void_, UsedArgs> {
  typedef UsedArgs type;
};

struct tag_a {};
struct tag_b {};
struct tag_c {};
struct arg_a {};
struct arg_b {};
struct arg_c {};

template<class... Spec>
struct parameters {
  typedef boost::mp11::mp_list<Spec...> parameter_spec;

  template<class... Args>
  struct bind
      : make_arg_list0<
            typename make_parameter_spec_items<parameter_spec, Args...>::type,
            boost::mp11::mp_list<> > {};
};

typedef parameters<optional<tag_a>, optional<tag_b>, optional<tag_c> > signature;
typedef signature::bind<arg_a, arg_b, arg_c>::type used;

int check[boost::mp11::mp_size<used>::value == 3 ? 1 : -1];

int main()
{
  return sizeof(check) == sizeof(int) ? 0 : 1;
}
