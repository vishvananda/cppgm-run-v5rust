#include <functional>

int main()
{
  int value = 42;
  std::function<int()> function = [&]() { return value; };
  return function() - 42;
}
