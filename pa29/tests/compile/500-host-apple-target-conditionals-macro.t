#if defined(__APPLE__)
#include <TargetConditionals.h>
#if !defined(TARGET_OS_MAC)
#error missing TargetConditionals macros
#endif
static_assert(TARGET_OS_MAC == 1, "TARGET_OS_MAC defined on Apple");
#endif
int main() { return 0; }
