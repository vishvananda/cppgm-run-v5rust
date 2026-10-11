// partner unit for 18-virtual-base-xunit.u1.cpp
#include "200-host-virtual-base-xunit-host-u2.helper.h"
struct F2 : VV { };
int check() { P* p = new F2; int r = p->f() == 1 ? 0 : 1; delete p; { VV v; } return r; }
