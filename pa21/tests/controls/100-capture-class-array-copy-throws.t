// N3485 5.1.2p21, 15.2p2: a closure's class array member is copied element by
// element; when an element's copy throws, the elements already copied are
// destroyed, then the objects alive in the try block, for short and long
// arrays; expect exit 0
int live, countdown;
struct C {
  int v;
  C(int x) : v(x) { ++live; }
  C(const C& o) : v(o.v) { if (countdown > 0 && --countdown == 0) throw 1; ++live; }
  ~C() { --live; }
};
struct D { D() { ++live; } ~D() { --live; } };
int three(int k) {
  countdown = k; live = 0;
  int r = 3;
  try { D d; C a[3] = { 1, 2, 3 }; r = [a] { return a[2].v; }(); } catch (int) {}
  return r == 3 && live == 0 ? 0 : 1;
}
int ten(int k) {
  countdown = k; live = 0;
  int r = 10;
  try { D d; C a[10] = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 }; r = [a] { return a[9].v; }(); } catch (int) {}
  return r == 10 && live == 0 ? 0 : 1;
}
int main() {
  for (int k = 1; k <= 5; ++k)
    if (three(k)) return k;
  for (int k = 1; k <= 12; ++k)
    if (ten(k)) return 10 + k;
  return 0;
}
