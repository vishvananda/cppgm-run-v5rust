#include "200-host-member-function-pointer-equality.helper.h"
#include <cstring>
namespace member_equality {
int Base::f() { return data; }
int Base::g() { return data + 1; }
int Base::v() { return data + 2; }
Pointer host_pointer(int which)
{
  Pointer result = 0;
  if (which == 1 || which == 2) {
    // ABI 2.3: the function word alone determines nullness. The adjustment
    // may differ even though both pointers denote the null member function.
    long words[2] = {0, which == 1 ? 8 : -24};
    std::memcpy(&result, words, sizeof(result));
  }
  if (which == 3 || which == 4) result = &Base::f;
  if (which == 4) {
    long words[2];
    std::memcpy(words, &result, sizeof(result));
    words[1] += 8;
    std::memcpy(&result, words, sizeof(result));
  }
  if (which == 5) result = &Base::g;
  if (which == 6) result = &Base::v;
  return result;
}
bool host_check()
{
  for (int left = 0; left != 7; ++left)
    for (int right = 0; right != 7; ++right) {
      Pointer a = host_pointer(left), b = host_pointer(right);
      if (candidate_equal(a, b) != (a == b) ||
          candidate_unequal(a, b) != (a != b)) return false;
    }
  return true;
}
}
