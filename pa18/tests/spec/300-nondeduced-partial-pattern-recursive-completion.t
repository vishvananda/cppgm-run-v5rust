// VALIDATION: compile-pass
// N3485 focus: 14.8.2 [temp.deduct], non-deduced dependent type matching.
// A reused typed partial-specialization pattern must receive deductions made
// by earlier arguments before recursive class completion resolves it.

template<bool V>
struct bool_
{
  enum { value };
  typedef bool_ type;
};

template<class T>
struct list;

template<class Sequence, class T>
struct contains : bool_<false>
{};

template<class T>
struct feature_tag
{
  typedef typename T::feature_tag type;
};

template<class T>
struct feature_of
{
  typedef int type;
};

template<class Feature>
struct wrapper
{
  typedef int feature_tag;
};

struct iterator_tag;
struct forward_traversal_tag;
struct random_access_traversal_tag;

template<class Derived>
struct iterator_base
{};

struct nil;

template<class Head, class Tail = nil>
struct cons
{
  typedef Head head_type;
};

template<class T>
struct iterator : iterator_base<iterator<T> >
{
  typedef iterator_tag fusion_tag;
  typedef typename T::head_type value_type;
};

template<class First, class Last, class Predicate>
struct static_find;

template<class Predicate>
struct transformed_predicate;

template<class Category, class First, class Last, class Predicate>
struct filter_iterator
    : iterator_base<filter_iterator<int, First, int, Predicate> >
{
  typedef iterator_tag fusion_tag;
  typedef typename static_find<
      First, int, transformed_predicate<Predicate> >::type first_type;
  typedef typename first_type::value_type value_type;
};

template<class T>
struct identity
{
  typedef int type;
};

struct no_tag
{
  char value[2];
};

template<class T>
struct type_wrapper;

template<class T, class Fallback = bool_<false> >
struct has_fusion_tag
{
  struct sfinae
  {
    template<class U>
    static char test(type_wrapper<U> const volatile *,
                     type_wrapper<typename U::fusion_tag> * = 0);
  };

  static const bool value =
      sizeof(sfinae::test(static_cast<type_wrapper<T> *>(0))) == sizeof(char);
};

template<bool B, class T = void>
struct enable_if_c
{
  typedef void type;
};

template<class Condition, class T = void>
struct enable_if : enable_if_c<Condition::value, T>
{};

template<class Iterator, class Active = void>
struct tag_of_impl;

template<class Iterator>
struct tag_of_impl<
    Iterator,
    typename ::enable_if<has_fusion_tag<Iterator> >::type>
{
  typedef typename Iterator::fusion_tag type;
};

template<class T>
struct remove_const
{
  typedef T type;
};

template<class Iterator, class Active = void>
struct trait_tag_of : tag_of_impl<Iterator, Active>
{};

template<class Iterator>
struct tag_of : trait_tag_of<typename remove_const<Iterator>::type>
{};

template<class Tag>
struct category_of_impl
{
  template<class Iterator>
  struct apply
  {
    typedef int type;
  };
};

template<class Iterator>
struct category_of
    : category_of_impl<typename tag_of<Iterator>::type>::template apply<Iterator>
{};

template<class Left, class Right>
struct is_same : bool_<false>
{};

template<class Iterator>
struct is_random_access
    : is_same<typename category_of<Iterator>::type,
              random_access_traversal_tag>
{};

template<class Iterator, class Predicate>
struct apply_filter;

template<class First, class Last, class Predicate>
struct static_find
{
  enum {
    value = is_random_access<iterator<cons<wrapper<int> > > >::value ||
            apply_filter<iterator<cons<wrapper<int> > >, Predicate>::value
  };
  typedef iterator<cons<wrapper<int> > > type;
};

template<class Tag>
struct value_of_impl;

template<>
struct value_of_impl<iterator_tag>
{
  template<class Iterator>
  struct apply
  {
    typedef typename Iterator::value_type type;
  };
};

template<class Iterator>
struct value_of
    : value_of_impl<typename tag_of<Iterator>::type>::template apply<Iterator>
{};

template<class T>
struct has_type
{
  struct sfinae
  {
    template<class U>
    static char test(type_wrapper<U> const volatile *,
                     type_wrapper<typename U::type> * = 0);
    static no_tag test(...);
  };

  enum {
    value = sizeof(sfinae::test(static_cast<type_wrapper<T> *>(0))) ==
            sizeof(char)
  };
};

template<class T, bool HasType>
struct quote_impl : T
{};

template<class T>
struct quote_impl<T, false>
{
  typedef T type;
};

template<template<class> class F>
struct quote1
{
  template<class T>
  struct apply : quote_impl<F<T>, has_type<F<T> >::value>
  {};
};

struct arg1;
struct na;

template<class F, class A>
struct bind1;

template<class F, class U1, class U2 = na, class U3 = na, class U4 = na,
         class U5 = na>
struct apply_wrap5 : F::template apply<U1, int, int, int, int>
{};

template<class F, class U>
struct apply_wrap1 : F::template apply<U>
{};

template<class T, class U1, class U2, class U3, class U4, class U5>
struct resolve_bind_arg
{
  typedef int type;
};

template<class U1, class U2, class U3, class U4, class U5>
struct resolve_bind_arg<arg1, U1, U2, U3, U4, U5>
{
  typedef U1 type;
};

template<class F, class A, class U1, class U2, class U3, class U4, class U5>
struct resolve_bind_arg<bind1<F, A>, U1, U2, U3, U4, U5>
{
  typedef typename apply_wrap5<bind1<F, A>, U1, U2, U3, U4, U5>::type type;
};

template<class F, class A>
struct bind1
{
  template<class U1, class U2 = na, class U3 = na, class U4 = na,
           class U5 = na>
  struct apply
  {
    typedef typename apply_wrap1<
        F,
        typename resolve_bind_arg<A, U1, int, int, int, int>::type>::type type;
  };
};

template<class Features, class Accumulator>
struct contains_feature_of
{
  typedef typename contains<
      int, typename Accumulator::feature_tag>::type type;
};

template<class Features>
struct contains_feature_of_
{
  template<class Accumulator>
  struct apply : contains_feature_of<int, Accumulator>
  {};
};

template<class Predicate>
struct transformed_predicate
{
  template<class Iterator>
  struct apply
      : Predicate::template apply<typename value_of<Iterator>::type>
  {};
};

template<class Iterator, class Predicate>
struct apply_filter
{
  typedef typename Predicate::template apply<Iterator>::type type;
  enum { value };
};

template<class T>
struct nested_type_wknd : T::type
{};

template<bool C, class T>
struct or_impl : bool_<false>
{};

template<class T>
struct or_impl<false, T> : nested_type_wknd<T>
{};

template<class T>
struct consume
    : or_impl<
          false,
          apply_filter<
              filter_iterator<
                  long,
                  iterator<cons<wrapper<int> > >,
                  iterator<cons<wrapper<long> > >,
                  contains_feature_of_<int> >,
              bind1<contains_feature_of_<int>,
                    bind1<quote1<value_of>, arg1> > > >
{};

template<bool C, class T, class F>
struct if_c
{
  typedef T type;
};

template<class Condition, class T, class F>
struct eval_if : if_c<Condition::value, T, int>::type
{};

template<class T>
struct cold_find
{
  typedef typename eval_if<
      consume<T>, identity<int>, identity<long> >::type type;
};

template<class T>
struct cold_owner
{
  typedef typename cold_find<T>::type type;
};

int main()
{
  cold_owner<long>::type value = 0;
}
