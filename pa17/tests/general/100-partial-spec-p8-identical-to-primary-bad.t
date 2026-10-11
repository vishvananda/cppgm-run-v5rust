// N3485 14.5.5p8 (third bullet): a partial specialization argument list shall
// not be identical to the primary implicit argument list; expect reject
template<class T, class U> struct S {};
template<class T, class U> struct S<T, U> {};
int main() { return 0; }
