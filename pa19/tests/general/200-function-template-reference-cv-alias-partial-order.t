// Reduced from Boost.Xpressive regex_search overload resolution.
// When two function templates instantiate to the same reference type for a
// const lvalue argument, a dependent alias parameter can leave primary partial
// ordering inconclusive. The source reference pattern still prefers `T const&`
// over `T&` for the const lvalue.

template<typename T>
struct match_results {
};

template<typename T>
struct regex_alias {
  typedef int type;
};

template<typename T>
using basic_regex = typename regex_alias<T>::type;

template<typename Range, typename Iter>
int regex_search(Range &,
                 match_results<Iter> &,
                 basic_regex<Iter> const &)
{
  return 1;
}

template<typename Range, typename Iter>
int regex_search(Range const &,
                 match_results<Iter> &,
                 basic_regex<Iter> const &)
{
  return 2;
}

struct iterator {
};

struct string_type {
};

int main()
{
  string_type const str = {};
  match_results<iterator> what;
  int rex = 0;
  return regex_search(str, what, rex) == 2 ? 0 : 1;
}
