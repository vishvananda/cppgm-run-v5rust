#include "200-host-vtable-group-order.helper.h"

T* host_make()
{
  T* made = new T;
  made->m1 = 1;
  made->m2 = 2;
  made->mp = 3;
  made->mt = 4;
  return made;
}
int host_f(B1& value) { return value.f(); }
int host_g(B2& value) { return value.g(); }
