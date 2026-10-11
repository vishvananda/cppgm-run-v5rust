#pragma once

extern int initializations;
extern int destructions;
extern "C" void abort();

struct shared
{
  shared() : value(++initializations) {}
  ~shared() { if(++destructions != 1) abort(); }
  int value;
};

inline shared & shared_value()
{
  static shared value;
  return value;
}
