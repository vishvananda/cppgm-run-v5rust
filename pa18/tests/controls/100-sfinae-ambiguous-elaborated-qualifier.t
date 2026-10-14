// N3485 10.2p6, 14.8.2p8: struct T::marker naming a member found in two bases
// is ambiguous, a deduction failure; expect exit 0
struct L { struct marker {}; };
struct R { struct marker {}; };
struct D : L, R {};
template<class T> char f(decltype((struct T::marker*)0) = 0);
template<class T> long f(...);
int main() { return sizeof(f<D>(0)) == sizeof(long) ? 0 : 1; }
