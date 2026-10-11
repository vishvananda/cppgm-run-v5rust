template<typename T>
struct reference_wrapper {
  reference_wrapper(T &);
};

template<typename I>
struct basic_regex {};

namespace proto {
  template<typename T>
  struct terminal {
    struct type {};
  };
}

template<typename BidiIter>
typename proto::terminal<reference_wrapper<basic_regex<BidiIter> const> >::type
by_ref(basic_regex<BidiIter> const & rex);

int host_by_ref()
{
  basic_regex<int> rex;
  by_ref(rex);
  return 7;
}
