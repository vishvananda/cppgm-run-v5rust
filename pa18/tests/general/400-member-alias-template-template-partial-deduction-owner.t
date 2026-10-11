namespace boost {
namespace mp11 {
template<class... T> struct mp_list {};
struct mp_false { static const bool value = false; };
struct mp_true { static const bool value = true; };

namespace detail {
template<bool C, class T, class... E> struct mp_if_c_impl {};
template<class T, class... E> struct mp_if_c_impl<true, T, E...> { typedef T type; };
template<class T, class E> struct mp_if_c_impl<false, T, E> { typedef E type; };
}

template<class C, class T, class... E>
using mp_if = typename detail::mp_if_c_impl<static_cast<bool>(C::value), T, E...>::type;

namespace detail {
template<template<class...> class F, class... T>
struct mp_valid_impl {
  template<template<class...> class G, class = G<T...> > static mp_true check(int);
  template<template<class...> class> static mp_false check(...);
  typedef decltype(check<F>(0)) type;
};

template<template<class...> class F, class... T>
struct mp_defer_impl {
  typedef F<T...> type;
};

struct mp_no_type {};
}

template<template<class...> class F, class... T>
using mp_valid = typename detail::mp_valid_impl<F, T...>::type;

template<template<class...> class F, class... T>
using mp_defer = mp_if<mp_valid<F, T...>, detail::mp_defer_impl<F, T...>, detail::mp_no_type>;

namespace detail {
template<class L, template<class...> class B> struct mp_rename_impl {};

template<template<class...> class L, class... T, template<class...> class B>
struct mp_rename_impl<L<T...>, B> : mp_defer<B, T...> {};
}

template<class Q, class L>
using mp_apply_q = typename detail::mp_rename_impl<L, Q::template fn>::type;
}

template<class A, class B> struct is_same { static const bool value = false; };
template<class A> struct is_same<A, A> { static const bool value = true; };

namespace parameter {
struct void_ {};

template<class Parameters, class Keyword, class Default>
struct value_type0 {
  typedef mp11::mp_apply_q<
      typename Parameters::binding,
      mp11::mp_list<Keyword, Default, mp11::mp_false> > type;
};

template<class Parameters, class Keyword, class Default = void_>
struct value_type {
  typedef typename value_type0<Parameters, Keyword, Default>::type type;
};

namespace aux {
struct empty_arg_list {
  struct binding {
    template<class KW, class Default, class Reference>
    using fn = Default;
  };
};

template<class TaggedArg, class Next = empty_arg_list>
struct arg_list : Next {
  typedef typename TaggedArg::key_type key_type;
  typedef typename TaggedArg::value_type value_type;
  typedef value_type reference;

  struct binding {
    typedef typename Next::binding next_binding;

    template<class KW, class Default, class Reference>
    using fn = mp11::mp_if<
        is_same<KW, key_type>,
        mp11::mp_if<Reference, reference, value_type>,
        mp11::mp_apply_q<next_binding, mp11::mp_list<KW, Default, Reference> > >;
  };
};

template<class Key, class T>
struct tagged_argument {
  typedef Key key_type;
  typedef T value_type;
};
}
}
}

struct visitor_tag {};
struct color_tag {};
struct visitor_type {};

struct args
    : boost::parameter::aux::arg_list<
          boost::parameter::aux::tagged_argument<visitor_tag, visitor_type> > {
};

typedef boost::parameter::value_type<args, color_tag, void>::type missing_color_type;
typedef boost::parameter::value_type<args, visitor_tag, void>::type visitor_lookup_type;

int main()
{
  return boost::is_same<missing_color_type, void>::value ? 0 : 1;
}
