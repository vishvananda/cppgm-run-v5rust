// Specifications use typed operands and their contextual conversion to bool.
void fractional() noexcept(0.5);
void compared() noexcept(1.0 < 2.0);
template<class T> void fixed() noexcept(1.0 < 2.0);
struct Context {
  int nonconst();
  // The frozen specification context uses Type::cv, not function_cv.
  void query() const noexcept(sizeof(this->nonconst()) == 4);
};
