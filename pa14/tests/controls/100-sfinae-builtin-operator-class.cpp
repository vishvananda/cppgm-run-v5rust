// N3485 14.8.2p8: an invalid built-in operator expression (class + int, no
// operator+) in a deduced return type is a substitution failure, not a hard
// error; expect exit 0
template <class T> T&& declval();
struct A {};
template <class T> auto f(int) -> decltype(declval<T>() + 1, char()) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<A>(0)) == sizeof(long) ? 0 : 1; }
