// VALIDATION: compile-pass
// Library-shaped Boost.MultiIndex/MP11 reduction: a class template static_assert
// may query an inherited index_type_list through mp_transform/mp_flatten while
// reference members are still being collected.

template<class... T>
struct list
{};

template<class L>
struct mp_clear_impl;

template<template<class...> class L, class... T>
struct mp_clear_impl<L<T...> >
{
  typedef L<> type;
};

template<class L>
using mp_clear = typename mp_clear_impl<L>::type;

template<template<class...> class F, class L>
struct mp_transform_impl;

template<template<class...> class F, template<class...> class L, class... T>
struct mp_transform_impl<F, L<T...> >
{
  typedef L<F<T>...> type;
};

template<template<class...> class F, class L>
using mp_transform = typename mp_transform_impl<F, L>::type;

template<class Q, class L>
using mp_transform_q = mp_transform<Q::template fn, L>;

template<class L2>
struct mp_flatten_impl
{
  template<class T>
  using fn = T;
};

template<class L, class L2 = mp_clear<L> >
using mp_flatten = mp_transform_q<mp_flatten_impl<L2>, L>;

template<class TagList>
struct no_duplicate_tags
{
  static const bool value = true;
};

template<class Index>
using index_tag_list = typename Index::tag_list;

template<class IndexList>
using no_duplicate_tags_in_index_list =
    no_duplicate_tags<mp_flatten<mp_transform<index_tag_list, IndexList> > >;

struct left_tag
{};

struct right_tag
{};

template<class Tag>
struct index
{
  typedef list<Tag> tag_list;
};

template<class IndexList>
struct base
{
  typedef IndexList index_type_list;
};

template<class IndexList>
struct container : base<IndexList>
{
  typedef typename base<IndexList>::index_type_list index_type_list;

  static_assert(no_duplicate_tags_in_index_list<index_type_list>::value, "");
};

template<class Container>
struct container_reference
{
  typedef typename Container::index_type_list type;
};

int main()
{
  typedef container<list<index<left_tag>, index<right_tag> > > container_type;
  typedef typename container_reference<container_type>::type selected_indices;
  container_type c;
  selected_indices indices;
  (void)c;
  (void)indices;
  return 0;
}
