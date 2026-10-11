// VALIDATION: compile-pass
// N3485 focus: 14.6.2 [temp.dep]

#include "../support.h"

struct source_type
{
  typedef int type;
};

template<typename T>
struct wrapper
{
  typedef typename T::type nested_type;
};

static_assert(is_same<wrapper<source_type>::nested_type, int>::value,
              "dependent typename should resolve to nested type");

int main()
{
  return 0;
}
