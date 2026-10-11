typedef unsigned long size_t;

template<size_t... I>
struct indices {};

template<class... T>
struct types {};

template<size_t... I>
struct impl {
  template<size_t... Uf, class... Tf, size_t... Ul, class... Tl, class... Up>
  explicit impl(indices<Uf...>, types<Tf...>, indices<Ul...>, types<Tl...>, Up&&... u) {}
};

struct S {};

int main()
{
  S s;
  impl<0> value(indices<0>(), types<S&&>(), indices<>(), types<>(), s);
  return 0;
}
