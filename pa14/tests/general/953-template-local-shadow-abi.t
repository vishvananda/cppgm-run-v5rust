template<class T>
int value()
{
  { struct A {}; }
  {
    struct A { int f() { return 1; } };
    A a;
    return a.f();
  }
}

int main()
{
  return value<int>() - 1;
}
