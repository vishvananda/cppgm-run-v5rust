template <class Tail>
struct cons {
  typedef Tail tail_type;
};

struct nil {};

namespace detail {
template <class T>
struct arity_impl {
  static const int value = 0;
};
}

template <class T>
struct tuple_arity {
  static const int value =
      detail::arity_impl<int>::value |
      tuple_arity<typename T::tail_type>::value;
};

template <>
struct tuple_arity<nil> {
  static const int value = 0;
};

namespace detail {
template <class Tail>
struct arity_impl<cons<Tail> > {
  static const int value = tuple_arity<cons<Tail> >::value;
};
}

template <class T>
struct arity {
  static const int value = detail::arity_impl<T>::value;
};

typedef cons<nil> expr;

template <int>
struct check {};

int main() {
  check<arity<expr>::value> c;
  (void)c;
  return 0;
}
