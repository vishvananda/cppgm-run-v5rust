// N3485 5.19, Annex B (>=512 recursive constexpr calls): a 500-deep constexpr
// recursion returning a literal class must evaluate; expect exit 0
struct P { int a; double b; };
constexpr P g(int n) { return n ? P{g(n - 1).a + 1, 0.0} : P{0, 0.0}; }
static_assert(g(500).a == 500, "");
int main() { return 0; }
