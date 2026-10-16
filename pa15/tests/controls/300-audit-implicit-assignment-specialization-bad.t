// expect reject
template<class T> struct A { int x; };
template<> A<int>& A<int>::operator=(const A<int>& a) { x = a.x; return *this; }
