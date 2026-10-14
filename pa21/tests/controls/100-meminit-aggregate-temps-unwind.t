// N3485 15.2p1/12.2p3: when a member copy throws inside mem-initializer
// q(Q2{W(1),W(2)}), the full-expression's temporaries W(1),W(2) are destroyed;
// expect exit 0
static int live, countdown;
struct W { W(int) noexcept { ++live; } W(const W&) { if (countdown > 0 && --countdown == 0) throw 5; ++live; } ~W() { --live; } };
struct Q2 { W a; W b; };
struct Holder { Q2 q; Holder() : q(Q2{W(1), W(2)}) { } };
int main() {
  for (int k = 1; k < 4; ++k) { countdown = k; live = 0; try { Holder h; } catch (int) {} if (live) return 10 + live * 3 + k; }
  return 0;
}
