// VALIDATION: compile-pass
// Reduced from Boost.Bimap through Boost.MP11. A nested alias template passed
// as a template-template argument must behave as a substitution failure when
// its target has no `type`, while the same alias remains usable through an
// mp_apply-style list expansion.

template<class T, T V>
struct integral_constant {
  static const T value = V;
};

using true_type = integral_constant<bool, true>;
using false_type = integral_constant<bool, false>;

template<class T, class U>
struct is_same : false_type {};

template<class T>
struct is_same<T, T> : true_type {};

template<class... T>
struct list {};

template<class... L>
struct append_impl {};

template<template<class...> class L, class... T>
struct append_impl<L<T...> > {
  using type = L<T...>;
};

template<template<class...> class L1, class... T1,
         template<class...> class L2, class... T2,
         class... Rest>
struct append_impl<L1<T1...>, L2<T2...>, Rest...>
    : append_impl<L1<T1..., T2...>, Rest...> {};

struct append_type_lists {
  template<class... L>
  using fn = typename append_impl<L...>::type;
};

template<class... L>
using append = append_type_lists::fn<L...>;

template<template<class...> class F, class... T>
struct valid_impl {
  template<template<class...> class G, class = G<T...> >
  static true_type check(int);

  template<template<class...> class>
  static false_type check(...);

  using type = decltype(check<F>(0));
};

template<template<class...> class F, class... T>
using valid = typename valid_impl<F, T...>::type;

template<template<class...> class F, class... T>
struct defer_impl {
  using type = F<T...>;
};

template<class L, template<class...> class B>
struct rename_impl {};

template<template<class...> class L, class... T, template<class...> class B>
struct rename_impl<L<T...>, B> : defer_impl<B, T...> {};

template<template<class...> class F, class L>
using apply = typename rename_impl<L, F>::type;

template<class L>
struct clear_impl;

template<template<class...> class L, class... T>
struct clear_impl<L<T...> > {
  using type = L<>;
};

template<class L>
using clear = typename clear_impl<L>::type;

template<class L, class T>
struct push_front_impl;

template<template<class...> class L, class... T, class U>
struct push_front_impl<L<T...>, U> {
  using type = L<U, T...>;
};

template<class L, class T>
using push_front = typename push_front_impl<L, T>::type;

template<class Q, class... T>
using invoke_q = typename Q::template fn<T...>;

template<class Q, class L>
struct transform_q_impl;

template<class Q, template<class...> class L, class... T>
struct transform_q_impl<Q, L<T...> > {
  using type = L<invoke_q<Q, T>...>;
};

template<class Q, class L>
using transform_q = typename transform_q_impl<Q, L>::type;

template<template<class...> class F, class... L>
struct transform_impl;

template<template<class...> class F, template<class...> class L, class... T>
struct transform_impl<F, L<T...> > {
  using type = L<F<T>...>;
};

template<template<class...> class F, class... L>
using transform = typename transform_impl<F, L...>::type;

template<class L>
struct size_impl;

template<template<class...> class L, class... T>
struct size_impl<L<T...> > {
  using type = integral_constant<int, sizeof...(T)>;
};

template<class L>
using size = typename size_impl<L>::type;

constexpr int cx_plus()
{
  return 0;
}

template<class T1, class... T>
constexpr int cx_plus(T1 t1, T... t)
{
  return static_cast<int>(t1) + cx_plus(t...);
}

template<int N>
using size_c = integral_constant<int, N>;

template<class L, class V>
struct count_impl;

template<template<class...> class L, class... T, class V>
struct count_impl<L<T...>, V> {
  using type = size_c<cx_plus(is_same<T, V>::value...)>;
};

template<class L, class V>
using count_actual = typename count_impl<L, V>::type;

template<bool B>
using bool_ = integral_constant<bool, B>;

template<class... T>
struct same_actual_impl;

template<>
struct same_actual_impl<> {
  using type = true_type;
};

template<class T1, class... T>
struct same_actual_impl<T1, T...> {
  using type = bool_<count_actual<list<T...>, T1>::value == sizeof...(T)>;
};

template<class... T>
using same_actual = typename same_actual_impl<T...>::type;

template<bool C, class T, class E>
struct if_c_impl {
  using type = E;
};

template<class T, class E>
struct if_c_impl<true, T, E> {
  using type = T;
};

template<class C, class T, class E>
using if_ = typename if_c_impl<C::value, T, E>::type;

struct list_size_mismatch {};

