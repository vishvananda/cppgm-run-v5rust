// N3485 5.19p2, 9.5p1: active variant identity survives class results,
// by-value arguments and published copies; expect exit 0
union U { int a; int b; };
constexpr U make(int n) { U u = {1}; u.b = n; return u; }
constexpr int read(U u) { return u.b; }
constexpr U first = make(7);
constexpr U second = first;
constexpr U values[2] = {make(3), make(9)};
static_assert(read(second) == 7 && values[1].b == 9, "union copies");
int main() { return second.b == 7 && values[0].b == 3 ? 0 : 1; }
