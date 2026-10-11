// N3485 10.2, 14.6.2p3, 11.2p6: in D<T> : B<int>, B<T>, B<int>::m names the
// member of the B<int> base (the dependent base B<T> is not searched at
// definition; with T = char no ambiguity); expect exit 0
template<class T> struct B { int m; };
template<class T> struct D : B<int>, B<T> { int f() { return B<int>::m + B<T>::m; } };
int main() { D<char> d; d.B<int>::m = 3; d.B<char>::m = 4; return d.f() == 7 ? 0 : 1; }
