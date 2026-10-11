// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], 14.8.2 [temp.deduct]
// Every source-dependent alias parameter must remain represented in a typed
// structural pattern before that pattern can be reused for concrete SFINAE.

namespace std {
template<class T>
T&& declval();

template<class T, T V>
struct integral_constant {
  static const T value = V;
  typedef integral_constant type;
};

template<class A, class B>
struct is_same : integral_constant<bool, false> {};

template<class A>
struct is_same<A, A> : integral_constant<bool, true> {};
}

namespace boost {
namespace mp11 {

template<bool B>
using mp_bool = std::integral_constant<bool, B>;

typedef mp_bool<true> mp_true;
typedef mp_bool<false> mp_false;

template<class T>
using mp_to_bool = mp_bool<static_cast<bool>(T::value)>;

namespace detail {
template<bool C, class T, class... E>
struct mp_if_c_impl {};

template<class T, class... E>
struct mp_if_c_impl<true, T, E...> { typedef T type; };

template<class T, class E>
struct mp_if_c_impl<false, T, E> { typedef E type; };
}

template<class C, class T, class... E>
using mp_if = typename detail::mp_if_c_impl<
    static_cast<bool>(C::value), T, E...>::type;

namespace detail {
template<template<class...> class F, class... T>
struct mp_valid_impl {
  template<template<class...> class G, class = G<T...> >
  static mp_true check(int);

  template<template<class...> class>
  static mp_false check(...);

  typedef decltype(check<F>(0)) type;
};
}

template<template<class...> class F, class... T>
using mp_valid = typename detail::mp_valid_impl<F, T...>::type;

namespace detail {
template<template<class...> class F, class... T>
struct mp_defer_impl { typedef F<T...> type; };

struct mp_no_type {};
}

template<template<class...> class F, class... T>
using mp_defer = mp_if<mp_valid<F, T...>, detail::mp_defer_impl<F, T...>,
                       detail::mp_no_type>;

namespace detail {
template<bool C, class T, template<class...> class F, class... U>
struct mp_eval_if_c_impl;

template<class T, template<class...> class F, class... U>
struct mp_eval_if_c_impl<true, T, F, U...> { typedef T type; };

template<class T, template<class...> class F, class... U>
struct mp_eval_if_c_impl<false, T, F, U...> : mp_defer<F, U...> {};
}

template<class C, class T, template<class...> class F, class... U>
using mp_eval_if = typename detail::mp_eval_if_c_impl<
    static_cast<bool>(C::value), T, F, U...>::type;

namespace detail {
template<class C, class T, class... E>
struct mp_cond_impl;
}

template<class C, class T, class... E>
using mp_cond = typename detail::mp_cond_impl<C, T, E...>::type;

namespace detail {
template<class C, class T, class... E>
using mp_cond_next = mp_eval_if<C, T, mp_cond, E...>;

template<class C, class T, class... E>
struct mp_cond_impl : mp_defer<mp_cond_next, C, T, E...> {};

template<class... T>
struct mp_or_impl;

template<>
struct mp_or_impl<> { typedef mp_false type; };

template<class T>
struct mp_or_impl<T> { typedef T type; };
}

template<class... T>
using mp_or = mp_to_bool<typename detail::mp_or_impl<T...>::type>;

namespace detail {
template<class T1, class... T>
struct mp_or_impl<T1, T...> {
  typedef mp_eval_if<T1, T1, mp_or, T...> type;
};
}

}
}

namespace lib {
struct tag;
template<class T> struct to_tag;
template<class T> struct try_to_tag;
struct output;

namespace detail {
template<class... Args>
using supports_tag_invoke = decltype(tag_invoke(std::declval<Args>()...));

template<class T>
using has_user_from = supports_tag_invoke<tag, output&, T&&>;

struct direction {};

template<class T, class Direction>
using has_user = boost::mp11::mp_if<
    std::is_same<Direction, direction>,
    boost::mp11::mp_valid<has_user_from, T>,
    boost::mp11::mp_false>;

template<class Context, class T>
using has_context_from =
    supports_tag_invoke<tag, output&, T&&, Context const&>;

template<class Context, class T>
using has_context_to =
    supports_tag_invoke<to_tag<T>, output const&, Context const&>;

template<class Context, class T>
using has_nonthrowing_context_to =
    supports_tag_invoke<try_to_tag<T>, output const&, Context const&>;

template<class Context, class T, class Direction>
using has_context = boost::mp11::mp_if<
    std::is_same<Direction, direction>,
    boost::mp11::mp_valid<has_context_from, Context, T>,
    boost::mp11::mp_or<
        boost::mp11::mp_valid<has_context_to, Context, T>,
        boost::mp11::mp_valid<has_nonthrowing_context_to, Context, T>>>;

template<class Context, class T>
using has_full_context_from =
    supports_tag_invoke<tag, output&, T&&, Context const&, Context const&>;

template<class Context, class T, class Direction>
using has_full_context = boost::mp11::mp_if<
    std::is_same<Direction, direction>,
    boost::mp11::mp_valid<has_full_context_from, Context, T>,
    boost::mp11::mp_false>;

struct user_tag {};
struct context_tag : user_tag {};
struct full_context_tag : context_tag {};

template<class Context, class T, class Direction>
using category = boost::mp11::mp_cond<
    has_full_context<Context, T, Direction>, full_context_tag,
    has_context<Context, T, Direction>, context_tag,
    has_user<T, Direction>, user_tag>;

struct context {};
}

struct tag {};
template<class T> struct to_tag {};
template<class T> struct try_to_tag {};
struct output {};
}

struct info {
  friend void tag_invoke(lib::tag, lib::output&, info const&) {}
};

using direct = boost::mp11::mp_valid<lib::detail::has_context_from,
                                     lib::detail::context,
                                     info const&>;
using selected = lib::detail::category<lib::detail::context,
                                       info const&,
                                       lib::detail::direction>;

static_assert(!direct::value, "the four-argument customization must be invalid");
static_assert(std::is_same<selected, lib::detail::user_tag>::value,
              "the category must select the three-argument customization");

int main() { return 0; }
