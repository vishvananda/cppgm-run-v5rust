template<typename T>
int pick(T &)
{
  return 1;
}

template<typename T>
int pick(const T &)
{
  return 2;
}

int main()
{
  int x = 0;
  const int y = 0;
  return pick(x) == 1 && pick(y) == 2 && pick(3) == 2 ? 0 : 1;
}
