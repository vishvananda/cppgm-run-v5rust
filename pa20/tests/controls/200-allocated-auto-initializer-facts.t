// N3485 5.3.4p2, 7.1.6.4p6: allocated placeholders use ordinary
// deduction, retain their initializer facts, and initialize one object.
// expect exit 0
int calls;
int next() { return ++calls; }
int increment(int n) { return n + 1; }
struct A {
  int n;
  A(int v) : n(v) {}
  A(const A& a) : n(a.n + 1) {}
};
template<class T> int read(T n) {
  auto p = new const auto(n);
  int result = *p;
  delete p;
  return result;
}
int main() {
  auto p = new auto(next());
  auto b = new auto(true);
  auto c = new auto(char(3));
  int array[2] = {4, 5};
  auto a = new auto(array);
  auto f = new auto(increment);
  auto l = new auto([] { static int n; return ++n; });
  A original(6);
  auto copy = new auto(original);
  int ok = *p == 1 && calls == 1 && *b && *c == 3
      && (*a)[1] == 5 && (*f)(6) == 7 && (*l)() == 1 && (*l)() == 2
      && copy->n == 7 && read(8L) == 8;
  delete p; delete b; delete c; delete a; delete f; delete l; delete copy;
  return ok ? 0 : 1;
}
