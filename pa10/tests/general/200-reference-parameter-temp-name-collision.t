bool same(const int &t1, const int &t2)
{
  return t1 == t2;
}

int main()
{
  int a = 0;
  int b = 0;
  return same(a, b) ? 0 : 1;
}
