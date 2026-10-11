// VALIDATION: compile-pass
// N3485 focus: 3.3.3 [basic.scope.local], 14.6 [temp.res]

int f(int x)
{
  return x;
}

int g(int x)
{
  return x + 1;
}

template<class F>
F preserve_target(F f)
{
  return f;
}

int main()
{
  int (*target)(int) = preserve_target(g);
  return target(1) == 2 ? 0 : 1;
}
