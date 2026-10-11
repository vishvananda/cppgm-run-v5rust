// N3485 15.3 (PA32 optimizer must preserve semantics): at -O1/-O2 the handler
// must see x == 5 (stored between the two throwing calls) and argc * 7
// computed in the handler, not a value CSE'd from the normal path;
// expect exit 0
int g;
__attribute__((noinline)) void thr(int i) { if (i == 3) throw 1; }
__attribute__((noinline)) int f1(int a) { int x; try { thr(a); x = 5; thr(a + 2); } catch (...) { return x; } return 99; }
__attribute__((noinline)) int f2(int a) { try { thr(a + 2); g = a * 7 + 1; } catch (...) { return a * 7; } return g; }
volatile int one = 1;
int main() { return f1(one) == 5 && f2(one) == 7 ? 0 : 1; }
