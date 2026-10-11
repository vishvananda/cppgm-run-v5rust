// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], 14.5.2 [temp.mem]

typedef unsigned long size_t;

template<class T, T... I>
struct integer_sequence
{
};

template<size_t... I>
using index_sequence = integer_sequence<size_t, I...>;

template<bool C, class T, class E>
struct if_c_impl;

template<class T, class E>
struct if_c_impl<true, T, E>
{
  typedef T type;
};

template<class T, class E>
struct if_c_impl<false, T, E>
{
  typedef E type;
};

template<bool C, class T, class E>
using if_c = typename if_c_impl<C, T, E>::type;

template<class T>
struct identity
{
  typedef T type;
};

template<class S1, class S2>
struct append_integer_sequence;

template<class T, T... I, T... J>
struct append_integer_sequence<
    integer_sequence<T, I...>,
    integer_sequence<T, J...> >
{
  typedef integer_sequence<T, I..., (J + sizeof...(I))...> type;
};

template<class T, T N>
struct make_integer_sequence_impl;

template<class T, T N>
struct make_integer_sequence_impl_
{
  static T const M = N / 2;
  static T const R = N % 2;
  typedef typename make_integer_sequence_impl<T, M>::type S1;
  typedef typename append_integer_sequence<S1, S1>::type S2;
  typedef typename make_integer_sequence_impl<T, R>::type S3;
  typedef typename append_integer_sequence<S2, S3>::type type;
};

template<class T, T N>
struct make_integer_sequence_impl:
  if_c<N == 0,
       identity<integer_sequence<T> >,
       if_c<N == 1,
            identity<integer_sequence<T, 0> >,
            make_integer_sequence_impl_<T, N> > >
{
};

template<class T, T N>
using make_integer_sequence = typename make_integer_sequence_impl<T, N>::type;

template<size_t N>
using make_index_sequence = make_integer_sequence<size_t, N>;

template<class... T>
using index_sequence_for = make_index_sequence<sizeof...(T)>;

template<class R>
struct result_type
{
};

struct fun
{
};

struct args
{
};

template<class... A>
struct list
{
  template<class R, class F, class A2, size_t... I>
  int call_impl(result_type<R>, F &, A2 &, index_sequence<I...>)
  {
    return sizeof...(I);
  }

  template<class R, class F, class A2>
  int operator()(result_type<R>, F & f, A2 & a2)
  {
    return call_impl(result_type<R>(), f, a2, index_sequence_for<A...>());
  }
};

int main()
{
  list<int, int> l;
  fun f;
  args a;
  return l(result_type<void>(), f, a) == 2 ? 0 : 1;
}
