// N3485 3.4.2p3: when ordinary unqualified lookup finds a class member,
// argument-dependent lookup is not performed (n::g must not be considered);
// expect exit 0
namespace n { struct S {}; int g(S, int) { return 2; } }
struct B { static int g(n::S, long) { return 1; } int call() { n::S s; return g(s, 0); } };
int main() { B b; return b.call() == 1 ? 0 : 1; }
