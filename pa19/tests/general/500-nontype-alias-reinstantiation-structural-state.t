// A concrete non-type alias instantiation must resolve its target from the
// current arguments instead of reusing structural metadata from an earlier
// instantiation.
template<class T, T... I>
struct integer_sequence
{
};

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
struct append_integer_sequence<integer_sequence<T, I...>,
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

template<unsigned long N>
using make_index_sequence = make_integer_sequence<unsigned long, N>;

template<unsigned long... I>
int count(integer_sequence<unsigned long, I...>)
{
  return sizeof...(I);
}

template<class R, unsigned long N>
R dispatch()
{
  return count(make_index_sequence<N>());
}

int main()
{
  if(dispatch<int, 2>() != 2) {
    return 1;
  }
  return dispatch<int, 3>() == 3 ? 0 : 2;
}
