#include "700-lazy-header-constexpr-sfinae-pair.h"

lazy_header_sfinae::pair_like<unsigned long, unsigned long>
make_lazy_header_pair(unsigned long first, unsigned long second)
{
  return lazy_header_sfinae::pair_like<unsigned long, unsigned long>(first,
                                                                    second);
}

int main()
{
  make_lazy_header_pair(1, 2);
  return 0;
}
