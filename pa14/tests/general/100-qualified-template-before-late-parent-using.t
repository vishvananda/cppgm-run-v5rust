namespace a { namespace b {
template<class T, class U> int f(T, U) { return 0; }
template<class T> int f(T value) { return ::a::b::f(value, 0); }
} using b::f; }
int main() { return a::b::f(0); }
