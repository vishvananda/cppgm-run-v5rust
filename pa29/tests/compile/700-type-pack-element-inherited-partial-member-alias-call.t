template<class... T>
struct tuple
{
};

template<class... T>
struct tuple_types
{
};

template<unsigned long I, class T>
struct tuple_element;

template<unsigned long I, class... T>
struct tuple_element<I, tuple_types<T...> >
{
  typedef __type_pack_element<I, T...> type;
};

template<unsigned long I, class... T>
struct tuple_element<I, tuple<T...> >
  : tuple_element<I, tuple_types<T...> >
{
};

template<unsigned long I, class... T>
typename tuple_element<I, tuple<T...> >::type & get(tuple<T...> &);

struct First
{
};

struct Second
{
};

void consume(Second &);

int main()
{
  tuple<First, Second> values;
  consume(get<1>(values));
}
