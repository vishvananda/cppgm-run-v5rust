// N3485 12.2p5: a call's class prvalue bound to an aggregate's reference
// member (H h = { mk(3) }) persists for h's lifetime and is then destroyed
// once; expect exit 0
static int dtors;
struct T { int id; T(int i) : id(i) {} ~T() { ++dtors; } };
struct H { const T& m; };
static T mk(int i) { return T(i); }
static void f() { H h = { mk(3) }; (void)h; }
int main() { f(); return dtors == 1 ? 0 : 1; }
