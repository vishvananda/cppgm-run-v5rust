// VALIDATION: compile-pass

#include "300-lazy-nested-class-enclosing-alias-lookup.h"

int main()
{
  return boost_like::json::object::table::allocate() ? 1 : 0;
}
