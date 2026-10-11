// N3485 focus: 14.8.2 [temp.deduct], substituted function result types

namespace lib {

template<bool B, class T = void>
struct enable_if_c {
  typedef T type;
};

template<class T>
struct enable_if_c<false, T> {};

template<class Cond, class T = void>
struct enable_if : enable_if_c<Cond::value, T> {};

namespace detail {

template<class T, class U>
struct same {
  static const bool value = false;
};

template<class T, class U>
struct assignable {
  static const bool value = true;
};

template<class T, class U, bool = same<T, U>::value>
struct candidate {
  static const bool value = false;
};

template<class T, class U>
struct candidate<T, U, false> : assignable<T, U> {};

}

template<class T>
struct optional {
  template<class Expr>
  typename enable_if<detail::candidate<T, Expr>, optional&>::type
  operator=(Expr)
  {
    return *this;
  }
};

}

struct edge {};
struct proxy {};

int main()
{
  lib::optional<edge> value;
  proxy p;
  value = p;
  return 0;
}
// VALIDATION: compile-pass
