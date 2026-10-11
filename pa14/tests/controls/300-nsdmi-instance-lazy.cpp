// N3485 14.7.1p1 (CWG 1396): a class template specialization's default member
// initializers are instantiated only when a constructor uses them, so Bad<int>
// is complete (sizeof, a static member function named in a substitution)
// though its initializer is invalid for int; instances' initializers a
// constructor uses (implicit, user-provided, inheriting) are analyzed then,
// their names looked up in the template's definition context still (14.6.4.2:
// h(int), declared later, is not a candidate). expect exit 35
template <class T> struct Bad { int x = T::nope; static int f(); };
template <class T, class = decltype(Bad<T>::f())> int k(T) { return 1; }
int k(...) { return 2; }
struct V { static const int value = 4; };
int h(long) { return 1; }
template <class T> struct H { int m = h(0); };
int h(int) { return 2; }
template <class T> struct S { int x = T::value; };
template <class T> struct U { int x = 3; int y = sizeof(T); U() {} U(int) : x(1) {} };
template <class T> struct B { int b = 6; B(int) {} };
template <class T> struct D : B<T> { using B<T>::B; int d = 8; };
int main() {
  S<V> s; U<char> u1, u2(7); D<int> d(1); H<int> hm;
  return k(0) + (int)sizeof(Bad<int>) - 4 + s.x + u1.x + u1.y + u2.x + u2.y + d.b + d.d + hm.m * 10;
}
