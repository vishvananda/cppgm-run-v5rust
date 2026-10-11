template<int N> int value() { return N; }
int use(int n) { return n; }

template<int... I> int expand()
{
  return use(value<I + sizeof...(I)>()...);
}

int main() { return expand<0>() == 1 ? 0 : 1; }
