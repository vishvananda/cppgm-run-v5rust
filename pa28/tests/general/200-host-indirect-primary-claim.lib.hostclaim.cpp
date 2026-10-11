#include "200-host-indirect-primary-claim.helper.h"

int A::f() { return 1; }
int Base::b() { return 3; }
long host_signature() { return signature(); }
int host_f(A& value) { return value.f(); }
int host_y(Y& value) { return value.y; }
Z* host_make()
{
  Z* made = new Z;
  made->x = 4;
  made->y = 5;
  made->z = 6;
  return made;
}
