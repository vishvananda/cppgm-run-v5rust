int main()
{
  int value = 0;
  int * p = &value;
  int ** pp = &p;
  const int ** cpp = pp;
  return cpp != 0;
}
