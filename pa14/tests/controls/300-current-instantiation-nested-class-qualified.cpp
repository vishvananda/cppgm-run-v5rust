// N3485 14.6.2.1p1, p3 (example): inside nested class B, A<T>::B::X names a
// member of the current instantiation and needs no typename; expect exit 0
template<class T> struct A { struct B { typedef int X; A<T>::B::X a; }; };
int main() { A<int>::B b; b.a = 2; return b.a == 2 ? 0 : 1; }
