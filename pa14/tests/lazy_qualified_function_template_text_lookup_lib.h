#pragma once

namespace lazy_lookup_lib {

template<class T>
T min_value(T a, T b)
{
  return a < b ? a : b;
}

}  // namespace lazy_lookup_lib
