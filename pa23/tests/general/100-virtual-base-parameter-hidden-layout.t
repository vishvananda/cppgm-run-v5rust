struct A { int x; };
struct B : virtual A {};
struct C : virtual A {};
struct D : B, C {};

int read_b(B & b) { return b.x; }

int main() {
  D d;
  d.x = 7;
  return read_b(d) - 7;
}
