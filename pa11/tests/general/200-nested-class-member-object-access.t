struct A { int x; };
struct B { A a; };
struct C { B b; int y; };

int main() {
  C c;
  c.b.a.x = 5;
  c.y = 1;
  return c.b.a.x - 5;
}
