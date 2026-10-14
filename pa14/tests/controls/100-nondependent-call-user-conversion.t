// N3485 14.6p8/p10, 13.3.3.1.2: a nondependent call in a template definition
// is viable through a converting constructor (int -> K); expect exit 0
struct K { K(int) {} };
int f(K) { return 1; }
template<class T> int g(T) { return f(0); }
int main() { return g(0) == 1 ? 0 : 1; }
