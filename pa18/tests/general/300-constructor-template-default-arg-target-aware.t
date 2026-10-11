struct iter {
  int value;
};

struct box {
  int first;
  int last;
  int extra;

  template<typename It>
  box(It a, It b, int x = 0) : first(a.value), last(b.value), extra(x) {}
};

box make_box(iter a, iter b)
{
  return box(a, b);
}

int main()
{
  iter a = {1};
  iter b = {4};
  box z = make_box(a, b);
  return (z.first == 1 && z.last == 4 && z.extra == 0) ? 0 : 1;
}
