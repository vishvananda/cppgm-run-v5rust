// VALIDATION: compile-pass
// N3485 focus: 14.5.7 [temp.alias]

#include "support.h"

template<typename T>
struct wrap
{
  typedef T type;
};

template<typename T>
using alias_t = typename wrap<T>::type;

static_assert(is_same<alias_t<int>, int>::value, "alias template should substitute transparently");

int main()
{
  return 0;
}
