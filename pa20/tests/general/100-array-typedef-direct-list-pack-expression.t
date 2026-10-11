int hits;

template<class T>
int touch()
{
  ++hits;
  return sizeof(T);
}

template<class... T>
int run()
{
  typedef int A[sizeof...(T) + 1];
  (void)A{0, touch<T>()...};
  return hits;
}

int main()
{
  return run<char, int>() == 2 ? 0 : 1;
}
