namespace A {
int helper(const int & value)
{
  return value;
}

int use()
{
  return helper(3);
}

namespace {
int helper(const int & value)
{
  return value + 1;
}
}
}

int main()
{
  return A::use();
}
