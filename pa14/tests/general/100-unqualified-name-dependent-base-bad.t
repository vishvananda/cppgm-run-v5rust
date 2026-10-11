// N3485 14.6.2p3: unqualified lookup does not search the dependent base B<T>;
// x is undeclared; expect reject
template<class T> struct B { int x; };
template<class T> struct D : B<T> { int f() { return x; } };
int main() { D<int> d; return d.f(); }
