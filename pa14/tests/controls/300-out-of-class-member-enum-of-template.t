// N3485 14.5.1.4, 7.2p3: an opaque member enumeration of a class template may
// be defined outside the class (template<class U> enum A<U>::G : int {...});
// expect exit 0
template<class T> struct A { enum G : int; };
template<class U> enum A<U>::G : int { g1 = sizeof(U), g2 };
int main() { return A<int>::g2 == 5 ? 0 : 1; }
