// N3485 14.8.2.1p1, 14.8.2.5p5: a function parameter pack not at the end is
// non-deduced and deduces to empty; f(5) calls f<>(int); expect exit 0
template<class... A> int f(A..., int) { return sizeof...(A); }
int main() { return f(5); }
