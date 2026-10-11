struct V
{
  int v;

  V() : v(42) {}
};

struct A
{
  int a;

  A() : a(7) {}
};

struct D : A, virtual V
{
  D() {}
};

void consume(D & d);

int main()
{
  D d;
  consume(d);
  return 0;
}
