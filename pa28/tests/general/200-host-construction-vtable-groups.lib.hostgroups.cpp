#include "200-host-construction-vtable-groups.helper.h"

int A::f() { return 1; }
int A::g() { return 2; }
X::X() : x(10) { note_value(static_cast<A*>(this)->g()); }
Y::Y() : y(20) { note_value(static_cast<A*>(this)->g()); note_value(h()); }
int Y::h() { return 3; }
Z::Z() : z(30)
{
  note_value(static_cast<A*>(this)->g());
  note_value(static_cast<A*>(this)->f());
  note_value(static_cast<Y*>(this)->y + x);
}
int Z::g() { return z + static_cast<Y*>(this)->y; }

HD::HD() : d(40)
{
  note_value(static_cast<HA*>(this)->f());
  note_value(static_cast<HA*>(this)->g());
}
int HD::f() { return d + 1; }
HD* host_make_hd() { return new HD; }
