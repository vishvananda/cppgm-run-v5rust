#include "200-host-primary-entry-overrider.helper.h"

int HX::x() { return 101; }
int HY::y() { return 102; }
int HV::y() { return 120; }
int HW::w() { return 103; }
int HD::x() { return 110; }

int host_d_x(D& value) { return value.x(); }
int host_v_y(V& value) { return value.y(); }
int host_v_x(V& value) { return value.x(); }
HD* host_make() { return new HD; }
