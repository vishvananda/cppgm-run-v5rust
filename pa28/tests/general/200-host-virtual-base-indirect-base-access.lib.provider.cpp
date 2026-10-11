#include "200-host-virtual-base-indirect-base-access.helper.h"

Base::Base() : w(6)
{
}

long Base::width() const
{
  return w;
}

Pad::Pad() : p(11)
{
}

static B g_b;

B & get_b()
{
  return g_b;
}
