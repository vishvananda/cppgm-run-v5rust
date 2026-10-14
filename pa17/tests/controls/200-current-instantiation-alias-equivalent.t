// N3485 14.6.2.1p1: A<Id<T>> (an equivalent alias template specialization)
// names the current instantiation, so A<Id<T>>::Z needs no typename;
// expect exit 0
template<class T> using Id = T;
template<class T> struct A { typedef int Z; A<Id<T> >::Z z; };
int main() { A<int> a; a.z = 3; return a.z == 3 ? 0 : 1; }
