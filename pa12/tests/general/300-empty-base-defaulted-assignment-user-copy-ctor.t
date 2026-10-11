struct EmptyBase
{
  EmptyBase() {}
  EmptyBase(const EmptyBase &) {}
  EmptyBase &operator=(const EmptyBase &) = default;
};

struct Holder : EmptyBase
{
  int value;
};

int main()
{
  Holder a;
  Holder b;
  a.value = 7;
  b = a;
  return b.value;
}
