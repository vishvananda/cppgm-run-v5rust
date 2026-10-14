// N3485 7.1.6.3p2, 14.8.2p8: an elaborated-type-specifier (struct T::marker)
// that resolves to a typedef-name is ill-formed, a deduction failure;
// expect exit 0
struct L { typedef int marker; };
template<class T> char f(decltype((struct T::marker*)0) = 0);
template<class T> long f(...);
int main() { return sizeof(f<L>(0)) == sizeof(long) ? 0 : 1; }
