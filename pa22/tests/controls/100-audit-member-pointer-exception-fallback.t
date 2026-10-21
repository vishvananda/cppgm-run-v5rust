// expect exit 0
// Exception continuations return literal zero; language null member pointers
// remain -1. Compare exact LowIR as well as the reachable results.
struct A { int n; };
int A::* guarded(bool take) noexcept {
  try { if (take) throw 7; return nullptr; }
  catch (int) { return &A::n; }
}
int A::* rethrowing() {
  try { throw 7; }
  catch (...) { throw; }
}
int main() {
  A a{9};
  if (guarded(false) != nullptr || a.*guarded(true) != 9) return 1;
  try { rethrowing(); }
  catch (int n) { return n == 7 ? 0 : 2; }
  return 3;
}
