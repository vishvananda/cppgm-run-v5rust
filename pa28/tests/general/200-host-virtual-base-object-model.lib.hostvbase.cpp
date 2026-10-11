#include "200-host-virtual-base-object-model.helper.h"

int order[8];
int order_count;
static int seen_in_middle;

Root::Root() : root(1) { order[order_count++] = 1; }
Root::~Root() { order[order_count++] = -1; }
Middle::Middle() : middle(2) { order[order_count++] = 2; seen_in_middle = which(); }
Middle::~Middle() { order[order_count++] = -2; }
int Middle::which() { return 20 + seen_in_middle; }

int host_read_rows(const Rows& rows) { return rows.a * 100 + rows.b * 10 + rows.c; }
void host_fill_rows(Rows& rows) { rows.a = 4; rows.b = 5; rows.c = 6; }
Rows* host_make_rows()
{
  static Rows rows;
  rows.a = 7; rows.b = 8; rows.c = 9;
  return &rows;
}
int host_read_value(OStream* out) { return out->value; }
Weird* host_make_weird()
{
  static Weird weird;
  weird.value = 31;
  return &weird;
}
void host_mark(Marker& marker, int value) { marker.mark(value); }
int host_which(Middle& middle) { return middle.which(); }
