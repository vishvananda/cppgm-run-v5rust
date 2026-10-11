// N3485 14.7.2p1, p8: `template int S<int>::x;` explicitly instantiates
// (defines) a static data member of a class template; expect exit 0
template<class T> struct S { static T x; };
template<class T> T S<T>::x = T(6);
template int S<int>::x;
int main() { return S<int>::x == 6 ? 0 : 1; }
