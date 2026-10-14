// N3485 5.2.5p1, 3.4.5p4: in a class member access the id-expression may be a
// qualified-id beginning with :: (p->::n1::A::g()); expect exit 0
namespace n1 { struct A { int g() { return 1; } }; }
struct C : n1::A { };
int main() { C c; C* p = &c; return p->::n1::A::g() == 1 ? 0 : 1; }
