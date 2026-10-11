int f(int x)
{
  switch(x)
  {
  case 0:
    {
target:
      int y = x;
      return y;
    }
  case 1:
    goto target;
  }
  return 0;
}

int main()
{
  return f(1) == 1 ? 0 : 2;
}
