// N3485 13.3.1p4, 7.3.3p15: a using-declared base member is treated as a
// member of the derived class for its implicit object parameter; in a class
// template member f(1L) calls B::f(long) (exact), not ambiguous; expect exit 0
struct B { int f(long) { return 1; } };
template<class T> struct D : B { using B::f; int f(char) { return 3; } int get() { return f(1L); } };
int main() { D<int> d; return d.get() == 1 ? 0 : 1; }
