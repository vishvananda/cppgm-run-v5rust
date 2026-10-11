// N3485 5.2.5p5, 3.4.5p4, 14.8.2p8: in x.T::m, T must name the object's class
// or a base; another class fails deduction; expect exit 0
template<class T> T&& declval();
struct S { int x; };
struct U { int x; };
template<class T> auto f(int) -> decltype(declval<S>().T::x, char());
template<class T> long f(...);
int main() { return sizeof(f<U>(0)) == sizeof(long) ? 0 : 1; }
