// N3485 14.5.5.1, 14.8.2.5, 8.3.5p4: R(*)(Args...,...) matches only C-variadic
// function types; A<int(*)(int,int)> selects the non-variadic partial
// specialization (libstdc++ std::bind _Bind_check_arity); expect exit 0
template<class T> struct A { static const int v = 0; };
template<class R, class... Args> struct A<R(*)(Args...)> { static const int v = 1; };
template<class R, class... Args> struct A<R(*)(Args..., ...)> { static const int v = 2; };
int main() { return A<int(*)(int, int)>::v == 1 && A<int(*)(int, ...)>::v == 2 ? 0 : 1; }
