inline namespace api
{
  int select(int value)
  {
    return value;
  }
}

int call()
{
  return select(1);
}
