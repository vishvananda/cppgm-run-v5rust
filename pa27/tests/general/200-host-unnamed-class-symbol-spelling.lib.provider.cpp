#include "200-host-unnamed-class-symbol-spelling.helper.h"

int host_named()
{
  NamedByTypedef n;
  n.y = 2;
  return n.get();
}

int host_unnamed()
{
  Holder h;
  return unnamed_size(&h.pair);
}
