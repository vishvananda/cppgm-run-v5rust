int h(int x, char y)
{
  return x + y;
}

template<class... Args>
int g(Args... args)
{
  return h(args...);
}

int main()
{
  return g(1, 'a') != 98;
}
