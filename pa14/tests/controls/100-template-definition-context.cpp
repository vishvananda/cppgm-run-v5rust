// N3485 14.6.3p1, 14.6.4p1: a nondependent operator/name in a template binds
// at the definition; later overloads are not candidates; expect exit 0
struct K { int v; };
int operator+(K, long) { return 1; }
template<class T> int g(T) { K k; return k + 0; }
int operator+(K, int) { return 2; }
int main() { return g(0) == 1 ? 0 : 1; }
