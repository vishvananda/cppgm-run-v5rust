// N3485 14.8.2p8, 14.6p2: in I<T::value>, T::value naming a type (not a value)
// is a substitution failure, not a hard error; expect exit 0
template <int> struct I {};
struct V { typedef int value; };
template <class T> char f(I<T::value>*) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<V>(0)) == sizeof(long) ? 0 : 1; }
