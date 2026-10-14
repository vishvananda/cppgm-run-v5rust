// N3485 14.8.2.1p6: an overload set argument (g(int), g(double)) for a
// parameter T(*)(int) deduces T by trying each member; exactly g(int) deduces;
// expect exit 0
int g(int) { return 1; }
int g(double) { return 2; }
template<class T> int by_pointer(T (*p)(int)) { return p(0); }
int main() { return by_pointer(g) == 1 ? 0 : 1; }
