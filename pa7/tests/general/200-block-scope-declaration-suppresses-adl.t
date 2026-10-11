// N3485 3.4.2p3: a block-scope function declaration found by unqualified
// lookup suppresses argument-dependent lookup (full program: exit 2)
namespace N { enum E { A }; int f(E) { return 1; } }
int f(double) { return 2; }
int local() { extern int f(double); return f(N::A); }
int main() { return local(); } // 2
