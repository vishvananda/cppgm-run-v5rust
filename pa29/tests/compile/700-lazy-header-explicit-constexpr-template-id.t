#include "700-lazy-header-explicit-constexpr-template-id.h"

int main()
{
  int value = 0;
  int * ptr = &value;

  lazy_header_explicit_constexpr::pair_like<int *, int *> direct =
      lazy_header_explicit_constexpr::make_pair_like(static_cast<int *>(ptr),
                                                    static_cast<int *>(ptr));
  lazy_header_explicit_constexpr::pair_like<int *, int *> lvalue =
      lazy_header_explicit_constexpr::make_pair_like(ptr, ptr);

  return direct.first != ptr || lvalue.first != ptr;
}
