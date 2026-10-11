template<class R>
struct Guard {
  Guard() {}
  Guard(const Guard&) {}
};

template<class T>
struct V {
  struct D {};
};

int f()
{
  enum E {
    A
  };
  Guard<typename V<E>::D> g;
  (void)g;
  return 0;
}

int main()
{
  return f();
}
