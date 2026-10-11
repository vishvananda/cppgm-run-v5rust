// VALIDATION: compile-pass
// N3485 focus: 14.6.2 [temp.dep]

#include "support.h"

template<typename T>
struct Holder
{
  typedef int type;
  Holder<T>::type value;

  Holder() : value(3) {}
};

static_assert(is_same<Holder<int>::type, int>::value, "current instantiation type should resolve");

int main()
{
  Holder<int> h;
  return h.value == 3 ? 0 : 1;
}
