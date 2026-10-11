#include "300-host-parameter-pack-specialization-link.helper.h"

extern "C" int host_pack_sum()
{
  int value = 0;
  return pack_sum(value);
}
