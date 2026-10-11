template<class T> struct remove_const { typedef T type; };
template<class T> struct remove_const<T const> { typedef T type; };
template<class T> struct remove_reference { typedef T type; };
template<class T> struct remove_reference<T&> { typedef T type; };

namespace mp11 {
struct mp_false { static const bool value = false; };
struct mp_true { static const bool value = true; };

namespace detail {
template<bool C, class T, class... E> struct mp_if_c_impl;
template<class T, class... E>
struct mp_if_c_impl<true, T, E...> { typedef T type; };
template<class T, class E>
struct mp_if_c_impl<false, T, E> { typedef E type; };
}

template<class C, class T, class... E>
using mp_if = typename detail::mp_if_c_impl<C::value, T, E...>::type;

template<class... T> struct mp_list {};

namespace detail {
template<class L, template<class...> class B> struct mp_rename_impl;
template<template<class...> class L, class... T, template<class...> class B>
struct mp_rename_impl<L<T...>, B> { typedef B<T...> type; };
}

template<class Q, class L>
using mp_apply_q = typename detail::mp_rename_impl<L, Q::template fn>::type;
}

template<class A, class B> struct is_same { static const bool value = false; };
template<class A> struct is_same<A, A> { static const bool value = true; };

namespace parameter {
struct void_ {};

namespace aux {
struct empty_arg_list {
  struct binding {
    template<class KW, class Default, class Reference>
    using fn = Default;
  };
};

template<class Key, class T>
struct tagged_argument {
  typedef Key key_type;
  typedef T value_type;
};

template<class TaggedArg, class Next = empty_arg_list,
         class EmitsErrors = mp11::mp_true>
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
        mp11::mp_apply_q<next_binding,
                         mp11::mp_list<KW, Default, Reference> > >;
  };
};

template<class Keyword, class TaggedArg, class EmitsErrors = mp11::mp_true>
struct flat_like_arg_tuple {};

template<class... ArgTuples> struct arg_list_cons;
template<> struct arg_list_cons<> { typedef empty_arg_list type; };
template<class ArgTuple0, class... Tuples>
struct arg_list_cons<ArgTuple0, Tuples...>;

template<class Keyword, class TaggedArg, class EmitsErrors, class... Tuples>
struct arg_list_cons<flat_like_arg_tuple<Keyword, TaggedArg, EmitsErrors>,
                     Tuples...> {
  typedef arg_list<TaggedArg,
                   typename arg_list_cons<Tuples...>::type,
                   EmitsErrors> type;
};

template<class... ArgTuples>
struct flat_like_arg_list : arg_list_cons<ArgTuples...>::type {};
}

template<class Parameters, class Keyword, class Default = void_>
struct binding {
  typedef typename mp11::mp_apply_q<
      typename Parameters::binding,
      mp11::mp_list<Keyword, Default, mp11::mp_true> > type;
};
}

struct requested_tag {};
struct other_tag {};
struct requested_feature {};
struct requested_type { typedef int result_type; };

template<class Args, class Feature>
struct extractor_result {
  typedef typename parameter::binding<
      typename remove_const<
          typename remove_reference<Args>::type>::type,
      requested_tag>::type requested;
  typedef typename requested::result_type type;
};

template<class Feature>
struct extractor {
  template<class Arg>
  typename extractor_result<Arg, Feature>::type operator()(Arg const&) const;
};

extractor<requested_feature> const extract = {};

template<class Args>
void use(Args const& args) {
  int value = extract(args);
  (void)value;
}

typedef parameter::aux::flat_like_arg_list<
    parameter::aux::flat_like_arg_tuple<
        other_tag,
        parameter::aux::tagged_argument<other_tag, double const>,
        mp11::mp_true>,
    parameter::aux::flat_like_arg_tuple<
        requested_tag,
        parameter::aux::tagged_argument<requested_tag, requested_type>,
        mp11::mp_true> >
    args_type;

int main() {
  args_type args;
  use(args);
}
