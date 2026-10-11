#include "200-host-thunk-emission.helper.h"

int TX::x() { return 1; }
int TY::y() { return 2; }
int TD::k() { return 3; }
CE::CE() : m6(60) { m3 = 3; }
int CE::f0() { return m6; }
int CE::f4() { return m6 + 4; }
int S0::f2() { return 20; }
int S2::g() { return 21; }
int S3::f2() { return 22 + m3; }

TD* host_make_td() { return new TD; }
int host_td_y(TY& value) { return value.y(); }
CE* host_make_ce() { return new CE; }
int host_c0_f0(C0& value) { return value.f0(); }
int host_s0_f2(S0& value) { return value.f2(); }
