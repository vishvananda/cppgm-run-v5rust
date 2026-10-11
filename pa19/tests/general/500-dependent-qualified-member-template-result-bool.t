template<int N>
struct int_ {
  enum { value = N };
  typedef int_<N + 1> next;
};

template<class T>
struct next {
  typedef typename T::next type;
};

template<class T, bool UseType>
struct maybe_type {
  typedef T type;
};

template<class T>
struct maybe_type<T, true> {
  typedef typename T::type type;
};

template<class T>
struct is_applyable {
  static const bool value = false;
};

template<class Sig>
struct call;

template<class Fun, class Arg>
struct is_applyable<call<Fun(Arg)> > {
  static const bool value = true;
};

template<class T>
struct unref {
  typedef T type;
};

template<class T>
struct unref<T &> {
  typedef T type;
};

template<class T>
struct unref<const T &> {
  typedef T type;
};

struct left_ {};

template<class R>
struct when_;

template<class R, class Expr, class State, class Data,
         bool IsApplyable = is_applyable<R>::value>
struct make_if_ {
  typedef R type;
  static const bool applied = false;
};

template<class R, class Expr, class State, class Data>
struct make_if_<R, Expr, State, Data, true> {
  typedef typename when_<R>::template impl<Expr, State, Data>::result_type type;
  static const bool applied = true;
};

template<class R, class Expr, class State, class Data, long Arity>
struct make_ {
  typedef R type;
};

template<template<class> class R, class A0, class Expr, class State, class Data>
struct make_<R<A0>, Expr, State, Data, 1>
    : maybe_type<
          R<typename make_if_<A0, Expr, State, Data>::type>,
          (make_if_<A0, Expr, State, Data>::applied || false)> {};

template<int N>
struct expr {
  typedef expr<N - 1> left;
};

template<>
struct expr<0> {};

template<class T>
struct list_set;

template<class Char, class Expr>
struct list_set_impl {
  typedef typename make_<next<call<list_set<Char>(left_)> >,
                         Expr, int, int, 1>::type result_type;
};

template<class Char>
struct list_set_impl<Char, expr<0> > {
  typedef int_<1> result_type;
};

template<class T>
struct list_set {
  template<class Expr, class State, class Data>
  struct impl : list_set_impl<T, typename unref<Expr>::type> {};
};

template<class Fun, class A0>
struct call<Fun(A0)> {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename when_<A0>::template impl<Expr, State, Data>::result_type a0;
    typedef typename Fun::template impl<a0, State, Data>::result_type result_type;
  };
};

template<class Fun, class A0>
struct when_<call<Fun(A0)> > {
  template<class Expr, class State, class Data>
  struct impl : call<Fun(A0)>::template impl<Expr, State, Data> {};
};

template<>
struct when_<left_> {
  template<class Expr, class State, class Data>
  struct impl {
    typedef typename unref<Expr>::type::left result_type;
  };
};

typedef list_set<char>::impl<const expr<2> &, int, int>::result_type result_type;
