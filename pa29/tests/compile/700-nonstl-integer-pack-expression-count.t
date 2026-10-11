typedef unsigned long size_t;

template<class T, T... I>
struct seq {};

template<size_t End, size_t Start>
struct make_indices {
  typedef seq<size_t, __integer_pack(End - Start)...> type;
};

typedef make_indices<3, 1>::type indices;

int main()
{
  return 0;
}
