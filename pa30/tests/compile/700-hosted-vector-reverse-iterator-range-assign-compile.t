#include <string>
#include <vector>

void assign_reversed_strings()
{
  std::vector<std::string> reversed;
  std::vector<std::string> result;
  result.assign(reversed.rbegin(), reversed.rend());
}
