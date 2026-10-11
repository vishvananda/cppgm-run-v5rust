struct Pair
{
  int a = 4;
  int b;

  Pair() : b(a + 3) {}
};

int main()
{
  Pair p;
  return p.a == 4 && p.b == 7 ? 0 : 1;
}
