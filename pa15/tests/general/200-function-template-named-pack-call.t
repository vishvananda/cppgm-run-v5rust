template<class... Args>
int f(Args... args)
{
  return 7;
}

int main()
{
  return f(1, 2, 3);
}
