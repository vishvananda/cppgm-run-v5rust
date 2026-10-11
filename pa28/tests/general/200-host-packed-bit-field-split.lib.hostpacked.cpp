#include "200-host-packed-bit-field-split.helper.h"

int host_read_wide(const Packed& value) { return value.wide + value.low * 1000 + value.after; }
void host_write_wide(Packed* value, int wide) { value->wide = wide; }
long host_read_value(const Tail& value) { return value.value + value.first + value.last; }
void host_write_value(Tail* value, long wide) { value->value = wide; }
