typedef unsigned long size_t;

namespace boost {
  template<bool B, class T = void>
  struct enable_if_c { typedef T type; };

  template<class T>
  struct enable_if_c<false, T> {};

  template<class T, class R = void>
  struct enable_if : enable_if_c<T::value, R> {};

  namespace detail {
    template<class T>
    class has_size_type {
      typedef char no_type;
      struct yes_type { char dummy[2]; };

      template<class C>
      static yes_type test(typename C::size_type);

      template<class C>
      static no_type test(...);

    public:
      static const bool value = sizeof(test<T>(0)) == sizeof(yes_type);
    };

    template<class C, class Enabler = void>
    struct range_size_ {
      typedef size_t type;
    };

    template<class C>
    struct range_size_<C, typename ::boost::enable_if<has_size_type<C>, void>::type> {
      typedef typename C::size_type type;
    };
  }

  template<class T>
  struct range_size : detail::range_size_<T> {};

  template<class Range>
  typename range_size<const Range>::type size(const Range&) {
    return 4;
  }
}

int main() {
  char text[] = "abc";
  return boost::size(text) == 4 ? 0 : 1;
}
