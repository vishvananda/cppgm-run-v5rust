// expect exit 134: the course runtime's terminate exits with status 134.
// A catch parameter throws during retirement; destroying the original exception
// then throws again while unwinding. The outer handler must not catch it.
struct E {
  int copy;
  E() : copy(0) {}
  E(const E&) : copy(1) {}
  ~E() noexcept(false) { throw copy ? 5 : 6; }
};
int main() {
  try {
    try { throw E(); }
    catch (E e) {}
  } catch (int) { return 0; }
  return 1;
}
