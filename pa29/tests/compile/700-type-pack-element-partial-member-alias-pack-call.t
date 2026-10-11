template<class... T>
struct pack
{
};

template<unsigned long I, class Tuple>
struct tuple_element;

template<unsigned long I, class... T>
struct tuple_element<I, pack<T...> >
{
  using type = __type_pack_element<I, T...>;
};

template<unsigned long I, class... T>
typename tuple_element<I, pack<T...> >::type & get(pack<T...> &)
{
  static typename tuple_element<I, pack<T...> >::type value;
  return value;
}

template<class T>
T && move(T & value)
{
  return static_cast<T &&>(value);
}

void consume(int &&, long &&)
{
}

template<unsigned long... I>
struct index_sequence
{
};

template<class H, class... T, unsigned long... I>
void execute(pack<H, T...> & values, index_sequence<I...>)
{
  consume(move(get<I + 1>(values))...);
}

int main()
{
  pack<char, int, long> values;
  execute(values, index_sequence<0, 1>());
}
