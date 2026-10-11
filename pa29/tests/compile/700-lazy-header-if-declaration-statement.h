#pragma once

namespace lazy_header_if_declaration_statement {

template<int N>
struct checker {
  int value;

  checker()
    : value(N)
  {
  }

  void run()
  {
    if (value)
      static_assert(N >= 0, "expected non-negative template value");
    else
      static_assert(sizeof(int) > 0, "expected ordinary declaration-statement");

    ++value;
  }
};

inline int run_case()
{
  checker<1> c;
  c.run();
  return c.value;
}

}
