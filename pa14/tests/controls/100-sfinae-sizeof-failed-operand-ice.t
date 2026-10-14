// N3485 14.8.2p8: an invalid expression inside sizeof in a function template
// signature (T::size missing) is a substitution failure; expect exit 0
struct Plain { };
template <class T> char f(int (*)[sizeof(T::size)]) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<Plain>(0)) == sizeof(long) ? 0 : 1; }
