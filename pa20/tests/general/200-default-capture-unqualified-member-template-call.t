struct parser_test {
  int calls;

  parser_test() : calls(0) {}

  template<class Parser, class Check>
  void parse(int message, Check const& check, bool skip = false)
  {
    if (!skip) {
      calls += check(message);
    }
  }

  void run()
  {
    auto const invoke = [&](int message)
    {
      parse<int>(message, [&](int value) { return value + 1; });
    };
    invoke(2);
  }
};

int main()
{
  parser_test test;
  test.run();
  return test.calls == 3 ? 0 : 1;
}
