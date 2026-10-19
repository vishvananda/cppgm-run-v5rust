// N3485 12.2p5/15.2: a complete aggregate bound through a subobject
// retains its address while initializer temporaries are destroyed.
// expect exit 0
int live, fail;
struct S {
  int n;
  S(int v) : n(v) { ++live; }
  S(const S& s) noexcept : n(s.n) { ++live; }
  ~S() noexcept(false) { --live; if (n == fail) throw n; }
};
int read(const S& s) { return s.n; }
struct E { S a; int n; S b; };
struct H { E e; S s; int n; };
void run(bool first) {
  S prefix(0);
  const S& r = (first ? H{{S(1), read(S(2)), S(3)}, S(4), 0}
                      : H{{S(5), read(S(6)), S(7)}, S(8), 0}).e.a;
  if (r.n != (first ? 1 : 5)) throw 9;
}
int main() {
  for (int i = 0; i != 4; ++i) {
    bool first = i % 2;
    fail = i < 2 ? -1 : (first ? 2 : 6);
    try { run(first); if (fail != -1) return 1; }
    catch (int n) { if (n != fail) return 2; }
    if (live) return 3;
  }
  return 0;
}
