// N3485 14.8.2p8, 5.3.4: a new-expression whose constructor call fails (S has
// no S(Plain)) in a deduced return type is a substitution failure;
// expect exit 0
template <class T> T&& declval();
struct Plain { };
struct S { S(int); };
template <class T> auto f(int) -> decltype(new S(declval<T>())) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<Plain>(0)) == sizeof(long) ? 0 : 1; }
