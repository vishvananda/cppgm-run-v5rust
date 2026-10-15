// N3485 12.8p31: a converting constructor selected from a same-class
// prvalue is called even for an empty class argument; expect exit 0
int constructed;
struct R { int n; };
struct A {
  A() {}
  A(A&) {}
  A(R, int n = 4) { constructed += n; }
  operator R() { R r; r.n = 1; return r; }
};
void take(A) {}
int main() {
  A a = A();
  take(A());
  return constructed == 8 ? 0 : 1;
}
