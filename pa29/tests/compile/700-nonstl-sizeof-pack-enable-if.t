template <class T, T... Values>
struct integer_sequence {
  static const unsigned long size = sizeof...(Values);
};

template <template <class T, T...> class Base, class T, T N>
using make_integer_sequence_impl = Base<T, __integer_pack(N)...>;

template <unsigned long N>
using make_index_sequence = make_integer_sequence_impl<integer_sequence, unsigned long, N>;

template <class... Args>
using index_sequence_for = make_index_sequence<sizeof...(Args)>;

template <class Seq, class... Ts>
struct tuple_impl {
  static const unsigned long size = Seq::size;
};

template <class... Ts>
struct tuple_like {
  typedef tuple_impl<index_sequence_for<Ts...>, Ts...> base_type;
};

int main()
{
  return tuple_like<int>::base_type::size == 1 ? 0 : 1;
}
