// Branch defaults live through base construction, then leave together; expect exit 0
int live;
int made;
int destroyed;
int bad;
struct T {
  T() { ++live; ++made; }
  ~T() { --live; }
};
struct A {
  int n;
  A(const T& = T()) : n(live) { if (live != 1) ++bad; }
  ~A() { ++destroyed; }
};
struct B : A {
  B(bool c) : A(c ? A() : A()) { if (live || n != 1) ++bad; }
};
struct Plain {
  int n;
  Plain(const T& = T()) : n(live) { if (live != 1) ++bad; }
};
struct C : Plain {
  C(bool c) : Plain{c ? Plain() : Plain()} { if (live || n != 1) ++bad; }
};
int main() {
  {
    B b(true), d(false);
    C c(true), e(false);
    if (live || made != 4 || destroyed || bad) return 1;
  }
  return live || destroyed != 2 || bad;
}
