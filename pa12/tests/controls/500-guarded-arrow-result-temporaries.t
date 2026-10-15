// expect exit 0
int destroyed;
struct A { int n; };
A a = {3};
struct Q {
  ~Q() noexcept(false) { destroyed += 1; }
  A* operator->() { return &a; }
};
struct P {
  ~P() { destroyed += 10; }
  Q operator->() { return Q(); }
};
int f(bool b) { return (b ? P()->n : 0) + (b ? 0 : P()->n); }
int main() {
  if (f(true) != 3 || destroyed != 11) return 1;
  if (f(false) != 3 || destroyed != 22) return 2;
  return 0;
}
