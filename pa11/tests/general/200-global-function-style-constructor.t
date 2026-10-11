struct X {
  int value;

  explicit X(int n) : value(n) {}
};

int f2 = 7;
X x(f2);

int main()
{
  return x.value == 7 ? 0 : 1;
}
