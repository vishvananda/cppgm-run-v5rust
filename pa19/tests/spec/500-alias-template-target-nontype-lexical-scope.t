// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias], alias-template target lexical scope
// A non-type argument in an alias target is evaluated in the alias declaration
// scope, not in the namespace of the selected target template.

namespace std {

struct string {};

template<class T, T V>
struct integral_constant {
  static const T value = V;
  typedef integral_constant type;
};

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class A, class B>
struct is_same : integral_constant<bool, false> {};

template<class A>
struct is_same<A, A> : integral_constant<bool, true> {};

}

namespace json {

struct string {};

namespace detail {

template<class T>
using string_and_stringlike = std::integral_constant<bool,
    std::is_same<T, string>::value>;

template<class T>
using string_comp_requirement = typename std::enable_if<
    string_and_stringlike<T>::value,
    bool>::type;

}

template<class T>
detail::string_comp_requirement<T>
equal(T const&)
{
  return true;
}

}

int main()
{
  json::string s;
  return json::equal(s) ? 0 : 1;
}
