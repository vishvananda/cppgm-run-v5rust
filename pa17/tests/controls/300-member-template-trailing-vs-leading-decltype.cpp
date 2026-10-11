// N3485 14.5.6.1p5-p6, 8.3.5p2: an out-of-class definition with trailing ->
// decltype(g(U())) redeclares the in-class member template declared with the
// leading decltype(g(U())); the call is not ambiguous; expect exit 0
int g(int);
struct K2 { template<class U> decltype(g(U())) put(U u) const; };
template<class U> auto K2::put(U u) const -> decltype(g(U())) { return 3; }
int main() { K2 l; return l.put(0) == 3 ? 0 : 1; }
