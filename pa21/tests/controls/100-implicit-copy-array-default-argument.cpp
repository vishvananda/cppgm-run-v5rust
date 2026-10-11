// N3485 12.8p15, 8.3.6p9, 15.2p2, 12.2p3: an implicit copy constructor copies
// an array member element by element, each copy constructor call making the
// temporary its default argument creates; when a copy or that temporary's
// construction throws, the elements already copied and the temporaries are
// destroyed; expect exit 0
int live, countdown;
void tick() { if (countdown > 0 && --countdown == 0) throw 1; }
struct P { int id; P(int i) : id(i) { tick(); ++live; } ~P() { --live; } };
struct R {
  int r;
  R() : r(0) { ++live; }
  R(const R& o, const P& p = P(4)) : r(o.r + p.id) { tick(); ++live; }
  ~R() { --live; }
};
struct C {
  int c;
  C() : c(0) { ++live; }
  C(const C& o, P p = P(3)) : c(o.c + p.id) { tick(); ++live; }
  ~C() { --live; }
};
struct HR { R arr[3]; int tail; };
struct HC { C arr[2]; R r; };
int main() {
  for (int k = 1; k < 12; ++k) {
    countdown = k; live = 0;
    try { HR h; HR copy = h; if (copy.arr[2].r != 4) return 50; } catch (int) {}
    if (live != 0) return k;
  }
  for (int k = 1; k < 12; ++k) {
    countdown = k; live = 0;
    try { HC h; HC copy = h; if (copy.r.r != 4) return 60; } catch (int) {}
    if (live != 0) return 20 + k;
  }
  return 0;
}
