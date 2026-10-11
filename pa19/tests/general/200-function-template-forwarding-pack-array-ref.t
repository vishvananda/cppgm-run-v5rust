template<class... Args>
int f(Args&&... args)
{
  return 7;
}

int main()
{
  const char (&x)[2] = "x";
  return f(x, 1);
}
