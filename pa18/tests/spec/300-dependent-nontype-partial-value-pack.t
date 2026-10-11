// VALIDATION: compile-pass
// A partial specialization whose non-type parameter depends on an earlier
// type deduction must preserve that type through value-pack expansion.

template<class T, class U>
struct same
{
  static const bool value = false;
};

template<class T>
struct same<T, T>
{
  static const bool value = true;
};

template<class E, class Enum = E>
struct condition
{
};

template<class T>
struct enum_type
{
  typedef T type;
};

template<class E>
struct enum_type<condition<E, E> >
{
  typedef E type;
};

enum class code
{
  first,
  second
};

template<class Match, class Value>
int compare_pack(Match, Value)
{
  static_assert(same<Value, code>::value, "expanded value keeps enum type");
  return 0;
}

template<class Match, class Value, class... Rest>
int compare_pack(Match match, Value value, Rest... rest)
{
  static_assert(same<Value, code>::value, "leading value keeps enum type");
  return compare_pack(match, value) + compare_pack(match, rest...);
}

template<class E,
         typename enum_type<E>::type First,
         typename enum_type<E>::type... Rest>
struct match
{
  typedef E selected;
};

template<class Enum,
         typename enum_type<condition<Enum, Enum> >::type First,
         typename enum_type<condition<Enum, Enum> >::type... Rest>
struct match<condition<Enum, Enum>, First, Rest...>
{
  typedef int selected;

  static int run()
  {
    return compare_pack(0, First, Rest...);
  }
};

static_assert(
    same<match<condition<code>, code::first, code::second>::selected, int>::value,
    "dependent non-type partial specialization selected");

int main()
{
  return match<condition<code>, code::first, code::second>::run();
}
