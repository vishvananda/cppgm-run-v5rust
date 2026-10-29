// A later noexcept call still preserves its full-expression's result.
bool first();
bool second() noexcept;
struct Guard { ~Guard() {} };
bool evaluate() {
  Guard guard;
  if (first() || !second()) return true;
  return false;
}
