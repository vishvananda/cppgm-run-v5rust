class Resolver
{
  struct Result
  {
    bool found;
    int value;
  };

  Result result;
  Result & resolve(int value);

public:
  int go(int value);
};

Resolver::Result & Resolver::resolve(int value)
{
  result.found = value > 0;
  result.value = value * 2;
  return result;
}

int Resolver::go(int value)
{
  Result & resolved = resolve(value);
  return resolved.found ? resolved.value : -1;
}

int main()
{
  Resolver resolver;
  return resolver.go(21) == 42 && resolver.go(0) == -1 ? 0 : 1;
}
