struct S
{
  int value;

  int twice()
  {
    return value * 2;
  }
};

int main()
{
  S s = {4};
  int S::* data = &S::value;
  int (S::* method)() = &S::twice;
  return s.*data == 4 && (s.*method)() == 8 ? 0 : 1;
}
