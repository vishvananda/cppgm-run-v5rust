// N3485 15.4p14 (with CWG 1351): an implicit default constructor is noexcept
// when every subobject constructor it calls is noexcept (M()
// noexcept(sizeof(int) == 4)) and every brace-or-equal-initializer is
// non-throwing (quiet() noexcept, sizeof(thrower()) unevaluated);
// expect exit 0
struct M { M() noexcept(sizeof(int) == 4) {} };
int quiet() noexcept { return 5; }
int thrower() { return 4; }
struct K { M m; };
struct K3 { int x = quiet(); int y = sizeof(thrower()); };
static_assert(noexcept(K()) && noexcept(K3()), "");
int main() { return 0; }
