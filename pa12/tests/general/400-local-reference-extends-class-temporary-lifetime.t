int destroyed;

struct value
{
  int payload;

  explicit value(int input) : payload(input) {}
  ~value() { ++destroyed; }
};

value make_value()
{
  return value(7);
}

int main()
{
  int observed = 0;
  {
    const value& reference = make_value();
    if (destroyed != 0)
      return 1;
    observed = reference.payload;
  }
  return destroyed == 1 && observed == 7 ? 0 : 2;
}
