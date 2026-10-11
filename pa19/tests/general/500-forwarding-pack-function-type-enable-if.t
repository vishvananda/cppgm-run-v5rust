// VALIDATION: compile-pass
// A forwarding-reference pack must expand in every function-type occurrence
// retained by a dependent enable_if return type.

template<bool B, class T = void>
struct enable_if {
};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool B, class T = void>
using enable_if_t = typename enable_if<B, T>::type;

enum overload_type {
  many_properties,
  ill_formed
};

template<class T, class Signature>
struct call_traits {
  static constexpr overload_type overload = ill_formed;
  typedef void result_type;
};

template<class T, class P0, class P1, class... PN>
struct call_traits<T, void(P0, P1, PN...)> {
  static constexpr overload_type overload = many_properties;
  typedef T result_type;
};

struct prefer_fn {
  template<class T, class P0, class P1, class... PN>
  enable_if_t<
      call_traits<T, void(P0, P1, PN...)>::overload == many_properties,
      typename call_traits<T, void(P0, P1, PN...)>::result_type>
  operator()(T&& value, P0&&, P1&&, PN&&...) const
  {
    return static_cast<T&&>(value);
  }
};

struct executor {
};

struct property_a {
};

struct property_b {
};

struct property_c {
};

int main()
{
  prefer_fn prefer;
  executor ex;
  const property_a a = {};
  const property_b b = {};
  const property_c c = {};
  executor result = prefer(ex, a, b, c);
  (void)result;
  return 0;
}
