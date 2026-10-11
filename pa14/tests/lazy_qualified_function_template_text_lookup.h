#pragma once

#include "lazy_qualified_function_template_text_lookup_lib.h"

namespace lazy_lookup_user {

int run()
{
  int i = 0;
  int j = 2;
  return (lazy_lookup_lib::min_value)(i, j);
}

}  // namespace lazy_lookup_user
