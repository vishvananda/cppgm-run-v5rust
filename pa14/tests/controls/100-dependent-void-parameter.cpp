// N3485 8.3.5p4, 14.8.2p8: void(T) with T = void is not an empty parameter
// list (only non-dependent void is): a substitution failure; expect exit 0
template <class T> char f(void (*)(T)) { return 0; }
template <class T> long f(...) { return 0; }
int main() { return sizeof(f<void>(0)) == sizeof(long) ? 0 : 1; }
