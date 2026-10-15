// The caller's result is not a temporary owned by the return; expect exit 0
int live;
int copies;
int moves;
struct A {
  int n;
  A(int value) : n(value) { ++live; }
  A(const A& a) : n(a.n) { ++live; ++copies; }
  A(A&& a) : n(a.n) { ++live; ++moves; }
  ~A() { --live; }
};
A moved(A a) { return a; }
A copied(A& a) { return a; }
int main() {
  {
    A a = moved(A(7));
    if (a.n != 7 || live != 1 || moves != 1 || copies) return 1;
    A b = copied(a);
    if (b.n != 7 || live != 2 || moves != 1 || copies != 1) return 2;
  }
  return live;
}
