int main()
{
  auto const add = [](int lhs, int rhs = 2)
  {
    return lhs + rhs;
  };
  return add(3) == 5 ? 0 : 1;
}
