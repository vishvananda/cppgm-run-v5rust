// Repeated selected elements retain the sizeof... operand's whole pack key.
template<int N> int value() { return N; }
int use(int a) { return a; }
int use(int a, int b) { return a + b; }

template<int... I> int expand()
{
  return use(value<I + sizeof...(I)>()...);
}

int main()
{
  return expand<0>() == 1 && expand<0, 0>() == 4 && expand<0>() == 1 ? 0 : 1;
}
