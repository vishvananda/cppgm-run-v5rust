struct Analyzer
{
  void register_builtin_function()
  {
  }

  void f()
  {
    auto reg = [this]()
    {
      this->register_builtin_function();
    };
    reg();
  }
};

int main()
{
  Analyzer a;
  a.f();
  return 0;
}
