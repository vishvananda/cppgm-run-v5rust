// N3485 14.8.2/8: T{} for an aggregate with an uninitialized reference member
// is an invalid expression in the immediate context of substitution, a
// deduction failure (SFINAE), not a hard error; expect exit 0
struct S { int& r; };
template<class T> decltype(T{}, char()) f(int);
template<class T> long f(...);
int main() { return sizeof(f<S>(0)) == sizeof(long) ? 0 : 1; }
