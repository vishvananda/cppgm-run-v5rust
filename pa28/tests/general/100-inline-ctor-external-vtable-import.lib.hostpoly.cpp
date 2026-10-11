#include "100-inline-ctor-external-vtable-import.helper.h"

int HostPoly::f() const
{
  return 7;
}

HostPoly::~HostPoly()
{
}
