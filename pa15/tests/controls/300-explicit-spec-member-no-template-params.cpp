// N3485 14.7.3p5, 14.5.1p3: in template<> int P<int>::f() no template
// parameter is in scope, and in template<class V> int P<V>::g() only V is; T
// names ::T (double) in both; expect exit 0
double T;
template <class T> struct P { int f(); int g(); };
template <class T> int P<T>::f() { return 7; }
template <> int P<int>::f() { return sizeof(T); }
template <class V> int P<V>::g() { return sizeof(T); }
int main() { P<int> p; return (p.f() == sizeof(double) && p.g() == sizeof(double)) ? 0 : 1; }
