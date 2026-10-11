#include "200-host-virtual-thunk-derived-class.helper.h"

struct Extended : Mid { int pad[3]; int extra; Extended() : extra(4) {} };

int host_value(Root& root) { return root.value(); }
int host_check()
{
  Extended extended;
  Root& root = extended;
  Side& side = extended;
  return root.value() == 33 && side.other() == 2 && extended.extra == 4 ? 0 : 1;
}
