template<int I, class... Ts>
struct Base;

template<int I, class Head, class... Tail>
struct Base<I, Head, Tail...> {
  template<class... Us>
  void f(const Base<I, Us...> &)
  {
  }
};

int main()
{
  Base<0, int, int> a;
  Base<0, int, int> b;
  a.f(b);
  return 0;
}
