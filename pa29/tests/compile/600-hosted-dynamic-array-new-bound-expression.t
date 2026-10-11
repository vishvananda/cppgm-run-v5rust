unsigned g();

int * f()
{
  return new int[10 + g()];
}

int main()
{
  return 0;
}
