#include "200-host-virtual-base-entry-arguments.helper.h"

struct Both : Left, Right
{
  int both;
  Both() : both(shared + left + right) { trace[trace_count++] = 4; }
  ~Both() { trace[trace_count++] = -4; }
};

int host_build()
{
  int result = 0;
  {
    Both both;
    Left& left = both;
    Right& right = both;
    if (both.shared != 7 || both.left != 8 || both.right != 9 || both.both != 24 ||
        &left.shared != &right.shared)
      result = 1;
  }
  static const int want[] = {1, 2, 3, 4, -4, -3, -2, -1};
  for (int i = 0; i < 8; ++i)
    if (trace_count != 8 || trace[i] != want[i]) return 2;
  return result;
}

int host_take(Value value) { return value.shared * 10 + value.value; }

int host_drive_value()
{
  Value value;
  value.shared = 4;
  value.value = 5;
  return our_take(value) == 45 ? 0 : 3;
}
