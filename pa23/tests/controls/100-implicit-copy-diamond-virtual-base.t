// N3485 12.8p15: the implicit copy constructor of a class with a virtual base
// (diamond D : A, B with virtual V) copies the virtual base subobject;
// expect exit 0
struct V { int v; V(int x) : v(x) {} };
struct A : virtual V { A(int x) : V(x) {} };
struct B : virtual V { B(int x) : V(x) {} };
struct D : A, B { D(int x) : V(x * 3), A(x), B(x) {} };
int main() { D d(5); D e(d); return e.v == 15 ? 0 : 1; }
