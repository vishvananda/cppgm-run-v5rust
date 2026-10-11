// VALIDATION: compile-pass

struct Box {
  int value;

  Box(int value_in) : value(value_in) {}
};

int pick(const Box&)
{
  return 1;
}

int pick(Box&& value)
{
  return value.value == 7 ? 0 : 2;
}

int main()
{
  return pick(7);
}
