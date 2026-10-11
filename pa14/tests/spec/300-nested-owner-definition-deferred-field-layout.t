template<class T> struct A { struct B; };
template<class T> struct A<T>::B {};
struct C { A<int>::B x; };
int main() { C c; return sizeof(c); }
