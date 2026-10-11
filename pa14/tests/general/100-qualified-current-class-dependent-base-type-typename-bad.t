// N3485 14.6.2.1p4-p5, 14.6p2: D::Z found only in dependent base B<T> is a
// member of an unknown specialization; typename required (g++ rejects; clang
// accepts); expect reject
template<class T> struct B { typedef int Z; };
template<class T> struct D : B<T> { D::Z z; };
int main() { return 0; }
