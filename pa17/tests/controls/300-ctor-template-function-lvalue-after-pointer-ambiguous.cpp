// N3485 13.3.1.7/14.8.2.1/3: Fn b(free_fn) deduces F = int(&)(int) from the
// one constructor template; a specialization instantiated earlier for F =
// int(*)(int) is not an extra candidate, so the call is not ambiguous
// (std::function<int(int)> from &f then from f hits this); expect exit 0
struct Fn { int (*p)(int); template<class F> Fn(F&& f) : p(f) {} };
int free_fn(int x) { return x * 3; }
int main() { Fn a(&free_fn); Fn b(free_fn); return a.p(1) + b.p(2) == 9 ? 0 : 1; }
