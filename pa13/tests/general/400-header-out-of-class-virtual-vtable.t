#include "400-header-out-of-class-virtual-vtable.h"

HeaderVtableDerived::HeaderVtableDerived() : x(7) {}

int HeaderVtableDerived::value()
{
  return x;
}

int main()
{
  HeaderVtableDerived d;
  return d.value() == 7 ? 0 : 1;
}
