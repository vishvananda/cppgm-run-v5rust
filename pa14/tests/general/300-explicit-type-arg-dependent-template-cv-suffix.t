// Reduced from Boost.Xpressive by_ref.
// A dependent template-id used as an explicit type argument can carry a
// top-level cv suffix directly after the closing angle bracket while resolving
// an instantiated qualified static member call.

template<typename T>
struct reference_wrapper {
  reference_wrapper(T &) {}
};

template<typename I>
struct basic_regex {};

namespace proto {
  template<typename T>
  struct terminal {
    struct type {
      static type make(T) { return type(); }
    };
  };
}

template<typename BidiIter>
typename proto::terminal<reference_wrapper<basic_regex<BidiIter> const> >::type
by_ref(basic_regex<BidiIter> const & rex)
{
  reference_wrapper<basic_regex<BidiIter> const> ref(rex);
  return proto::terminal<reference_wrapper<basic_regex<BidiIter> const> >::type::make(ref);
}

int main()
{
  basic_regex<int> rex;
  by_ref(rex);
  return 0;
}
