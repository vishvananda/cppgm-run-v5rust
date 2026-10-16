// N3485 8.3.6p6: valid defaults remain on their original member declaration
// across renamed outside definitions and specializations; expect exit 0.
template<class T> struct A {
  int value;
  A(int n = sizeof(T));
  int f(int x = sizeof(T));
  static int g(int x = sizeof(T));
  struct B { int f(int x = sizeof(T)); };
};
template<class U> A<U>::A(int n) : value(n) {}
template<class U> int A<U>::f(int x) { return value + x; }
template<class U> int A<U>::g(int x) { return x; }
template<class U> int A<U>::B::f(int x) { return x; }
struct Plain { int f(int); };
int Plain::f(int x = 3) { return x; }
int main() {
  A<int> a;
  A<long> b;
  A<int>::B nested;
  Plain plain;
  return a.f() + b.f() + a.g() + b.g() + nested.f() + plain.f() - 43;
}
