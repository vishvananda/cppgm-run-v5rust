#include "200-host-array-new-bad-length-unwind.helper.h"
#include <stdlib.h>

// Without hosted headers: the C++ ABI names the type of the exception being
// handled, and libstdc++ defines std::bad_array_new_length's type_info.
namespace std { class type_info; }
namespace __cxxabiv1 { extern "C" std::type_info* __cxa_current_exception_type() throw(); }
extern "C" char _ZTISt20bad_array_new_length;

int live = 0;
int allocations = 0;

void* operator new[](size_type size)
{
  ++allocations;
  return malloc(size ? size : 1);
}

void operator delete[](void* storage) noexcept { free(storage); }

template <class Count>
int rejected(int (*allocate)(Count), Count count)
{
  const int before = allocations;
  try { allocate(count); }
  catch (...) {
    if (static_cast<void*>(__cxxabiv1::__cxa_current_exception_type()) !=
        &_ZTISt20bad_array_new_length) return 3;
    return live == 0 && allocations == before ? 0 : 2;
  }
  return 1;
}

int host_check()
{
  int status = 0;
  status |= rejected(student_signed, -1) << 0;
  status |= rejected(student_long, -5L) << 2;
  status |= rejected(student_sized, ~size_type(0)) << 4;
  status |= rejected(student_class, -2L) << 6;
  status |= rejected(student_class, long(size_type(1) << 61)) << 8;
  if (student_signed(3) != 3 || student_class(2) != 7 || live != 0) status |= 1 << 10;
  return status;
}
