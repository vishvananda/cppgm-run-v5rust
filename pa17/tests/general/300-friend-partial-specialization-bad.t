// N3485 14.5.4p8: friend declarations shall not declare partial
// specializations; expect reject
template <class T> class A { };
class X { template <class T> friend class A<T*>; };
int main() { return 0; }
