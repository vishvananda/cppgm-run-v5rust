struct Inner { int x; };
struct Outer { Inner field; int y; };

int main() {
  Outer o;
  o.field.x = 42;
  o.y = 1;
  return o.field.x - 42;
}
