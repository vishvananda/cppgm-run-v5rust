// N3485 focus: 14.7.3 [temp.expl.spec], 14.8.2 [temp.deduct]
// A function-template explicit specialization with an explicit template-id
// must match a primary whose trailing result type becomes non-dependent only
// after substituting the explicit non-type argument.

namespace std {
typedef decltype(sizeof(0)) size_t;

template<bool B, class T, class F>
struct conditional {
  typedef T type;
};

template<class T, class F>
struct conditional<false, T, F> {
  typedef F type;
};
}

struct string_view {
  int value;
};

struct value {
  int value;
};

struct key_value_pair {
  string_view key() const
  {
    return {7};
  }
};

template<std::size_t I>
auto get(key_value_pair const &) ->
    typename std::conditional<I == 0, string_view const, value const &>::type
{
  static_assert(I == 0, "index out of range");
}

template<>
string_view const get<0>(key_value_pair const & kvp)
{
  return kvp.key();
}

int main()
{
  key_value_pair kvp;
  string_view x = get<0>(kvp);
  return x.value == 7 ? 0 : 1;
}
