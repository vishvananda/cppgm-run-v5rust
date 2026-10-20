// Repeated targets move to the front of the definition's thunk list.
// Covariant and virtual-this variants must retain that order; expect exit 0
namespace Covariant {
struct C0 { virtual C0* f(); virtual int g(); };
struct C1 : virtual C0 { virtual int g(); };
struct C2 : virtual C0, virtual C1 { int data; };
struct C3 : C1 { virtual C3* f(); };
struct C4 : virtual C3 { virtual C4* f(); int data; };
struct C5 : virtual C1, virtual C2 {};
struct C6 : virtual C5 {};
C0* C0::f() { return this; }
int C0::g() { return 0; }
int C1::g() { return 1; }
C3* C3::f() { return this; }
C4* C4::f() { return this; }
void use() { C6 object; }
}
namespace Repeated {
struct C0 { virtual int f(); virtual int g(); };
struct C1 : C0 { virtual int f(); int data; };
struct C2 : C0, C1 { int data; };
struct C3 : virtual C0, C1 {};
struct C4 : virtual C1, virtual C2 { virtual int g(); int data; };
struct C5 : virtual C2, C4 { virtual int f(); int data; };
struct C6 : virtual C2, C4 { virtual int f(); int data; };
int C0::f() { return 0; }
int C0::g() { return 0; }
int C1::f() { return 1; }
int C4::g() { return 4; }
int C5::f() { return 5; }
int C6::f() { return 6; }
void use() { C6 object; }
}
int main() { return 0; }
