int f(int x)
{
  int y = 0;
  switch(x)
  {
  case 0:
    {
      y = x + 4;
target:
      return y;
    }
  case 1:
    y = x + 4;
    goto target;
  }
  return 0;
}

int main()
{
  return f(1) == 5 ? 0 : 2;
}
