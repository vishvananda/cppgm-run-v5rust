// expect reject
template<class T> struct A { int x; };
template<> A<int>::~A() {}
