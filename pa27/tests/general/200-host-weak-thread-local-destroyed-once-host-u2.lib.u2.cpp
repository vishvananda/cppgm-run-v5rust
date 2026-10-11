#include "200-host-weak-thread-local-destroyed-once-host-u2.helper.h"
__attribute__((weak)) thread_local D w = { 6 };
int read_u2() { return w.v; }
