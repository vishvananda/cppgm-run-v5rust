// N3485 12.2p5: a reference member's construction region starts after
// its temporary address, including within a tracked conditional aggregate.
// expect exit 0
int live;
struct S {
  int n;
  S(int v) : n(v) { ++live; }
  S(const S& s) noexcept : n(s.n) { ++live; }
  ~S() noexcept { --live; }
};
int read(const S& s) { return s.n; }
struct R { const S& a; int n; };
void run(bool first) {
  S prefix(0);
  const R& r = first ? R{S(read(S(1))), 0} : R{S(read(S(2))), 0};
  // The conditional's reference-member temporaries end with this full
  // expression. Only the complete R and prefix remain; r.a is not read.
  if (r.n != 0 || live != 1) throw 9;
}
int main() {
  run(true); if (live) return 1;
  run(false); return live ? 2 : 0;
}
