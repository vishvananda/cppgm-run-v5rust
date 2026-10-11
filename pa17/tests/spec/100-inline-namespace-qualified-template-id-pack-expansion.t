// VALIDATION: compile-pass
// N3485 focus: 14.5.3 [temp.variadic], 7.3.1 [namespace.def]

namespace library {
inline namespace version {
template<unsigned long... I>
struct indices
{
};

template<class... _Tp>
struct tuple;

template<class... _Types>
struct __tuple_types
{
};

template<unsigned long _Ip, class _Head, class... _Tail>
struct pack_element : pack_element<_Ip - 1, _Tail...>
{
};

template<class _Head, class... _Tail>
struct pack_element<0, _Head, _Tail...>
{
  using type = _Head;
};

template<unsigned long _Ip, class _Tp>
struct tuple_element;

template<unsigned long _Ip, class... _Tp>
typename tuple_element<_Ip, tuple<_Tp...> >::type get(tuple<_Tp...> &)
{
  return typename tuple_element<_Ip, tuple<_Tp...> >::type();
}

template<unsigned long _Ip, class... _Types>
struct tuple_element<_Ip, __tuple_types<_Types...> >
{
  using type = typename pack_element<_Ip, _Types...>::type;
};

template<unsigned long _Ip, class... _Tp>
struct tuple_element<_Ip, tuple<_Tp...> >
{
  using type = typename tuple_element<_Ip, __tuple_types<_Tp...> >::type;
};

template<class... _Tp>
struct tuple
{
};

template<class T>
struct reference_wrapper
{
};

template<class T, class U>
int transform(reference_wrapper<T>, U &);

template<class T, class U>
int transform(T, U &)
{
  return 1;
}

int sum(int left, int right)
{
  return left + right;
}

template<class Bound, unsigned long... I>
int apply(Bound & bound, indices<I...>)
{
  int args = 0;
  return sum(library::transform(library::get<I>(bound), args)...);
}
}
}

int main()
{
  library::tuple<int, double> bound;
  return library::apply(bound, library::indices<0, 1>());
}
