// N3485 14.8.2p8: a cast to an ambiguous base inside decltype in a function
// template signature is a substitution failure (the other overload is chosen),
// not a hard error; expect exit 0
template<class T> T* ptr();
struct A { int v; };
struct I1 : A { };
struct I2 : A { };
struct M : I1, I2 { };
template<class T> char up(int, decltype((A*)ptr<T>())* = 0);
template<class T> long up(...);
int main() { return sizeof(up<M>(0)) == sizeof(long) && sizeof(up<I1>(0)) == 1 ? 0 : 1; }
