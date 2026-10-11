// N3485 14.7.3p1, p5 (member enumeration), 7.2p3: template<> enum A<int>::E :
// int {...} explicitly specializes an opaque member enumeration; expect exit 0
template<class T> struct A { enum E : T; };
template<> enum A<int>::E : int { eint = 3 };
int main() { return A<int>::eint == 3 ? 0 : 1; }
