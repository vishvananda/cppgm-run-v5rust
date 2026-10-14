// N3485 5.19p2, 9.5p5: a member of an anonymous union of a constexpr aggregate
// (g.i) is usable in a constant expression; expect exit 0
struct U { union { int i; float f; }; int y; };
constexpr U g = {1, 2};
static_assert(g.i + g.y == 3, "");
int main() { return 0; }
