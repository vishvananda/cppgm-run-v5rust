#include "700-lazy-using-import-retains-base-method.h"

typedef lazy_using_import_retains_base_method::deque_like<int> referenced_deque;

referenced_deque * referenced_only;

int main()
{
  referenced_deque values;
  return referenced_only == &values ? 1 : 0;
}
