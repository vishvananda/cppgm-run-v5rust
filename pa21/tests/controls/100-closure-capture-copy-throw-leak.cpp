// N3485 5.1.2p21 + 15.2p2: if copying the second by-copy capture throws, the
// already-copied closure member is destroyed; expect exit 0
int live, made, throw_at = -1;
struct C { C() { ++live; } C(const C&) { if (made++ == throw_at) throw 8; ++live; } ~C() { --live; } };
int main() {
  int r = 0;
  C a, b;
  throw_at = 1;
  try { auto f = [a, b]() { return 0; }; r = 100; } catch (int) { r = live; }
  return r == 2 ? 0 : 1;
}
