// N3485 9/11 + 7.3.1/8: struct N::X {...} may define a class previously
// declared in an element of N's inline namespace set; it completes N::I::X;
// expect exit 0
namespace N { inline namespace I { struct X; } }
struct N::X { int v; };
int main() { N::I::X x; x.v = 0; return x.v; }
