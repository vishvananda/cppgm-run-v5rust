// N3485 8.2p2, 5.3.3p1, 14.8.2p8: sizeof(typename T::tn()) has a type-id
// operand (function type int()), so it is ill-formed / a substitution failure,
// not sizeof(int); expect exit 0
struct Good { typedef int tn; };
template <class T> auto f(int) -> decltype(sizeof(typename T::tn()), char()) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<Good>(0)) == sizeof(long) ? 0 : 1; }
