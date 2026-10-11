// N3485 3.4.3.2p2, 7.3.1p8: qualified lookup of N::g includes the members of
// N's inline namespace, so N::g(1) calls g(int) (full program: exit 1)
namespace N { inline namespace I { int g(int) { return 1; } } int g(long) { return 2; } }
int main() { return N::g(1); } // 1
