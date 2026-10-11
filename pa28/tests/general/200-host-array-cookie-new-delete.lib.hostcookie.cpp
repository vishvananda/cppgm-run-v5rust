#include "200-host-array-cookie-new-delete.helper.h"
#include <cstdlib>

int destroyed = 0;
size_type sized_released = 0;

void* Sized::operator new[](size_type size) { return std::malloc(size); }
void Sized::operator delete[](void* pointer, size_type size)
{
  sized_released = size;
  std::free(pointer);
}

Trivial* host_make_trivial(unsigned count) { return new Trivial[count]; }
Destructible* host_make_destructible(unsigned count) { return new Destructible[count]; }
Sized* host_make_sized(unsigned count) { return new Sized[count]; }
void host_drop_trivial(Trivial* array) { delete[] array; }
void host_drop_destructible(Destructible* array) { delete[] array; }
void host_drop_sized(Sized* array) { delete[] array; }
