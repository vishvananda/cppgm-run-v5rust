typedef unsigned long size_t;

template<class T, T... I>
struct integer_sequence
{
};

template<size_t... I>
using index_sequence = integer_sequence<size_t, I...>;

template<class T, size_t... J>
int pick(T, index_sequence<J...>)
{
  return 1;
}

template<class T>
int pick(T, index_sequence<>)
{
  return 2;
}

int main()
{
  return pick(0, index_sequence<>()) == 2 ? 0 : 1;
}
