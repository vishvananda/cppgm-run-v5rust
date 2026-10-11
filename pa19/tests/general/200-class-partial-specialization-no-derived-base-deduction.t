// N3485 focus: [temp.class.spec.match] matching class partial specializations.
// A class partial-specialization template-id pattern is matched against the
// actual template-id shape. It must not fall back to derived-to-base deduction
// and select a base-pattern partial specialization through inheritance.

template<class T>
struct arity;

template<class T>
struct wrapper : T {
  static const int arity_bits = arity<T>::value;
};

template<class A>
struct base {
  typedef A type;
};

namespace detail {

template<class T>
struct arity_impl {
  static const int value = 0;
};

template<class T>
struct arity_impl<wrapper<T> > {
  static const int value = arity<T>::value + 1;
};

template<class A>
struct arity_impl<base<A> > {
  static const int value = 10;
};

}

template<class T>
struct arity {
  static const int value = detail::arity_impl<T>::value;
};

int main()
{
  wrapper<base<int> > value;
  (void)value;
  return arity<wrapper<base<int> > >::value == 11 ? 0 : 1;
}
