// VALIDATION: compile-pass
// The member-template parameter N must not be replayed as the global N while
// collecting a class template specialization's member declarations.

namespace std {
typedef unsigned long size_t;

template<class A, class B>
struct pair {
  A first;
  B second;
  pair(A a, B b) : first(a), second(b) {}
};

template<class A, class B>
pair<A, B> make_pair(A a, B b)
{
  return pair<A, B>(a, b);
}
}

typedef std::pair<unsigned long, unsigned long> Pair;
static const std::size_t N = 6;

namespace boost {
namespace tuples {

struct null_type {};

template<class HT, class TT>
struct cons;

namespace detail {
template<std::size_t I>
struct drop_front {
  template<class Tuple>
  struct apply {
    typedef typename drop_front<I - 1>::template apply<Tuple> next;
    typedef typename next::type::tail_type type;
    static type& call(Tuple& tuple) { return next::call(tuple).tail; }
  };
};

template<>
struct drop_front<0> {
  template<class Tuple>
  struct apply {
    typedef Tuple type;
    static type& call(Tuple& tuple) { return tuple; }
  };
};
}

template<std::size_t I, class T>
struct element {
  typedef typename detail::drop_front<I>::template apply<T>::type::head_type type;
};

template<class T>
struct access_traits {
  typedef T& non_const_type;
  typedef const T& parameter_type;
};

template<class T>
struct access_traits<T&> {
  typedef T& non_const_type;
  typedef T& parameter_type;
};

template<std::size_t I, class HT, class TT>
typename access_traits<typename element<I, cons<HT, TT> >::type>::non_const_type
get(cons<HT, TT>& c)
{
  typedef typename detail::drop_front<I>::template apply<cons<HT, TT> > impl;
  return impl::call(c).head;
}

template<class HT, class TT>
struct cons {
  typedef HT head_type;
  typedef TT tail_type;
  typedef cons self_type;
  head_type head;
  tail_type tail;

  cons(typename access_traits<head_type>::parameter_type h, const tail_type& t)
    : head(h), tail(t)
  {
  }

  template<std::size_t N>
  typename access_traits<typename element<N, self_type>::type>::non_const_type
  get() { return boost::tuples::get<N>(*this); }
};

template<class HT>
struct cons<HT, null_type> {
  typedef HT head_type;
  typedef null_type tail_type;
  typedef cons self_type;
  head_type head;

  cons(typename access_traits<head_type>::parameter_type h,
       const null_type& = null_type())
    : head(h)
  {
  }

  template<std::size_t N>
  typename access_traits<typename element<N, self_type>::type>::non_const_type
  get() { return boost::tuples::get<N>(*this); }
};

template<class T0, class T1>
struct tuple : cons<T0, cons<T1, null_type> > {
  typedef cons<T0, cons<T1, null_type> > inherited;

  tuple(typename access_traits<T0>::parameter_type t0,
        typename access_traits<T1>::parameter_type t1)
    : inherited(t0, cons<T1, null_type>(t1))
  {
  }

  template<class U1, class U2>
  tuple& operator=(const std::pair<U1, U2>& pair)
  {
    this->head = pair.first;
    this->tail.head = pair.second;
    return *this;
  }
};

template<class T0, class T1>
tuple<T0&, T1&> tie(T0& t0, T1& t1)
{
  return tuple<T0&, T1&>(t0, t1);
}

}
using tuples::tie;
}

int main()
{
  Pair *first = 0;
  Pair *last = 0;
  boost::tie(first, last) = std::make_pair(first, last);
  return 0;
}
