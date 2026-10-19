// expect exit 0
// Completed rows and constructor members retain distinct construction owners.
int live, copies, stop;
struct S {
  int value;
  S(int n) : value(n) { ++live; }
  S(const S& other) : value(other.value) {
    if (++copies == stop) throw copies;
    ++live;
  }
  ~S() noexcept { --live; }
};
struct H {
  S first;
  S array[3];
  S last;
  H(const S& x) : first(x), array{x,x,x}, last(x) {}
};
struct Matrix { S rows[2][2]; S last; };
struct Inner { S row[2]; };
struct Nested { Inner inner; S last; };
void probe(int mode, const S& x) {
  if (mode == 0) { H h(x); }
  if (mode == 1) { Matrix m={{{x,x},{x,x}},x}; }
  if (mode == 2) { Nested n={{{x,x}},x}; }
}
int main() {
  for (int mode=0; mode!=3; ++mode) {
    for (stop=0; stop!=7; ++stop) {
      copies=0;
      { S x(9);
        try { probe(mode,x); }
        catch (int n) { if (n!=stop) return 1; }
        if (live!=1) return 2;
      }
      if (live) return 3;
    }
  }
  return 0;
}
