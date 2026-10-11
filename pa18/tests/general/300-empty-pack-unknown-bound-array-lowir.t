typedef unsigned long size_t;

template<class T, T... I>
struct integer_sequence
{
};

template<size_t... I>
using index_sequence = integer_sequence<size_t, I...>;

struct walker
{
  template<size_t... J>
  static void walk(index_sequence<J...>)
  {
    int a[] = { ((void)J, 0)... };
    (void)a;
  }
};

int main()
{
  walker::walk(index_sequence<>());
  return 0;
}
