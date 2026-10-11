int f()
{
  int * p = new int[4];
  p[0] = 4;
  return p[0];
}

int main()
{
  return f() - 4;
}
