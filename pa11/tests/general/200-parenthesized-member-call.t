struct Accumulator
{
  int base;

  Accumulator() : base(3) {}

  int add(int value) const
  {
    return base + value;
  }
};

int main()
{
  Accumulator accumulator;
  return (accumulator.add)(4) == 7 ? 0 : 1;
}
