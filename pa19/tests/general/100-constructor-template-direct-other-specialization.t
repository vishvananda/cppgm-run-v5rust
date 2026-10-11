template<int N>
struct tag {};

template<class Rep, class Period>
struct duration {
  duration(Rep) {}

  template<class Rep2, class Period2>
  duration(const duration<Rep2, Period2>&) {}
};

int main()
{
  duration<int, tag<2>> src(8);
  duration<long, tag<1>> dst(src);
  (void)dst;
  return 0;
}
