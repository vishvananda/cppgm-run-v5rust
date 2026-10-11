#pragma once

namespace H {
template<class T>
T min(T a, T b)
{
  return a < b ? a : b;
}

template<class T>
struct Box {
  static T f(T a, T b)
  {
    return (H::min)(a, b);
  }
};
}
