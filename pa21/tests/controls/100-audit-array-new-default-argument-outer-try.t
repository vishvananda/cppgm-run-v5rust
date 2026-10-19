// expect exit 0
int alive, arguments, attempts, outer;
struct H { H() { ++outer; } ~H() noexcept { --outer; } };
struct D {
  D() { if (++attempts == 2) throw 7; ++arguments; }
  ~D() noexcept(false) { --arguments; }
};
struct C {
  int value;
  C(D = D()) noexcept : value(3) { ++alive; }
  ~C() noexcept(false) { --alive; }
};
int main() {
  { H h;
    try { C* p = new C[9]; delete[] p; return 1; }
    catch (int n) { if (n != 7 || alive || arguments || outer != 1) return 2; }
  }
  return outer;
}
