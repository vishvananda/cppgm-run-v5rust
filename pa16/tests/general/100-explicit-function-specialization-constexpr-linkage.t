template<class T> constexpr int cexpr();
template<> constexpr int cexpr<int>() { return 1; }

template<class T> constexpr int strong_primary();
template<> int strong_primary<int>() { return 2; }

int main()
{
  return cexpr<int>() + strong_primary<int>() - 3;
}
