// N3485 5.2.3p1, 5.2.9p4, 3.10p4: T(x) and static_cast<T>(x) with T = const A
// yield a prvalue of type const A, so the const member function overload is
// chosen; expect exit 0
struct A { int v; A(int x) : v(x) {} int f() const { return 1; } int f() { return 2; } };
typedef const A CA;
int main() { return (CA(3).f() == 1 && static_cast<const A>(3).f() == 1) ? 0 : 1; }
