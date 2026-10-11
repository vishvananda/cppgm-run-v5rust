// N3485 14.2p4: T::f<int>(0) without the template keyword parses as (T::f <
// int) > (0), ill-formed; expect reject
struct X { template<class U> static int f(int) { return 1; } };
template<class T> int g() { return T::f<int>(0); }
int main() { return g<X>(); }
