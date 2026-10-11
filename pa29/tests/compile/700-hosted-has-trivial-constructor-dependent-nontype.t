template <bool B>
struct dispatch {
  static int run() { return 1; }
};

template <>
struct dispatch<true> {
  static int run() { return 0; }
};

template <class T>
int use_trait(T *p)
{
  (void)p;
  return dispatch<__has_trivial_constructor(T)>::run();
}

struct Box {
  int value;
};

int main()
{
  Box b;
  return use_trait(&b);
}
