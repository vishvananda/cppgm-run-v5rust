#include "200-host-packed-bit-field-wide-split.helper.h"

unsigned long long host_read(const Spans& value)
{
  return value.wide + value.low * 1000 + value.after;
}
void host_write(Spans* value, unsigned long long wide) { value->wide = wide; }
