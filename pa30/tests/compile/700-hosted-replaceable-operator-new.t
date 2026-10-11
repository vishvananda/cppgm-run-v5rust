// VALIDATION: compile-pass
// A C++11 replacement matches the unrestricted declaration in <new>.
#include <new>
void* operator new(__SIZE_TYPE__) { for (;;) {} }
