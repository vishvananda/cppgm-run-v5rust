template<int N>
struct int_ {
  static int const value = N;
  typedef int_<N + 1> next;
};

template<class T>
struct nested_type {
  typedef typename T::type type;
};

template<class T, bool Applied>
struct nested_type_if {
  typedef T type;
};

template<class T>
struct nested_type_if<T, true> : nested_type<T> {};

template<class T>
struct is_applyable {
  static bool const value = false;
};

template<class T>
struct uncvref {
  typedef T type;
};

template<class T>
struct arity {
  static long const value = 0;
};

template<template<class> class R, class A0>
struct arity<R<A0> > {
  static long const value = 1;
};

template<class R, class Expr, class State, class Data,
         long Arity = arity<R>::value>
struct make_ {
  typedef R type;
};

template<class R, class Expr, class State, class Data,
         bool Apply = is_applyable<R>::value>
struct make_if_ : make_<R, Expr, State, Data> {};

template<class R, class Expr, class State, class Data>
struct make_if_<R, Expr, State, Data, true>
    : uncvref<typename R::template impl<Expr, State, Data>::result_type> {
  static bool const applied = true;
};

template<class T>
struct next {
  typedef typename T::next type;
};

template<class T>
struct arity<next<T> > {
  static long const value = 1;
};

template<template<class> class R, class A0, class Expr, class State, class Data>
struct make_<R<A0>, Expr, State, Data, 1>
    : nested_type_if<R<typename make_if_<A0, Expr, State, Data>::type>,
                     (make_if_<A0, Expr, State, Data>::applied || false)> {};

template<class Left, class Right>
struct comma_expr {
  typedef Left left_type;
};

template<class Right>
struct assign_expr {};

struct left {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename Expr::left_type result_type;
  };
};

template<class Object>
struct make {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename make_if_<Object, Expr, State, Data>::type result_type;
  };
};

template<class Fun>
struct call {};

template<class Fun, class A0>
struct call<Fun(A0)> {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename A0::template impl<Expr, State, Data>::result_type a0;
    typedef typename Fun::template impl<a0, State, Data>::result_type result_type;
  };
};

template<class Fun>
struct is_applyable<call<Fun> > {
  static bool const value = true;
};

template<class Char>
struct ListSet {
  template<class Expr, class State, class Data>
  struct impl;
};

template<class Char, class Expr, class State, class Data>
struct list_set_impl;

typedef int_<1> one;

template<class Char, class Right, class State, class Data>
struct list_set_impl<Char, assign_expr<Right>, State, Data>
    : make<one>::template impl<assign_expr<Right>, State, Data> {};

template<class Char, class L, class R, class State, class Data>
struct list_set_impl<Char, comma_expr<L, R>, State, Data>
    : make<next<call<ListSet<Char>(left)> > >::template impl<
          comma_expr<L, R>, State, Data> {};

template<class Char>
template<class Expr, class State, class Data>
struct ListSet<Char>::impl : list_set_impl<Char, Expr, State, Data> {};

struct state;
struct data;

typedef comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<
    comma_expr<assign_expr<int>, int>, int>, int>, int>, int>, int>, int>,
    int>, int> ten;

typedef ListSet<int>::impl<ten, state, data>::result_type result_type;

int main()
{
  return result_type::value == 10 ? 0 : 1;
}
