struct first {};
struct second {};

template<class A, class B>
struct pair_box
{
  friend pair_box<B, A> flip(const pair_box&, B)
  {
    return pair_box<B, A>();
  }
};

int main()
{
  pair_box<first, second> p;
  pair_box<second, first> q = flip(p, second());
  (void)q;
  return 0;
}
