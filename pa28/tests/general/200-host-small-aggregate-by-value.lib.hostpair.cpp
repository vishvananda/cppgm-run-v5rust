#include "200-host-small-aggregate-by-value.helper.h"

int sum_pair(Pair value)
{
  return static_cast<int>(value.a + value.b);
}
