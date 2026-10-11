// N3485 5.19p2 (lvalue-to-rvalue conversion of a non-constant x passed to...),
// 7.1.5p5: f(x) is not a constant expression; must be rejected
int x = 1;
constexpr bool f(...) { return true; }
static_assert(f(x), "");
int main() { return 0; }
