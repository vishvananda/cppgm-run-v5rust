// VALIDATION: compile-pass

template<class D> struct base {
  constexpr const D& self() const { return static_cast<const D&>(*this); }
};
struct derived : base<derived> {
  int value;
  constexpr derived() : value(3) {}
};
static_assert(static_cast<const base<derived>&>(derived()).self().value == 3, "");
int main() { return 0; }
