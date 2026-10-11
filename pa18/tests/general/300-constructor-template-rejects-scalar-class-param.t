struct piecewise_construct_t {};

template<class... T>
struct tuple {};

struct iter {};

template<class First, class Second>
struct pair {
  pair(First, Second);

  template<class... A, class... B>
  pair(piecewise_construct_t, tuple<A...>, tuple<B...>) {}
};

int main()
{
  pair<iter, bool> p(iter(), true);
  (void)p;
  return 0;
}
