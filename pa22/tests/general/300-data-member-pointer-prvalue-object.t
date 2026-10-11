struct X
{
  int m;
};

X make(int v)
{
  X x = { v };
  return x;
}

int main()
{
  int X::* pm = &X::m;
  return make(42).*pm == 42 ? 0 : 1;
}
