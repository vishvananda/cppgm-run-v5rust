#pragma once

struct input_base { template<class T> input_base & operator>>(T &); };
template<class> struct input : input_base {
  template<class T> input_base & operator>>(T & v)
  { return input_base::operator>>(v); }
};
