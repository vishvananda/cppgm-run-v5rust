struct A {
  int x;

  int run()
  {
    auto f = [this]() -> int { return x; };
    return f();
  }
};

int main()
{
  A a;
  a.x = 7;
  return a.run() - 7;
}
