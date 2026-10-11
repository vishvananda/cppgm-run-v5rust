struct index {};

struct base
{
  static long select(index);
};

struct derived : base
{
  static char select(index);
  using base::select;
};

int main()
{
  return sizeof(derived::select(index())) == sizeof(char) ? 0 : 1;
}
