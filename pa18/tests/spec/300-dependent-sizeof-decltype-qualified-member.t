// VALIDATION: compile-pass
// A qualified concrete type substituted into sizeof and a nested decltype
// must retain enough structured identity for return-type SFINAE evaluation.

template<bool, class T = void>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class T>
T && declval();

namespace serialization
{
struct wrapped
{
  char const * value;
};
}

template<class T>
typename enable_if<
    sizeof(T) == sizeof(decltype(declval<T const &>().value)),
    int>::type
probe(T const &)
{
  return 7;
}

int main()
{
  serialization::wrapped value = {"value"};
  return probe(value) == 7 ? 0 : 1;
}
