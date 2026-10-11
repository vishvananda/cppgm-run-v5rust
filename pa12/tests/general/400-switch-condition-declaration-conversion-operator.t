// N3485 focus: 6.4.2 [stmt.switch], 12.3.2 [class.conv.fct]

struct state_ref
{
  int value;

  state_ref(int & source)
    : value(source)
  {
  }

  operator int() const
  {
    return value;
  }
};

int main()
{
  int source = 2;
  int result = 0;

  switch (state_ref state = source)
  {
  case 2:
    result = 7;
    break;
  default:
    result = 1;
    break;
  }

  return result == 7 ? 0 : 1;
}
