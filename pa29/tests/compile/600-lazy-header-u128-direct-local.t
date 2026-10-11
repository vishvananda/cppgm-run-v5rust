#include "600-lazy-header-u128-direct-local.h"

int main()
{
  unsigned long long hi = 1;
  unsigned long long lo = lazy_header_u128_mul(2, 3, hi);
  return lo == 6 && hi == 0 ? 0 : 1;
}
