// VALIDATION: compile-fail
// A restricted replacement conflicts with the unrestricted C++11
// declaration of operator new in <new>.

#include <new>

void* operator new(__SIZE_TYPE__) throw(int)
{
  for (;;) {}
}
