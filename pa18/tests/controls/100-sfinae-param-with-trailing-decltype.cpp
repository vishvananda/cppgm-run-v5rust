// N3485 14.8.2p8: a deduction failure in a parameter type (typename T::nope*)
// of a function template whose trailing return type is decltype(t.m) on a
// function parameter removes that candidate; expect exit 0
struct HasM { int m; };
template<class T> auto sd1(T t, typename T::nope* = 0) -> decltype(t.m) { return 1; }
template<class T> auto sd1(T t, ...) -> decltype(t.m) { return 2; }
int main() { HasM h = { 4 }; return sd1(h, 0) == 2 ? 0 : 1; }
