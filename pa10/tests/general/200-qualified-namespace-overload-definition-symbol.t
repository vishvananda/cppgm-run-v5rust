namespace N {
  int f(float) noexcept;
  int f(double) noexcept;
}

int N::f(float x) noexcept
{
  return x == 1.0f ? 1 : 0;
}

int N::f(double x) noexcept
{
  return x == 1.0 ? 2 : 0;
}

int main()
{
  return N::f(1.0) == 2 ? 0 : 1;
}
