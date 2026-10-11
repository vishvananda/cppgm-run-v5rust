// VALIDATION: compile-pass
// N3485 focus: 14.4 [temp.type]

#include "../support.h"

template<typename T, typename U = int>
struct pair_like
{
};

typedef pair_like<char> left_t;
typedef pair_like<char, int> right_t;

static_assert(is_same<left_t, right_t>::value,
              "defaulted and explicit equivalent template arguments should name the same type");

int main()
{
  return 0;
}
