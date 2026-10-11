int select(int);

int select(const int value)
{
  return value;
}

int call(int value)
{
  return select(value);
}
