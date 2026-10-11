struct no_default
{
  explicit no_default(int) {}
};

struct has_default
{
  has_default() {}
};

template<class T>
struct box
{
  explicit box(const T &) {}
  box(int, const T & = T()) {}
};

template<class T>
int choose(T *, T = T())
{
  return 1;
}

int choose(no_default *)
{
  return 2;
}

int main()
{
  no_default value(1);
  box<no_default> first(value);
  box<has_default> second(1);

  no_default *without_default = 0;
  has_default *with_default = 0;
  return choose(without_default) + choose(with_default) - 3;
}
