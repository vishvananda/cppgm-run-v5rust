namespace api
{
int transform(int value)
{
  return value + 1;
}

long transform(long value)
{
  return value + 2;
}
}

int apply(int (*function)(int), int value)
{
  return function(value);
}

int main()
{
  return apply(api::transform, 4) == 5 ? 0 : 1;
}
