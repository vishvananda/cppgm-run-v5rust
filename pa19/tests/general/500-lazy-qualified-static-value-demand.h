template<long N> struct number {
  static const long value = N;
};

namespace detail {
template<long N> struct iterator { static const long index = N; };
}

struct advance {
  template<class N, class I = detail::iterator<0> > struct apply {
    typedef detail::iterator<I::index + N::value> type;
  };
};
