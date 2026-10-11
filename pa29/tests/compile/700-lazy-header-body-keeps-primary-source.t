#include "700-lazy-header-body-keeps-primary-source.h"

int main()
{
  return alignof(lazy_primary_source_type) > 0 ? 0 : 1;
}
