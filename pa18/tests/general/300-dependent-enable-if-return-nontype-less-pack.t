template<bool B, class T>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class... T>
struct tuple
{
};

template<class T, int I = 0, class... U>
typename enable_if<I == sizeof...(U) - 1, T>::type const *
find(tuple<U...> const &)
{
  return 0;
}

template<class T, int I = 0, class... U>
typename enable_if<I < sizeof...(U) - 1, T>::type const *
find(tuple<U...> const &)
{
  return 0;
}

struct first
{
};

struct second
{
};

int main()
{
  tuple<first, second> value;
  return find<second>(value) == 0 && find<second, 1>(value) == 0 ? 0 : 1;
}
