// N3485 focus: 8.3.5 [dcl.fct], 5.19 [expr.const]
template<unsigned, class T> int read(T&) { return 0; }
template<unsigned, class T> constexpr int read(T const&) { return 1; }
template<unsigned I, class T> constexpr int test(T const value) { return read<I>(value); }
static_assert(test<0>(0) == 1, "const parameter");
int main() {}
