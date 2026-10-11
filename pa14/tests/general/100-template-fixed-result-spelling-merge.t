// A fixed trailing decltype and an ordinary result spell the same type.
template<class T> auto result(T, int value) -> decltype(value);
template<class U> int result(U, int value) { return value; }
template<class T> auto literal(T) -> decltype(0);
template<class U> int literal(U) { return 9; }
int main() { return result(0, 7) == 7 && literal(0) == 9 ? 0 : 1; }
