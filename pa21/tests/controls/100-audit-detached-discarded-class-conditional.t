// N3485 12.2/15.2: a discarded conditional in a detached constructor
// body opens its segment after result storage and destroys the selected arm.
// expect exit 0
int live;
struct S {
  int n;
  S(int v) : n(v) { ++live; }
  S(const S& s) noexcept : n(s.n) { ++live; }
  ~S() noexcept { --live; }
};
int read(const S& s) { return s.n; }
struct E { S a; int n; S b; };
struct F {
  S m;
  F(bool first) : m(0) {
    S prefix(0);
    (first ? E{S(1), read(S(2)), S(3)} : E{S(4), read(S(5)), S(6)});
    if (live != 2) throw 9;
  }
};
int main() {
  { F f(true); if (live != 1) return 1; }
  if (live) return 2;
  { F f(false); if (live != 1) return 3; }
  return live ? 4 : 0;
}
