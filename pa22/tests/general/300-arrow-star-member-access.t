struct S
{
  int value;

  int plus(int x)
  {
    return value + x;
  }
};

int main()
{
  S s = {3};
  S * p = &s;
  int S::* data = &S::value;
  int (S::* method)(int) = &S::plus;
  return p->*data == 3 && (p->*method)(4) == 7 ? 0 : 1;
}
