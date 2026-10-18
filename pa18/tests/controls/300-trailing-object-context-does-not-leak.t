// expect reject
// PA18: completing a nested return signature restores the outer query context.
struct Good {
  template<class T> auto nested(T) const -> auto (*)(T) -> decltype(this);
};

struct Bad {
  static auto invalid() -> decltype(this);
};
