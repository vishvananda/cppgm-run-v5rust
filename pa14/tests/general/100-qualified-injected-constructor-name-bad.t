// N3485 3.4.3.1p2: after a nested-name-specifier nominating a class, the
// class's own injected-class-name names its constructor, which is no type.
struct K { int x; };
template<class T> struct A { T a; };
int main() { return int(sizeof(K::K*)) + int(sizeof(A<int>::A*)); }
