#include "200-host-reference-temporary-symbol-spelling.helper.h"

int host_reads()
{
  return g.v + ns::nr.v + gi + shared();
}
