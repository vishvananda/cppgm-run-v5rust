// N3485 15.2/15.3: a detached constructor body retains its subobjects
// while a conditional local's source is built or its other arm throws.
// expect exit 0
int live;
struct S {
  int n;
  S(int v) : n(v) { ++live; }
  S(const S& v) noexcept : n(v.n) { ++live; }
  ~S() noexcept { --live; }
};
int read(const S& s) { return s.n; }
struct E { S a; int n; S b; };
struct F {
  S m;
  F(bool first) try : m(0) {
    try {
      S prefix(0);
      E e = first ? throw 9 : E{S(1), read(S(2)), S(3)};
      if (e.n != 2) throw 8;
    } catch (char) { throw 7; }
  } catch (int) { throw; }
};
int main() {
  try { F f(true); return 1; }
  catch (int n) { if (n != 9 || live) return 2; }
  { F f(false); if (live != 1) return 3; }
  return live ? 4 : 0;
}
