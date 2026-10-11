#include "../lazy_parenthesized_qualified_function_template_call.h"

int main()
{
  return H::Box<int>::f(3, 2);
}
