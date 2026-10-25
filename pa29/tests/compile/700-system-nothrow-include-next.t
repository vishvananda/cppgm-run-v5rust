#include "system-nothrow-forward/chain.h"
extern "C" int system_nested();
static_assert(noexcept(system_nested()), "include_next selected a system directory");
int invoke() noexcept { return system_nested(); }
