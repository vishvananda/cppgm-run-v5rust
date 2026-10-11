struct number {
  operator int() const { return 7; }
};

int main()
{
  number a;
  number b;
  return a + b - 14;
}
