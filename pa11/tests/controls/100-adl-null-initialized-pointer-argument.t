// N3485 3.4.2p2: a call argument of type ns::X* has associated namespace ns,
// regardless of the pointer variable's initializer; g is found by ADL;
// expect exit 0
namespace ns { struct X { }; int g(const void*) { return 0; } }
int main() { ns::X* p = 0; return g(p); }
