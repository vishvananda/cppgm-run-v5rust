// N3485 3.4.2p2, 7.3.1p8: the inline namespaces of an associated namespace are
// associated namespaces too (full program: exit 2)
namespace N { enum E { A }; inline namespace V { int h(E) { return 2; } } }
int main() { return h(N::A); } // 2
