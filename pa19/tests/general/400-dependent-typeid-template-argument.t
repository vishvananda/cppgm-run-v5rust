template<class A, class B> struct Pair {};
template<class T, bool = true> struct S {};
template<class T> struct S<T, true> { typedef int type; };
template<class T>
using X = Pair<int, typename S<T>::type>;
using Y = X<int>;
int main() { return 0; }
