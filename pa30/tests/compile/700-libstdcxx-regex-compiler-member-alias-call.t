#include <regex>

int main()
{
  std::regex expression("x");
  return expression.mark_count();
}
