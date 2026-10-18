// expect exit 0
// Auto deduction and copying preserve each expression's closure type. Lexical
// numbering groups equal parameter signatures, independently of return types.
template<class F> int call(F const& f) { return f(3); }
inline int run() {
  auto a = [](int x) { return x + 1; };
  auto b = [](char x) { return x + 2; };
  auto c = [](int x) noexcept { return x + 3; };
  auto copy = a;
  return call(copy) + call(b) + call(c);
}
int main() { return run() == 15 ? 0 : 1; }
