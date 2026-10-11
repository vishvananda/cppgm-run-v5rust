typedef unsigned long size_t;

template<size_t... I>
struct indexes
{
};

template<class... Args>
struct type_list
{
};

template<class T>
struct tuple1
{
  T value;
};

template<size_t I, class T>
T get(tuple1<T> & value)
{
  return value.value + I;
}

template<class T>
T identity(T value)
{
  return value;
}

template<class... Args, size_t... I>
int run(tuple1<int> & tuple, type_list<Args...>, indexes<I...>)
{
  struct pair_like
  {
    int first;

    pair_like(tuple1<int> & value)
      : first(identity<Args>(get<I>(value))...)
    {
    }
  };

  pair_like value(tuple);
  return value.first == 7 ? 0 : 1;
}

int main()
{
  tuple1<int> tuple = { 7 };
  return run(tuple, type_list<int>(), indexes<0>());
}
