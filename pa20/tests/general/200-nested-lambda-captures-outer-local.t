int main()
{
  auto outer = [&](int input) -> int
  {
    int local = input + 1;
    auto inner = [&local]() -> int
    {
      return local;
    };
    return inner();
  };
  return outer(4) == 5 ? 0 : 1;
}
