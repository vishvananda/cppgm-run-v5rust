struct C {
  template <class R, class F>
  R lookup(int & target, const int & name, const F & f)
  {
    return f(target, name);
  }

  int g(int & target, const int & name)
  {
    return lookup<int>(target, name,
                       [](int &, const int & x) -> int
                       {
                         return x + 2;
                       });
  }
};

int main()
{
  int x = 0;
  int y = 1;
  C c;
  return c.g(x, y) - 3;
}
