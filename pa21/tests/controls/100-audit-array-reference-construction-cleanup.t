// expect exit 0
// A static aggregate's array-reference temporary keeps its Storage path while
// prefix cleanup guards construction; a failed attempt aborts the static guard.
int live, attempts, order;
struct Verify {
  ~Verify() { if (live || order!=1321) __builtin_abort(); }
} verify;
struct Element {
  int value;
  Element(int n) : value(n) {
    if (n==2 && ++attempts==1) throw n;
    ++live;
  }
  ~Element() noexcept { --live; order=order*10+value; }
};
struct References { const Element (&array)[2]; const Element& last; };
void initialize() {
  static References references={{Element(1),Element(2)},Element(3)};
  (void)references;
}
int main() {
  try { initialize(); return 1; }
  catch (int n) { if (n!=2 || live || order!=1) return 2; }
  initialize();
  initialize();
  return live==3 && attempts==2 ? 0 : 3;
}
