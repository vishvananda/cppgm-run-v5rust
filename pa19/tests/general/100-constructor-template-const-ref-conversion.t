template<int N>
struct tag {};

template<class Rep, class Period>
struct duration {
  duration(Rep) {}

  template<class Rep2, class Period2>
  duration(const duration<Rep2, Period2>&) {}
};

int accept(const duration<long, tag<1>>&)
{
  return 0;
}

int main()
{
  return accept(duration<int, tag<2>>(8));
}
