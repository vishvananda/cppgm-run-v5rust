struct sink {
  template<class T>
  void operator()(T) {}
};

template<class... T, class F>
F run_pack(F f)
{
  using A = int[sizeof...(T)];
  return (void)A{ ((void)f(T()), 0)... }, f;
}

int main()
{
  run_pack<char, short, int>(sink());
  return 0;
}
