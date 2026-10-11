int main()
{
  auto f = []()
  {
    int x = 1;
    return x;
  };

  return f() - 1;
}
