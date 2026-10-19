// N3485 8.5.1/12.2/15.2: a conditional aggregate member retains its
// initializer temporaries and completed prefix until initialization ends.
// expect exit 0
int live, fail;
struct S {
  int n;
  S(int v) : n(v) { ++live; }
  S(const S& v) noexcept : n(v.n) { ++live; }
  ~S() noexcept { --live; }
};
int read(const S& s) { return s.n; }
int step() { if (fail) throw 9; return 0; }
struct E { S a; int n; S b; };
struct H { E e; S s; int n; };
void run(bool first) {
  S prefix(0);
  H h = {first ? E{S(1), read(S(2)), S(3)}
               : E{S(4), read(S(5)), S(6)}, S(7), step()};
  if (h.s.n != 7) throw 8;
}
int main() {
  for (int i = 0; i != 4; ++i) {
    fail = i / 2;
    try { run(i % 2); if (fail) return 1; }
    catch (int n) { if (!fail || n != 9) return 2; }
    if (live) return 3;
  }
  return 0;
}
