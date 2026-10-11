// N3485 14.8.2.1p1, 14.8.2.5p5, 14.8.1p3: a function parameter pack not at the
// end is non-deduced and empty unless explicitly given; g(7) calls g<>(int);
// expect exit 0
template<class... T> int g(T..., int x) { return x + (int)sizeof...(T); }
int main() { return g(7) == 7 && g<int, int>(1, 2, 3) == 5 ? 0 : 1; }