template<template<class...> class F, class... L>
struct transform_checked_impl {};

template<template<class...> class F, template<class...> class L, class... T>
struct transform_checked_impl<F, L<T...> > {
  using type = L<F<T>...>;
};

template<template<class...> class F, class... L>
using transform_checked =
    typename if_<same_actual<size<L>...>,
                 transform_checked_impl<F, L...>,
                 list_size_mismatch>::type;

template<class... T>
struct similar_impl;

template<>
struct similar_impl<> : true_type {};

template<class T>
struct similar_impl<T> : true_type {};

template<class T>
struct similar_impl<T, T> : true_type {};

template<class T1, class T2>
struct similar_impl<T1, T2> : false_type {};

template<template<class...> class L, class... T1, class... T2>
struct similar_impl<L<T1...>, L<T2...> > : true_type {};

template<class... T>
using similar = similar_impl<T...>;

template<class L2>
struct flatten_impl {
  template<class T>
  using fn = T;
};

template<class L, class L2 = clear<L> >
using flatten =
    apply<append, push_front<transform_q<flatten_impl<L2>, L>, clear<L> > >;

template<class L2>
struct flatten_boostlike_impl {
  template<class T>
  using fn = if_<similar<L2, T>, T, list<T> >;
};

template<class L, class L2 = clear<L> >
using flatten_boostlike =
    apply<append,
          push_front<transform_q<flatten_boostlike_impl<L2>, L>, clear<L> > >;

template<class T>
struct info {};

template<class... T>
struct tag {};

template<class TagList>
using tag_list = TagList;

template<class Index>
using index_tag_list = apply<list, tag_list<typename Index::tag_list> >;

template<class TagList>
struct ordered_unique {
  using tag_list = TagList;
};

template<int N, class Value, class IndexList>
struct nth_layer {};

template<class TagList, class Super>
struct random_access_index {
  using tag_list = TagList;
  using super = Super;
};

template<class TagList, class Super>
struct ordered_index {
  using tag_list = TagList;
  using super = Super;
};

template<class Value, class IndexList>
struct multi_index_like {
  using index_type_list = list<
      random_access_index<tag<long>, nth_layer<1, Value, IndexList> >,
      ordered_index<tag<int>, nth_layer<2, Value, IndexList> > >;
  using flattened_tags =
      flatten_boostlike<transform_checked<index_tag_list, index_type_list> >;
  static_assert(is_same<flattened_tags, list<long, int> >::value,
                "nested index tag lists should flatten in class context");
};

using invalid = valid<append, list<>, int, double>;
using combined = apply<append, list<list<>, list<int>, list<double> > >;
using flattened = flatten<list<list<int>, list<long>, list<info<int> > > >;
using transformed_flattened = flatten<
    transform<index_tag_list,
              list<ordered_unique<tag<int> >,
                   ordered_unique<tag<long> >,
                   ordered_unique<tag<info<int> > > > > >;
using checked_transformed_flattened = flatten<
    transform_checked<index_tag_list,
                      list<ordered_unique<tag<int> >,
                           ordered_unique<tag<long> >,
                           ordered_unique<tag<info<int> > > > > >;
using boostlike_transformed_flattened = flatten_boostlike<
    transform_checked<index_tag_list,
                      list<ordered_unique<tag<int> >,
                           ordered_unique<tag<long> >,
                           ordered_unique<tag<info<int> > > > > >;
using nested_one = multi_index_like<info<int>,
                                    list<ordered_unique<tag<int> >,
                                         ordered_unique<tag<long> > > >;
using nested_two = multi_index_like<info<long>,
                                    list<ordered_unique<tag<long> >,
                                         ordered_unique<tag<int> > > >;

static_assert(!invalid::value, "invalid append arguments should be SFINAE");
static_assert(is_same<combined, list<int, double> >::value,
              "alias-template list expansion should preserve the alias");
static_assert(is_same<flattened, list<int, long, info<int> > >::value,
              "defaulted flatten alias should expand list arguments");
static_assert(is_same<transformed_flattened, list<int, long, info<int> > >::value,
              "transformed nested tag lists should flatten through the alias");
static_assert(is_same<checked_transformed_flattened,
                      list<int, long, info<int> > >::value,
              "checked transform should still present a list to flatten");
static_assert(is_same<boostlike_transformed_flattened,
                      list<int, long, info<int> > >::value,
              "boostlike flatten should expand only similar lists");

int main()
{
  return invalid::value ? 1 : 0;
}
