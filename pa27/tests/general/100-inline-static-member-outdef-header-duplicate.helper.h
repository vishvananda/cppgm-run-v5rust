#pragma once

struct InlineStaticMemberOutdef
{
  static inline int value();
};

int InlineStaticMemberOutdef::value()
{
  return 4;
}
