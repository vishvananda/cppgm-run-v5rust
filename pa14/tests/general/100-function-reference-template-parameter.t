template<typename Fn>
int apply(int value, const Fn & fn)
{
  return fn(value);
}

int inc(int value)
{
  return value + 1;
}

int main()
{
  return apply(41, inc) != 42;
}
