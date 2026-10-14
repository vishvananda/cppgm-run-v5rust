// N3485 14.7.3/1,/16: template<> struct A<int>::B {...} explicitly specializes
// the member class B of A<int>; A<char>::B uses the primary member definition;
// expect exit 0
template<class T> struct A { struct B; };
template<> struct A<int>::B { int v; };
template<class T> struct A<T>::B { long v; };
int main() { A<int>::B b; b.v = 0; A<char>::B c; c.v = 0; return sizeof(b) == sizeof(int) && sizeof(c) == sizeof(long) ? 0 : 1; }
