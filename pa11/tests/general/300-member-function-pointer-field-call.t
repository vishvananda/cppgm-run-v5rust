struct Hooks {
  int (*fn)(int);
};

int plus_one(int x)
{
  return x + 1;
}

int call(Hooks const & h)
{
  return (h.fn)(4);
}

int main()
{
  Hooks h;
  h.fn = plus_one;
  return call(h) == 5 ? 0 : 1;
}
