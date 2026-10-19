// The frozen static-lvalue query retains the operand's relocation even when
// a procedural runtime reference cast would fail; expect exit 0
struct B { virtual ~B() {} };
struct D : B {};
B object;
D& cast = dynamic_cast<D&>(object);
int main() { return static_cast<void*>(&cast) != static_cast<void*>(&object); }
