template<typename T>
int apply(T (*fn)(T))
{
  return fn(3);
}

int g(int x)
{
  return x + 1;
}

char g(char, char)
{
  return 0;
}

int main()
{
  return apply(g) == 4 ? 0 : 1;
}
