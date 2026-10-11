struct A { int x; };
struct B : virtual A { };
struct C : virtual A { };
struct D : B, C { };

struct Holder {
  B & b;
  Holder(B & b) : b(b) {}
  int f() { return b.x; }
};

int main() {
  D d;
  d.x = 7;
  Holder h(d);
  return h.f() != 7;
}
