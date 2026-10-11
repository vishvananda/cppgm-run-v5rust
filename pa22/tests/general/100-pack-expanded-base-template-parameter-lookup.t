// Reduced from Boost.Fusion vector. Template parameter names from instantiated
// pack-expanded bases are not inherited member typedefs and must not shadow the
// partial specialization's own parameter pack while collecting constructor
// member references.
namespace detail {
template<unsigned long... I>
struct index_sequence {
  typedef index_sequence type;
};

template<class Left, class Right>
struct make_index_sequence_join;

template<unsigned long... Left, unsigned long... Right>
struct make_index_sequence_join<index_sequence<Left...>, index_sequence<Right...> >
    : index_sequence<Left..., (sizeof...(Left) + Right)...> {
};

template<unsigned long N>
struct make_index_sequence
    : make_index_sequence_join<typename make_index_sequence<N / 2>::type,
                               typename make_index_sequence<N - N / 2>::type> {
};

template<>
struct make_index_sequence<1> : index_sequence<0> {};

template<>
struct make_index_sequence<0> : index_sequence<> {};
}

namespace vector_detail {
struct each_elem {};

template<class T>
struct sequence_base {};

template<unsigned long I, class T>
struct store {
  store() : elem() {}

  template<class U>
  explicit store(U&& u) : elem(static_cast<U&&>(u)) {}

  T elem;
};

template<class Sequence, class... T>
struct vector_data;

template<unsigned long... I, class... T>
struct vector_data<detail::index_sequence<I...>, T...>
    : store<I, T>..., sequence_base<vector_data<detail::index_sequence<I...>, T...> > {
  vector_data() {}

  template<class... U>
  explicit vector_data(each_elem, U&&... u) : store<I, T>(static_cast<U&&>(u))... {}
};
}

template<class... T>
struct vector
    : vector_detail::vector_data<typename detail::make_index_sequence<sizeof...(T)>::type, T...> {
  typedef vector_detail::vector_data<typename detail::make_index_sequence<sizeof...(T)>::type, T...> base;

  vector() {}

  template<class... U>
  explicit vector(U&&... u) : base(vector_detail::each_elem(), static_cast<U&&>(u)...) {}
};

int main() {
  vector<int, short, long> v(1, 2, 3);
  (void)v;
  return 0;
}
