// N3485 14.8.2.2, 14.8.2.5p10: taking the address of f(T&&) for target
// int(*)(int&) deduces T = int&; expect exit 0
template<class T> int f(T&&) { return 1; }
int main() { int (*p)(int&) = f; int i = 0; return p(i) == 1 ? 0 : 1; }
