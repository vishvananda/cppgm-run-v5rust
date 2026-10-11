// N3485 focus: 12.6.2 [class.base.init] delegating constructor mem-initializer.
struct value {
  constexpr value() : member(0) {}
  constexpr value(int) : value() {}
  int member;
};
