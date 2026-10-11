struct Live
{
  int * count;

  Live(int * value) : count(value)
  {
    ++*count;
  }

  ~Live()
  {
    --*count;
  }
};

struct Temporary
{
  int value;

  Temporary(int input) : value(input)
  {
    if (input < 0)
      throw input;
  }

  ~Temporary()
  {
  }
};

struct StaticValue
{
  int value;

  StaticValue(const Temporary & left, const Temporary & right)
    : value(left.value + right.value)
  {
    if (value < 0)
      throw value;
  }
};

int read_static(int * live_count)
{
  Live live(live_count);
  static StaticValue value(Temporary(1), Temporary(2));
  return value.value;
}

int main()
{
  int live_count = 0;
  if (read_static(&live_count) != 3 || live_count != 0)
    return 1;
  if (read_static(&live_count) != 3 || live_count != 0)
    return 2;
  return 0;
}
