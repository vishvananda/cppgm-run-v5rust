// N3485 14.7.3p12, 14.8.2.6, 14.8.2p8: matching template<> int f<int>(int*)
// against f(T) fails by substitution (int::type) and selects f(T*);
// expect exit 0
template <class T> typename T::type f(T) { return 1; }
template <class T> int f(T*) { return 2; }
template <> int f<int>(int*) { return 3; }
int main() { int x; return f(&x) == 3 ? 0 : 1; }
