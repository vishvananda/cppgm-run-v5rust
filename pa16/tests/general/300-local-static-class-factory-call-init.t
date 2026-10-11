int factory_calls = 0;

struct Wide
{
  long first;
  long middle;
  long last;
};

Wide make_wide(long seed)
{
  factory_calls = factory_calls + 1;
  Wide result = {seed, seed + 1, seed + 2};
  return result;
}

long read_cached(long seed)
{
  static Wide cached = make_wide(seed);
  return cached.first + cached.last;
}

int main()
{
  if (read_cached(7) != 16 || factory_calls != 1)
    return 1;
  if (read_cached(9) != 16 || factory_calls != 1)
    return 2;
  return 0;
}
