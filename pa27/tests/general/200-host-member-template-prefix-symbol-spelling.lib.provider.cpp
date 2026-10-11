struct context
{
  template<class Value>
  struct executor
  {
    int id;
    struct inner { int depth; };
    template<class Other>
    executor<Other> require(const Other &) const;
    int take(executor other) const;
    int depth(inner value) const;
  };
};

template<class> struct outer
{
  template<class> struct item
  {
    int value;
    item() : value(3) {}
    template<class U> item(item<U> const & other);
  };
};

template<class A0, class A1, class A2, class A3, class A4, class A5,
  class A6, class A7, class A8, class A9, class A10, class A11>
int last(A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11 value);

int host_members()
{
  context::executor<int> e;
  e.id = 1;
  context::executor<int>::inner n;
  n.depth = 2;
  const int required = e.require(5L).id;
  const int taken = e.take(e);
  const int deep = e.depth(n);
  outer<int>::item<long> converted = outer<int>::item<int>();
  const int tail = last(0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3L);
  return required + taken + deep + converted.value + tail - 3 - 1;
}
