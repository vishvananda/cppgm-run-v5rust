#pragma once

struct base {
  constexpr base() {}
  virtual int get() const = 0;
};
struct object : base {
  constexpr object() : base() {}
  int get() const { return 7; }
};
union holder {
  object value;
  constexpr holder() : value() {}
};
extern holder instance;
